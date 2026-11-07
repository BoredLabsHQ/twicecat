Task 1: done (cm401.1)

## Task 1
- Added `update_delayed_leave` in `crates/core/src/call/own_membership.rs`: `unstable_v2` with `client.request_config().force_auth()` (matrix-sdk 0.19.1 `src/config/request.rs:171`, `SendRequest::with_request_config` in `src/client/futures.rs:67`), falling back to `unstable_v1` on `ErrorKind::Unrecognized`. `restart_delayed_leave`, `rejoin` and `remove` use only it.
- Tests (fake homeserver on 127.0.0.1:0): token on restart/cancel/send; v1 fallback with body `{"action":"restart"}`; `M_NOT_FOUND` returned with no retry.
- Mutation: dropping `force_auth()` made `every_update_carries_the_access_token` fail (the other two passed). Reverted.
- Build note: this machine needs `CC=clang-22 CXX=clang++-22` (default clang is 18).
- End-to-end `call_signalling`: the three ignored tests pass on the local stack (run after the Task 2 fixup).
Task 1: complete (074dc65..568c80f, gate ok, security PASS, review PASS, evidence PROVEN)
  evidence: docs/plans/call-member-401.evidence/task-1.md
  notes: ignored e2e call_signalling tests pass on the local stack (run later); plan's literal grep for update_delayed_event::unstable_v1/v2 cannot match because imports are aliased, intent met; fallback test does not guard force_auth (v1 authenticates by default); reviewer/auditor Low items: v1 fallback only on M_UNRECOGNIZED
Task 2: started

## Task 2
- `MediaNotice::{MembershipGone, MembershipBack, MembershipLost}` (`call/media.rs`); `OwnMembership` gets `notices` and `give_up_after`; `RejoinPolicy`, `Failure`, `Next`, `failure_of` in `own_membership.rs`; `call_held`, `effective_mute`, `failure_reason`, `fail_call` in `engine/mod.rs` (`engine/calls.rs` is not a module of the crate, left alone). `rejoin`/`restart_delayed_leave` now use `try_schedule_delayed_leave` and `send_membership_raw` to keep the server's error kind.
- API checked: `ErrorKind::{MissingToken, UnknownToken(UnknownTokenErrorData), Forbidden}` (ruma-common 0.20.0 `src/api/error/kind.rs:180,332`), `RequestConfig::disable_retry` (matrix-sdk 0.19.1 `src/config/request.rs:117`), used in tests so the SDK's own 5xx retries do not hide our policy.
- Red first: tests written before the types existed failed to compile (`RejoinPolicy`, `Failure`, `Next`, `notices` missing).
- Mutations: `Fatal => Retry` plus no window made 5 tests fail; dropping `held` from `effective_mute` failed `effective_mute_is_true_when_any_input_is`. Both reverted.
- e2e `a_removed_membership_is_restored_and_stays` extended (Reconnecting then Connected); passes on the local stack.
Task 2: complete (7a013da..5c42ab4, gate ok, security PASS, review PASS, evidence PROVEN)
  evidence: docs/plans/call-member-401.evidence/task-2.md
  notes: security Mediums resolved in the fixup: the give-up window is enforced per attempt (`timeout_at`, minimum 3 s; restart limited to 10 s) and camera, screen share and its sound are held with the microphone. Low: notices sent during join are handled only after join returns; set_muted failure ignored. review warnings: engine notice send errors dropped; dead crates/core/src/engine/calls.rs.
Final evidence: EVIDENCE: PROVEN (everything observable without the local stack; e2e call_signalling later run and passing)
Final review: VERDICT: PASS

## Task 2 fixup (security Mediums)
- Give-up window: `within(RejoinPolicy::attempt_deadline(now), own.rejoin())` (`tokio::time::timeout_at`; deadline is the end of the window, at least `MIN_ATTEMPT` 3 s ahead); a timeout is `Failure::Retry`, so the policy gives up past the window. Delayed-leave restarts get `RESTART_LIMIT` 10 s. Test `the_window_holds_with_the_sdk_retrying_server_errors` uses the SDK default config (retries on) against an always-500 fake and expects `MembershipLost` within 6 s of a 1 s window. Mutation: rejoin without the deadline made it fail (8 s timeout); restored. The fake answers setup well and refuses only once the keeper runs (otherwise login retried for minutes).
- Hold: `SfuConnection::set_held` (`call/media.rs`) mutes our camera, screen-share video and screen-share sound tracks (livekit 0.9.5 `LocalVideoTrack::mute/unmute`, `LocalAudioTrack::mute/unmute`) on `MembershipGone` and restores them on `MembershipBack`; a track started during the hold is held at once. Pure `hold_step(held, muted, ours)` restores only what the hold muted, so a track the user turned off stays off. Engine calls it through `CallSession::set_held`. No public API change. Mutation: restoring regardless of `ours` fails `a_hold_mutes_what_is_sending_and_restores_only_that`; restored.
- Gate: fmt, clippy `-D warnings`, `cargo test --workspace`, `check-core-api.sh`, `check-app-deps.sh` ok; `call_signalling -- --ignored`: 3 passed.
- Follow-up: The periodic membership refresh is bounded too (`OwnMembership::refresh`, `within(RESTART_LIMIT)`; a timeout or error is a logged warning); test `a_refresh_the_sdk_keeps_retrying_is_given_up_at_its_limit` (SDK retries on, always-500 fake, 1 s limit), mutation (limit made huge) fails it. The wall-clock bound in `the_window_holds_with_the_sdk_retrying_server_errors` is now 10 s (about 4 s expected). Low findings left: a schedule step cut off by the timeout can leave an unknown delayed leave on the server; a track started while held is announced unmuted for a moment.
