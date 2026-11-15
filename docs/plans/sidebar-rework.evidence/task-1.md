# Evidence — Task 1: room favourite / low-priority tags
Commit: eeef151
Environment: local homeserver on 127.0.0.1:8008 (already up); CC=clang-22 CXX=clang++-22; cargo test runs.

## Integration: Favourite -> (true,false); LowPriority -> (false,true); None -> (false,false); second SDK client sees m.favourite
Status: PROVEN
```console
$ cargo test -p twicecat-core --test room_tags -- --ignored
running 2 tests
test tagging_an_unknown_room_fails_without_a_panic ... ok
test tags_are_set_switched_and_removed ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.59s
```
tags_are_set_switched_and_removed passes against the real homeserver (it asserts the flag pairs and the second client's server-side tags; assertions read from the test source's helpers `flags` and `server_tags`, not re-read line by line here).

## Unknown room -> ActionFailed { kind: RoomTag } without panic
Status: PROVEN
Same run: `tagging_an_unknown_room_fails_without_a_panic ... ok`.

## Debug redaction (unit test) and no_secret_logging
Status: PROVEN
```console
$ cargo test -p twicecat-core --lib room_tag
test api::tests::set_room_tag_debug_follows_the_notification_mode ... ok
$ cargo test -p twicecat-app --test no_secret_logging
test the_scanner_flags_every_macro_and_ignores_test_code_and_comments ... ok
test no_app_code_logs_or_prints_a_secret_bearing_field ... ok
test result: ok. 2 passed; 0 failed
```

## scripts/check-core-api.sh passes
Status: PROVEN
```console
$ scripts/check-core-api.sh; echo "exit=$?"
exit=0
```

## Mutation: hard-code is_favourite: false in summarize
Status: PROVEN
```console
$ sed -i '269s/item.is_favourite()/false/' crates/core/src/room_list.rs
$ cargo test -p twicecat-core --test room_tags -- --ignored
test tagging_an_unknown_room_fails_without_a_panic ... ok
test tags_are_set_switched_and_removed ... FAILED
thread 'tags_are_set_switched_and_removed' panicked at crates/core/tests/room_tags.rs:52:10:
timed out waiting for the room list: Elapsed(())
test result: FAILED. 1 passed; 1 failed
$ git checkout crates/core/src/room_list.rs
Updated 1 path from the index
```
The test fails under the mutation (the pump never sees the favourite flag); file restored, tree clean.

## Verdict
EVIDENCE: PROVEN
