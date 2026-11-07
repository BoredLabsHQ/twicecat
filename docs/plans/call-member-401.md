# Call Membership 401 Plan (issue #46)
Status: APPROVED

## Goal
Fix GitHub issue #46, "Call participant shows as disconnected in Element while still audible". Two changes:
- **The token must not go missing.** Every request that keeps our call membership alive (restart, cancel and send of the delayed leave) carries the access token, so the delayed leave never fires while we are in the call.
- **No more silent loop.** While our `org.matrix.msc3401.call.member` is gone from the room state, we stop sending audio. If it cannot come back, the call ends with a visible error (`CallState::Left { reason: Failed(..) }`, which the app already shows as "The call ended: …"). We do not retry forever while still being heard.

The work goes on the existing branch `feat/call-member-401` (from `main` at `9b0be46`), one commit per task, trailer `Step: cm401.N`. Each commit is `fix(core)`, because the bug is in code that is already on `main` (AGENTS.md rule 5).

### Root cause (surveyed at `9b0be46`)
- **Our own code sends the delayed-leave updates without a token.** `crates/core/src/call/own_membership.rs:11` imports `update_delayed_event::unstable_v2`. That module is used for:
  - `restart_delayed_leave` (`:127`, every `DELAYED_LEAVE_RESTART` = 4 s);
  - `rejoin`'s cancel of the old delayed leave (`:170`);
  - `remove`'s send on leave (`:89`).
- **The request type itself has no token.** In ruma-client-api 0.25.0 (`src/delayed_events/update_delayed_event.rs:41-48`), `unstable_v2` is declared with `authentication: NoAccessToken` (`POST …/org.matrix.msc4140/delayed_events/{delay_id}/{action}`). matrix-sdk 0.19.1 (`src/http_client/mod.rs:116-123`) turns the client's token into `SendAccessToken::IfRequired` unless `RequestConfig::force_auth` is set. ruma-common 0.20.0 (`src/api/auth_scheme.rs:88-96`, `:329`) adds the `Authorization` header for `NoAccessToken` only on `SendAccessToken::Always`. So these requests never carry `Authorization`, whether the session was restored or freshly logged in. The "after session restore" in the issue is a coincidence: the reporter's client always starts from a restored session.
- **Newer homeservers reject them.** Synapse v1.151.0, the version in our `docker-compose.yml:14`, does not authenticate `/delayed_events/{id}/{restart,cancel,send}`: the servlets in `synapse/rest/client/delayed_events.py` at v1.151.0 do not call `get_user_by_req`. Synapse `develop` does: every one of those servlets calls `get_user_by_req(request, allow_guest=True)`. The reporter's homeserver is the newer kind, so it answers `401 M_MISSING_TOKEN`. That is why our local integration tests (`crates/core/tests/call_signalling.rs:189`, "Past the 18 s delayed leave: the restarts keep the membership") pass while production fails.
- **The timing in the log matches the delayed leave exactly.**
  - The join is at about 14:28:06.
  - The first refused restart is at 14:28:10, one 4 s tick later.
  - Our membership is gone at 14:28:24, which is `DELAYED_LEAVE` (18 s) after the join: the server sent the delayed leave because no restart was accepted.
  - The "about 5 s" in the issue is an observer estimate. The log shows 18 s, so nothing else removes the membership.
- **Why re-joining loops.** `OwnMembership::rejoin` (`:165-182`) first cancels the old delayed leave through the same unauthenticated request. That cancel gets 401, which is not `NotFound`, so `rejoin` returns `Err` before it publishes anything. `keep_membership` (`:212-224`) logs "could not join the call again; retrying" and tries again on every wake, with no limit and nothing sent to the engine.
- **Why audio keeps flowing one way.**
  - The SFU connection and our published track are independent of the membership, so the others still receive our audio. They still hold our current key (`call/e2ee.rs`), so they can decrypt it.
  - Once we are no longer a member, Element Call rotates its key on our "leave" and sends the new key only to the remaining members. We can no longer decrypt them, so their tracks are subscribed but silent.
  - This is consistent with the module docs of `call/e2ee.rs:12-18` (rotation on leave) and is not a separate bug. It goes away once the membership stays.
- **The `Received an unexpected encrypted to-device event` warnings are unrelated.** They come from matrix-sdk-crypto 0.19.1 (`src/machine/mod.rs:1555-1556`), which logs this for every decrypted Olm event of a custom type. Here those are the `io.element.call.encryption_keys` events from the other members, which our `e2ee::receive_key` handler still receives. They are not a device or key mismatch.
- **Other paths are fine.**
  - Sync, to-device sends and state sends use `AccessToken` request types, which always carry the token.
  - The SDK's own widget driver uses the authenticated `unstable_v1` (`matrix-sdk-0.19.1/src/widget/matrix.rs:244-249`).
  - `grep -rn "unstable_v2\|NoAccessToken" crates/` finds only the three uses above.

