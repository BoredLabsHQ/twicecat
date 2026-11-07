# Evidence — Task 1: send the access token with call delayed-leave updates
Commit: 568c80f
Environment: cargo test with CC=clang-22 CXX=clang++-22; in-process fake homeserver (TCP listener in the unit tests). No external services.

## Restart, cancel and send each produce exactly one POST to /delayed_events/d1/{action} with Authorization: Bearer sentinel-access-token-x
Status: PROVEN
```console
$ cargo test -p twicecat-core --lib call::own_membership::tests
running 3 tests
test call::own_membership::tests::an_unknown_delay_id_is_not_retried_on_the_older_path ... ok
test call::own_membership::tests::a_server_without_the_action_path_gets_the_older_request ... ok
test call::own_membership::tests::every_update_carries_the_access_token ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 579 filtered out; finished in 0.10s
```
`every_update_carries_the_access_token` loops the three actions, asserts one request, method POST, path `{V2}/{word}`, and the Bearer header.

## 404 M_UNRECOGNIZED on v2 is followed by POST .../delayed_events/d1 with same Authorization and body {"action":"restart"}; returns Ok
Status: PROVEN
Same run: `a_server_without_the_action_path_gets_the_older_request ... ok` (asserts 2 requests, v1 path, header, body, Ok).
Note: this test still passes with `force_auth()` removed (mutation below), so the v1 request carries auth on its own; the fallback header is not guarded by the mutation.

## 404 M_NOT_FOUND is returned as NotFound with no v1 retry
Status: PROVEN
Same run: `an_unknown_delay_id_is_not_retried_on_the_older_path ... ok` (asserts ErrorKind::NotFound and exactly 1 request).

## Only update_delayed_leave calls the update_delayed_event requests (check by alias)
Status: PROVEN
```console
$ grep -rn "update_delayed::\|update_delayed_v1::\|update_delayed_event" crates --include=*.rs
crates/core/src/call/own_membership.rs:17:use ...update_delayed_event::UpdateAction;
crates/core/src/call/own_membership.rs:18:use ...update_delayed_event::{
crates/core/src/call/own_membership.rs:84:        .send(update_delayed::Request::new(
crates/core/src/call/own_membership.rs:93:            .send(update_delayed_v1::Request::new(delay_id, action))
```
Lines 84 and 93 are inside `update_delayed_leave` (lines 77-97); no other file references them.

## Mutation: drop force_auth()
Status: PROVEN
```console
$ sed -i 's/client.request_config().force_auth()/client.request_config()/' crates/core/src/call/own_membership.rs
$ cargo test -p twicecat-core --lib call::own_membership::tests
test call::own_membership::tests::every_update_carries_the_access_token ... FAILED
test call::own_membership::tests::an_unknown_delay_id_is_not_retried_on_the_older_path ... ok
test call::own_membership::tests::a_server_without_the_action_path_gets_the_older_request ... ok
  left: None
 right: Some("Bearer sentinel-access-token-x")
test result: FAILED. 2 passed; 1 failed
$ git checkout crates/core/src/call/own_membership.rs   # restored; git status clean
```

## Ignored e2e call_signalling tests (local stack)
Status: NOT PROVEN (not run)
The local stack cannot start: host port 8080 is held by an unrelated container (`peppy-client-peppy-client-1  0.0.0.0:8080->80/tcp`). I did not stop it.

## Verdict
EVIDENCE: PROVEN
