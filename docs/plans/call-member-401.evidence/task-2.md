# Evidence — Task 2: Hold the microphone while the membership is gone, and end the call visibly when it cannot come back
Commit: 5c42ab4
Environment: no server started; unit tests only (the keep_membership tests start the in-process fake homeserver from Task 1). `CC=clang-22 CXX=clang++-22 cargo test -p twicecat-core --lib ...`. Outputs trimmed to the `test` and `result` lines.

## RejoinPolicy unit tests
Status: PROVEN
```console
$ cargo test -p twicecat-core --lib call::own_membership::tests
test call::own_membership::tests::errors_a_retry_cannot_fix_give_up_at_once ... ok
test call::own_membership::tests::only_token_and_permission_errors_are_fatal ... ok
test call::own_membership::tests::other_failures_retry_until_the_window_has_passed ... ok
test call::own_membership::tests::an_unknown_delay_id_is_not_retried_on_the_older_path ... ok
test call::own_membership::tests::a_server_without_the_action_path_gets_the_older_request ... ok
test call::own_membership::tests::a_refused_token_ends_the_rejoin_at_once ... ok
test call::own_membership::tests::every_update_carries_the_access_token ... ok
test call::own_membership::tests::server_errors_are_retried_until_the_window_has_passed ... ok
test call::own_membership::tests::a_membership_that_comes_back_is_reported_and_kept ... ok
test call::own_membership::tests::a_refused_restart_ends_the_call ... ok
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 581 filtered out; finished in 4.06s
```
The pure-policy tests (fatal gives up at once, retry gives up only after the injected window) pass.

## keep_membership against the fake homeserver (401, 500, accept-all, refused restart)
Status: PROVEN
Same run as above: a_refused_token_ends_the_rejoin_at_once (401 -> Gone, Lost), server_errors_are_retried_until_the_window_has_passed (500 -> Lost after >=1s, several attempts), a_membership_that_comes_back_is_reported_and_kept (Gone, Back, no Lost), a_refused_restart_ends_the_call (Lost) all ok.

## Engine tests effective_mute and failure_reason
Status: PROVEN
```console
$ cargo test -p twicecat-core --lib engine::tests::effective_mute
test engine::tests::effective_mute_is_true_when_any_input_is ... ok
$ cargo test -p twicecat-core --lib engine::tests::only_losing
test engine::tests::only_losing_the_call_gives_a_failure_reason ... ok
```
(Filter `engine::tests::failure_reason` matches 0 tests because the test is named only_losing_the_call_gives_a_failure_reason; it was run by that name.)

## MembershipLost and Disconnected share fail_call (code reading)
Status: PROVEN
crates/core/src/engine/mod.rs:
```
2027:            MediaNotice::Disconnected | MediaNotice::MembershipLost => {
2028:                if let Some(reason) = failure_reason(&notice) {
2029:                    self.fail_call(room_id, reason).await;
2049:    async fn fail_call(&mut self, room_id: RoomId, reason: &'static str) {
2050:        self.call_held = false;
2056:            call.leave().await;
2059:        self.stop_call_audio().await;
```
One match arm routes both notices to fail_call, which calls call.leave() (SFU) and stop_call_audio(), then emits Left with CallEndReason::Failed. Also read: MembershipGone (2012) sets call_held and calls apply_mute; apply_mute (1980) uses effective_mute(mic_muted, deafened, call_held); call_held reset at 1870, 2050, 2132.

## Mutations
Status: PROVEN
```console
$ # remove the give-up branch (own_membership.rs:383-384: send MembershipLost + return, replaced by a retry)
$ cargo test -p twicecat-core --lib call::own_membership::tests
test call::own_membership::tests::a_refused_token_ends_the_rejoin_at_once ... FAILED
test call::own_membership::tests::server_errors_are_retried_until_the_window_has_passed ... FAILED
test result: FAILED. 8 passed; 2 failed
$ git checkout crates/core/src/call/own_membership.rs

$ # drop `|| held` from effective_mute
$ cargo test -p twicecat-core --lib engine::tests::effective_mute
test engine::tests::effective_mute_is_true_when_any_input_is ... FAILED
assertion failed: effective_mute(false, false, true)
$ git checkout crates/core/src/engine/mod.rs
```
Both mutations fail the expected tests; both reverted (git status clean). Note: a first mutation attempt matched two sites and did not apply; it changed nothing.

## scripts/check-core-api.sh
Status: PROVEN
```console
$ bash scripts/check-core-api.sh; echo "exit=$?"
exit=0
```

## Ignored e2e a_removed_membership_is_restored_and_stays
Status: NOT PROVEN (not run: local stack cannot start, host port 8080 is held by an unrelated container, which was left alone)

## Verdict
EVIDENCE: PROVEN (all outside-observable criteria; e2e not run)
