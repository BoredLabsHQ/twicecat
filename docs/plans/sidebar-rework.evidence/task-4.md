# Evidence — Task 4: section keys for favourites and low priority
Commit: 0102cb3
Environment: `export CC=clang-22 CXX=clang++-22`; cargo test, twicecat-core lib. Cargo rejects two filters, so they ran as two commands.

## Round trip with both new keys; file still says "version":1
Status: PROVEN

```console
$ cargo test -p twicecat-core --lib paths::
test paths::tests::favourites_and_low_priority_keys_round_trip_in_a_version_1_file ... ok
test paths::tests::a_layout_round_trips_and_the_file_is_private_and_versioned ... ok
...
test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 645 filtered out
```
The test (paths.rs:1621-) asserts the written text contains `"version":1` and `"lowpriority"`, and the layout reads back equal.

Mutation: `from_key` made to return None for "lowpriority" (sidebar.rs line 192):
```console
$ cargo test -p twicecat-core --lib favourites_and_low_priority
test paths::tests::favourites_and_low_priority_keys_round_trip_in_a_version_1_file ... FAILED
panicked at crates/core/src/paths.rs:1637:9: assertion `left == right` failed
test result: FAILED. 0 passed; 1 failed
```
Restored with `git checkout`; `git status --short` was empty afterwards. The test detects the break.

## A version 1 file with unknown key "pinned" drops only that key
Status: PROVEN

```console
$ cargo test -p twicecat-core --lib an_unknown_section_key_drops_only_that_key
test paths::tests::an_unknown_section_key_drops_only_that_key ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 706 filtered out
```
Its fixture is version 1 with order ["favourites","pinned","channels","lowpriority","dms"] and collapsed ["pinned","lowpriority"].

## Every existing sidebar test in paths:: passes unchanged
Status: PROVEN

```console
$ cargo test -p twicecat-core --lib paths::
test result: ok. 62 passed; 0 failed
$ cargo test -p twicecat-core --lib api::sidebar::
running 11 tests ... (incl. section_keys_round_trip_through_their_text)
test result: ok. 11 passed; 0 failed
```
All pass; none were failing or modified in the run (I did not diff the test bodies beyond the run).

## Verdict
EVIDENCE: PROVEN