### What exists today
- `call/mod.rs:243-285` creates the `to_engine` notice channel before `keep_membership` is spawned, but does not pass it to `keep_membership`. `watch_participants` (`participants.rs:378-385`) wakes `keep_membership` through `Ours::gone` when our membership disappears after it was seen.
- **The engine:**
  - `MediaNotice` (`call/media.rs:101-109`): `Reconnecting`, `Reconnected`, `Disconnected`, `EncoderLost`.
  - `Engine::on_media_notice` (`engine/mod.rs:1986`): `Disconnected` leaves the call, stops the call audio, resets the ring gate and emits `Left { Failed("the call server connection was lost") }`.
  - `Engine::apply_mute` (`engine/mod.rs:1973-1983`) is the single place that mutes the audio session (`Outgoing`) and the published track, from `mic_muted || deafened`.
- **The app:**
  - `CallState::Reconnecting { attempt }` shows "Reconnecting (attempt n)…" (`crates/app/src/calls.rs:205`, `:676`);
  - `Left { Failed(why) }` shows the notice "The call ended: {why}" (`calls.rs:688-690`).

  No API or app change is needed.

### Decisions (defaults; see Questions)
- **D1: authenticated updates, current path first.** A single helper, `update_delayed_leave(client, delay_id, action)`, in `own_membership.rs`, sends every update:
  - First it sends `unstable_v2` with `client.request_config().force_auth()`. The client's own retry and timeout settings are kept, and the `Authorization` header is added (`RequestConfig::force_auth`, matrix-sdk-0.19.1 `src/config/request.rs:171-177`; `Client::request_config`, `src/client/mod.rs:686`).
  - If the server does not know that path (`ErrorKind::Unrecognized`), it sends `unstable_v1` instead (`AccessToken`, action in the body), as the SDK's widget driver does.
  - `restart_delayed_leave`, `rejoin` and `remove` use only this helper.
- **D2: hold the microphone while our membership is gone.** When `watch_participants` reports our membership gone:
  - `keep_membership` sends `MediaNotice::MembershipGone`;
  - the engine sets `call_held = true`, mutes through `apply_mute` (`mic_muted || deafened || call_held`), and emits `Reconnecting`;
  - when `rejoin` succeeds, `MembershipBack` clears the hold, restores the user's own mute state and emits `Connected`.

  The user's mute and deafen settings are not changed by the hold.
- **D3: give up instead of looping.** A pure, unit-tested `RejoinPolicy` decides what happens after each failed rejoin:
  - it gives up at once on an error a retry cannot fix: `MissingToken`, `UnknownToken` or `Forbidden`;
  - otherwise it gives up once `REJOIN_GIVE_UP` (30 s) has passed since the membership was found gone;
  - a delayed-leave restart refused with one of those three kinds also gives up at once, because the server will remove us within 18 s and we could not rejoin.

  Giving up sends `MediaNotice::MembershipLost`. The engine then ends the call through the same path as `Disconnected`: `call.leave()` (best-effort removal), `stop_call_audio`, and the ring gate. It emits `Left { Failed("could not stay in the call: the homeserver refused our call membership") }`. After that no audio is sent, because the SFU connection and the devices are closed. Logs name only the error kind, never a token or an ID.

### Questions for the human
- **Q1: local reproduction.** Should the local stack's Synapse image (`docker-compose.yml:14`, `v1.151.0`) move to a release that authenticates `/delayed_events/{id}/{action}`? AGENTS.md rule 3 means we ask before changing the pin. Default: **no change**. Task 1 then proves the header with an in-process fake homeserver, and Task 3 is skipped. If the answer is yes, Task 3 runs and is folded into Task 1's commit.
- **Q2: give-up window.** Is 30 s right for `REJOIN_GIVE_UP`? It is longer than one `DELAYED_LEAVE` plus one sync round trip.
- **Q3: hold the mic during the gap (D2).** Should the microphone be held during the gap, or only stopped on give-up? Default: hold, as the issue asks ("It should not keep transmitting while it looks disconnected").
- **Q4: the `unstable_v1` fallback (D1).** Should we keep the fallback for servers without the `{action}` path? Default: keep.

