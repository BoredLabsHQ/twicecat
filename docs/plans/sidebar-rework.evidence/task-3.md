# Evidence — Task 3: room sort and unread-first in the settings
Commit: 7be68f1
Environment: CC=clang-22 CXX=clang++-22, `cargo test -p twicecat-core --lib paths::` (60 passed). No server or UI surface: the consumer is the settings.json read/write API, exercised through the crate's tests.

## Round trip of both fields (room_sort, unread_first) through settings.json
Status: PROVEN

```console
$ cargo test -p twicecat-core --lib paths::
...
test paths::tests::the_room_sort_round_trips_and_defaults_per_field ... ok
test paths::tests::ui_settings_stays_copy ... ok
test paths::tests::an_oversized_settings_file_gives_defaults ... ok
test result: ok. 60 passed; 0 failed; 0 ignored; 0 measured; 645 filtered out
```
The test writes all four room_sort x unread_first combinations with write_settings and reads them back equal.
Mutation: unread_first made always false in settings_from_map (paths.rs line 69):
```console
$ cargo test -p twicecat-core --lib paths::tests::the_room_sort
test paths::tests::the_room_sort_round_trips_and_defaults_per_field ... FAILED
panicked at crates/core/src/paths.rs:1131:17: assertion `left == right` failed
  left: ... room_sort: Activity, unread_first: false }
 right: ... room_sort: Activity, unread_first: true }
test result: FAILED. 0 passed; 1 failed
```
The test catches the broken field. The file was restored with `git checkout`, and `git status` was clean.

## Missing fields give Activity/false; bad values give the field default and keep the rest
Status: PROVEN

The same test (lines 1133-1149) asserts that `{"tray_badge":false}` gives Activity and false. It also asserts that `{"room_sort":"zigzag","unread_first":3,"theme":"Dark"}` gives Activity, false and theme Dark, and that `{"room_sort":"Name","unread_first":"yes"}` gives Name and false. It passed in the run above.

## an_oversized_settings_file_gives_defaults and the serializer guard test pass unchanged
Status: PROVEN

`an_oversized_settings_file_gives_defaults ... ok` is in the run above. `git diff 3a0930c..7be68f1` does not touch that test; the only match is a PROGRESS.md line.
```console
$ cargo test -p twicecat-core --lib to_bytes_is_the_only_serializer
test session::tests::to_bytes_is_the_only_serializer_and_its_errors_carry_no_secret ... ok
```

## UiSettings is still Copy (compile-time assertion)
Status: PROVEN

`paths::tests::ui_settings_stays_copy ... ok`. It uses `fn is_copy<T: Copy>()` with `is_copy::<UiSettings>()`.
