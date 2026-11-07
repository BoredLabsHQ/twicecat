# Evidence — whole feature: call-member-401 (issue #46)
Commit: 0bf466b
Environment: CC=clang-22 CXX=clang++-22; unit/fake-homeserver tests in-process (no local stack). Output below is trimmed to the decisive lines; full runs were observed.

## Task 1: delayed-leave updates carry the token (incl. fallback)
Status: PROVEN
```console
$ cargo test -p twicecat-core --lib call::own_membership::tests
running 10 tests
test call::own_membership::tests::every_update_carries_the_access_token ... ok
test call::own_membership::tests::a_server_without_the_action_path_gets_the_older_request ... ok
test call::own_membership::tests::an_unknown_delay_id_is_not_retried_on_the_older_path ... ok
test call::own_membership::tests::a_refused_token_ends_the_rejoin_at_once ... ok
test call::own_membership::tests::a_refused_restart_ends_the_call ... ok
test call::own_membership::tests::a_membership_that_comes_back_is_reported_and_kept ... ok
(+4 more, all ok)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 581 filtered out
```
The fake homeserver sees Authorization on every update; the older-path fallback tests pass.

## Task 2 / Goal: 401 M_MISSING_TOKEN gives Gone then Lost; engine maps MembershipLost like Disconnected
Status: PROVEN (fake homeserver + engine unit tests; the planned name `engine::tests::failure_reason` matches 0 tests, the real test is `only_losing_the_call_gives_a_failure_reason`)
```console
$ cargo test -p twicecat-core --lib -- membership_lost fail_call failure   (trimmed)
test engine::tests::only_losing_the_call_gives_a_failure_reason ... ok
$ cargo test -p twicecat-core --lib engine::tests::effective_mute
test engine::tests::effective_mute_is_true_when_any_input_is ... ok
```
Test source (own_membership.rs): `a_refused_token_ends_the_rejoin_at_once` and `a_refused_restart_ends_the_call` use keeper(Some(MISSING_TOKEN), ..) and assert next notice == MembershipGone then MembershipLost (lines 751-752, 765-766), both passing in the 10-test run above. engine/mod.rs:2027 handles `Disconnected | MembershipLost` through the same `fail_call`, with MembershipLost giving reason "could not stay in the call: the homeserver refused our call membership".
The visible error in the app is a human checkpoint (not exercised).

## Accepting fake: Gone then Back
Status: PROVEN
```console
test call::own_membership::tests::a_membership_that_comes_back_is_reported_and_kept ... ok
```
Asserts MembershipGone (line 785) then back; engine MembershipBack unmutes (held=false).

## Workspace tests and scripts
Status: PROVEN
```console
$ cargo test --workspace
60 "test result" lines, all "ok"; no FAILED. Core lib: 591 tests; app lib: 1339 passed; ignored tests are the e2e needing the stack.
$ scripts/check-core-api.sh; echo exit=$?
exit=0
$ scripts/check-app-deps.sh; echo exit=$?
exit=0
```

## Task 3: e2e `cargo test -p twicecat-core --test call_signalling -- --ignored` on the local stack
Status: NOT PROVEN (not run)
```console
$ ss -ltnp | grep 8080     (checked at start and again at end)
LISTEN 0      4096         0.0.0.0:8080       0.0.0.0:*
LISTEN 0      4096            [::]:8080          [::]:*
```
Host port 8080 is still held by the unrelated container peppy-client (not touched), so `scripts/stack.sh up -d --wait` cannot start. The e2e claims (Reconnecting then Connected on a removed membership; fails with Task 1 reverted) are not demonstrated.

## Verdict
EVIDENCE: PROVEN (outside-observable criteria via fake homeserver; e2e call_signalling not run)