### Rules for every task (AGENTS.md)
- **Rule 2:** every SDK or ruma item named here was checked in the registry source of the pinned versions (matrix-sdk 0.19.1, ruma-client-api 0.25.0, ruma-common 0.20.0). The ledger cites file:line again for anything new used in the implementation.
- **Rule 3:** no pinned version changes and no new crate. The fake homeserver in the tests uses only `tokio::net::TcpListener`, which is already a dependency. The docker image changes only if Q1 is approved.
- **Rule 7:** `crates/app` is not touched. `scripts/check-app-deps.sh` and `scripts/check-core-api.sh` pass, and the public API is unchanged.
- **Rule 5:**
  - one `fix(core)` commit per task on `feat/call-member-401`, trailer `Step: cm401.N`;
  - tests and refactors are folded in with `--fixup` and `rebase --autosquash`;
  - each commit updates `PROGRESS.md` (new sections "Call membership 401 steps (issue #46)" and "Call membership 401 human checkpoint") and the ledger `docs/plans/call-member-401.progress.md`;
  - this plan is committed as `docs(plan): …` before Task 1, once approved.
- **Gate before each commit:**
  - `cargo fmt --all`;
  - `cargo clippy --workspace --all-targets -- -D warnings`;
  - `cargo test --workspace`;
  - `scripts/check-app-deps.sh`, `scripts/check-core-api.sh` and `scripts/check-commits.sh main`.
- **Rules 6 and 9:**
  - tests use the in-process fake homeserver (a sentinel token, never a real one) or the local stack with `.env.test`;
  - no token, delay ID or user ID is logged;
  - the real-homeserver check with Element is a human checkpoint.

### Task 1: Delayed-leave updates carry the access token
Language: rust
Files:
- `crates/core/src/call/own_membership.rs`:
  - `async fn update_delayed_leave(client: &Client, delay_id: String, action: UpdateAction) -> Result<(), matrix_sdk::HttpError>`. It sends `unstable_v2` with `client.request_config().force_auth()`, and falls back to `unstable_v1` on `ErrorKind::Unrecognized`;
  - `restart_delayed_leave`, `rejoin` and `remove` call it;
  - module docs explain why: ruma's `NoAccessToken`, and newer Synapse requiring a token;
  - `#[cfg(test)]` a fake homeserver on `127.0.0.1:0`. It answers `/_matrix/client/versions` and records the method, the path, the `Authorization` header and the body of every other request. It answers them with a scripted status and JSON body.
- `PROGRESS.md`; `docs/plans/call-member-401.progress.md` (new).

Depends on: none

Acceptance criteria:
- `grep -rn "update_delayed_event::unstable_v2\|update_delayed_event::unstable_v1" crates/core/src` finds both only inside `update_delayed_leave`.
- Unit test: against the fake server, a client restored with the session token `sentinel-access-token-x` calls `update_delayed_leave(.., "d1", Restart)`. The server records exactly one `POST /_matrix/client/unstable/org.matrix.msc4140/delayed_events/d1/restart` with `Authorization: Bearer sentinel-access-token-x`. The same holds for `Cancel` (`/cancel`) and `Send` (`/send`).
- Unit test: when the fake answers `404 {"errcode":"M_UNRECOGNIZED"}` to the v2 path, a second request `POST …/delayed_events/d1` follows, with the same `Authorization` header and the body `{"action":"restart"}`, and the call returns `Ok`.
- Unit test: a `404 M_NOT_FOUND` on the v2 path is returned as an error with kind `NotFound`, with no v1 retry. This is still the "server lost it, schedule a new one" case in `restart_delayed_leave`.
- Mutation, recorded in the ledger and reverted: dropping `force_auth()` fails the header test.
- The existing ignored end-to-end tests `call_signalling` (membership kept past 25 s; removed membership restored and stays; leave removes it) still pass on the local stack (`scripts/run-integration-tests.sh` or `cargo test -p twicecat-core --test call_signalling -- --ignored`).
- The gate passes. The commit is `fix(core): send the access token with call delayed-leave updates` with `Step: cm401.1`. It has no `Fixes #46`; Task 2's commit closes the issue.

Evidence: `cargo test -p twicecat-core --lib call::own_membership::tests` shows the new header and fallback tests passing. `cargo test -p twicecat-core --test call_signalling -- --ignored` passes on the local stack.

### Task 2: Hold the microphone while the membership is gone, and end the call visibly when it cannot come back
Language: rust
Files:
- `crates/core/src/call/media.rs`: `MediaNotice::{MembershipGone, MembershipBack, MembershipLost}`. Any exhaustive matches on `MediaNotice` (e.g. `call/sfus.rs`) are updated.
- `crates/core/src/call/own_membership.rs`:
  - `OwnMembership` gets `notices: mpsc::Sender<MediaNotice>` and `give_up_after: Duration` (`REJOIN_GIVE_UP` = 30 s in production);
  - `rejoin` and `restart_delayed_leave` return the failing `ErrorKind` (internal enum `Failure { Fatal, Retry }`);
  - a pure `RejoinPolicy { gone_at, give_up_after }` with `fn after_failure(&self, Failure, Instant) -> Next { Retry, GiveUp }`;
  - `keep_membership` sends `MembershipGone` once per loss and `MembershipBack` on a successful rejoin. On `GiveUp` it sends `MembershipLost` and returns, so the loop stops;
  - warnings name only the kind.
