# Evidence — Task 10a: remember the spaces rail choice in the UI settings
Commit: d3b828d
Environment: local, no Docker. CC=clang-22 CXX=clang++-22. Named tests via cargo; consumer program in the scratchpad (`.../scratchpad/consumer`, path dependency on twicecat-core, repo Cargo.lock copied), calling the public `DataPaths::at` / `read_settings` / `write_settings`.

## Named tests (paths:: and the UiSettings doctest)
Status: PROVEN
```console
$ cargo test -p twicecat-core paths:: | grep -E "spaces_rail|settings_ignore|test result: .* 70"
test paths::tests::settings_ignore_unknown_fields_and_fill_missing_ones ... ok
test paths::tests::spaces_rail_reads_old_files_and_odd_values_as_expanded ... ok
test paths::tests::spaces_rail_round_trips ... ok
test result: ok. 70 passed; 0 failed; 0 ignored; 0 measured; 646 filtered out; finished in 0.91s
$ cargo test -p twicecat-core --doc UiSettings
test crates/core/src/api/rooms.rs - api::rooms::UiSettings (line 541) ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.00s
```
The three named tests and the doctest (which asserts the Expanded default) pass.

## Old-format file loads with Expanded, other settings unchanged
Status: PROVEN
```console
$ cargo run (scratch consumer)
old file: {"theme":"Dark","tray_badge":false,"density":"Compact"}
read old: UiSettings { theme: Dark, desktop_notifications: true, show_message_content: true, tray_badge: false, theme_name: None, density: Compact, room_list: Expanded, spaces_rail: Expanded, text_scale_percent: 100, ... }
```
A file with no spaces_rail key gives spaces_rail: Expanded; theme, tray_badge and density keep their file values and the rest are defaults. Default is also Expanded (see the `defaults:` line, output trimmed).

## "Sideways" / 3 / null read as Expanded, file not renamed .bad
Status: PROVEN
```console
{"spaces_rail":"Sideways"} -> Expanded; settings.json.bad exists: false
{"spaces_rail":3} -> Expanded; settings.json.bad exists: false
{"spaces_rail":null} -> Expanded; settings.json.bad exists: false
```

## Persisted as "Expanded"/"Collapsed"; round trip
Status: PROVEN
```console
written key spaces_rail = "Collapsed"
read back: Collapsed
```
The writer emits the string "Collapsed" and reading it back gives Collapsed. The Expanded half of the round trip is covered by `spaces_rail_round_trips`, which loops over both values and passed above.

## UiSettings still Copy
Status: PROVEN
```console
UiSettings: Copy ok
```
The consumer compiled a `T: Copy` bound check against the public `UiSettings`.

## check-core-api.sh passes
Status: PROVEN
```console
$ scripts/check-core-api.sh; echo "exit=$?"
exit=0
```
The script printed nothing and exited 0.
