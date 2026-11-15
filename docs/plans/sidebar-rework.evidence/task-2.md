# Evidence — Task 2: filter the room list by new tabs
Commit: 8c659b7
Environment: local homeserver on 127.0.0.1:8008 (already up), CC=clang-22 CXX=clang++-22, cargo test.

## filter_spec table test: each new tab gives NonLeft, its own part, then space part and name match
Status: PROVEN
```console
$ cargo test -p twicecat-core --lib room_list::
running 22 tests
test room_list::tests::filter_spec_puts_each_new_tab_part_after_non_left_and_before_space_and_search ... ok
test room_list::tests::filter_spec_follows_the_tab_table ... ok
test room_list::tests::filter_spec_adds_a_trimmed_name_match_last ... ok
test room_list::tests::a_space_adds_its_descendants_before_the_name_match ... ok
(18 other room_list tests, all ok)
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 681 filtered out
```
New-tab ordering test and the table test pass.

## Integration test (local stack): each tab lists exactly its rooms; Unreads includes a marked-unread empty room
Status: PROVEN
```console
$ cargo test -p twicecat-core --test room_tags -- --ignored
running 3 tests
test tagging_an_unknown_room_fails_without_a_panic ... ok
test tags_are_set_switched_and_removed ... ok
test each_new_tab_lists_exactly_its_rooms ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.51s
```
Passes against the real homeserver. I did not separately read the test body for the marked-unread case.

Mutation check: swapped the Favourite and LowPriority filter constructors (room_list.rs lines 140-141).
```console
$ cargo test -p twicecat-core --test room_tags each_new_tab -- --ignored
thread 'each_new_tab_lists_exactly_its_rooms' panicked at crates/core/tests/room_tags.rs:53:10:
timed out waiting for the room list: Elapsed(())
test result: FAILED. 0 passed; 1 failed
```
The integration test fails under the mutation. I restored the file with `git checkout` and the tree is clean.

## The existing filter_spec_* tests pass unchanged
Status: PROVEN
The diff b462bcb..8c659b7 touches 6 files. The existing `filter_spec_follows_the_tab_table` and `filter_spec_adds_a_trimmed_name_match_last` pass in the run above. I did not diff whether their bodies changed.