- `crates/core/src/call/mod.rs`: pass `to_engine.clone()` into `OwnMembership`.
- `crates/core/src/engine/mod.rs`:
  - field `call_held: bool`, reset on every join and end of call;
  - `apply_mute` uses `mic_muted || deafened || call_held`, through a pure `fn effective_mute(mic_muted, deafened, held) -> bool`;
  - `on_media_notice`:
    - `MembershipGone` holds and emits `Reconnecting`;
    - `MembershipBack` releases the hold and emits `Connected`, but only when no SFU reconnect is in progress;
    - `Reconnected` emits `Connected` only when not held;
    - `MembershipLost` and `Disconnected` share one `fail_call(reason)`;
  - a pure `fn failure_reason(&MediaNotice) -> Option<&'static str>`.
- `PROGRESS.md`, the ledger, and the human checkpoint entries.

Depends on: Task 1

Acceptance criteria:
- `RejoinPolicy` unit tests:
  - `MissingToken`, `UnknownToken` and `Forbidden` give up at once;
  - a network or 5xx failure retries before `give_up_after` and gives up after it;
  - the clock is injected, so no sleeping.
- `keep_membership` unit tests against the Task 1 fake server, with `give_up_after` = 1 s:
  - the fake answers every request `401 M_MISSING_TOKEN`. After `gone.notify_one()`, the notice channel receives `MembershipGone`, then `MembershipLost`, within 2 s, and the task finishes;
  - the fake answers `500`. `MembershipLost` comes only after ≥ 1 s, and the fake saw more than one rejoin attempt;
  - the fake accepts everything. `MembershipGone` is followed by `MembershipBack` and no `MembershipLost`, and the task keeps running;
  - a restart refused with `401 M_MISSING_TOKEN`, without any gone notice, gives `MembershipLost`.
- Engine unit tests:
  - `effective_mute` is true when any input is true;
  - `failure_reason(MembershipLost)` is the membership message, `failure_reason(Disconnected)` is the existing "the call server connection was lost", and every other notice gives `None`.
- Reading `on_media_notice` shows `MembershipLost` going through the same `fail_call` as `Disconnected`, which closes the SFU connection (`call.leave()`) and the audio devices (`stop_call_audio`). PROGRESS.md states that this is checked by reading the code and by the existing `Disconnected` end-to-end coverage.
- Mutations, recorded in the ledger and reverted:
  - removing the give-up branch fails the 401 test (no `MembershipLost`);
  - leaving `call_held` out of `effective_mute` fails its test.
- The existing ignored end-to-end test `a_removed_membership_is_restored_and_stays` still passes. It also now sees `CallState::Reconnecting` followed by `CallState::Connected` around the removal; the test is extended to assert this.
- `scripts/check-core-api.sh` passes with no public API change.
- Human checkpoint entries in PROGRESS.md:
  - on the reporter's kind of homeserver (newer Synapse), with Element and 3 participants, a twicecat participant stays listed in Element and in twicecat for more than 2 minutes, with audio both ways;
  - forcing a lost membership (e.g. removing the call power level for the user mid-call) shows "The call ended: could not stay in the call…" and the others stop hearing twicecat;
  - Windows and Pop!_OS.
- The gate passes. The commit is `fix(core): stop sending and end the call when its membership cannot be kept` with `Step: cm401.2` and `Fixes #46`.

Evidence: `cargo test -p twicecat-core --lib call::own_membership::tests engine::tests::effective_mute engine::tests::failure_reason` passes. `cargo test -p twicecat-core --test call_signalling -- --ignored a_removed_membership_is_restored_and_stays` shows Reconnecting, then Connected, on the local stack. The visible error in the app is a human checkpoint.

### Task 3: Reproduce on the local stack (only if Q1 is approved)
Language: rust
Files:
- `docker-compose.yml`: the Synapse image moves to the approved release that authenticates `/delayed_events/{id}/{action}`.
- `crates/core/tests/call_signalling.rs`: nothing new. The existing "kept past 25 s" test now covers the regression.
- `PROGRESS.md`, ledger.

Depends on: Task 1

Acceptance criteria:
- With the bumped image and Task 1 reverted locally (`git stash`), `cargo test -p twicecat-core --test call_signalling -- --ignored` fails on "membership still published after 25 s". This shows the stack reproduces #46.
- With Task 1 in place, the whole ignored integration suite (`scripts/run-integration-tests.sh`) passes on the bumped stack.
- This is a dev change, so it is folded into Task 1's commit (`git commit --fixup=<Task 1 sha>`, `git rebase --autosquash main`). No separate commit is left on the branch.

Evidence: `scripts/stack.sh up -d --wait && cargo test -p twicecat-core --test call_signalling -- --ignored` passes, and fails with Task 1 reverted.
