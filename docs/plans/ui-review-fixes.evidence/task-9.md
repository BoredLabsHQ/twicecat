# Evidence — Task 9: encryption shown only when it is a problem
Commit: 83447c7
Environment: local cargo, CC=clang-22 CXX=clang++-22. No Docker, no homeserver. Gallery PNGs rendered with `TWICECAT_GALLERY_DIR=<scratchpad>/gal TWICECAT_GALLERY_ONLY=<main-|room-settings|call> cargo test -p twicecat-app --test ui_gallery write_the_gallery -- --ignored` (3 runs, each "1 passed"). Viewed: main-room-states-twicecat-comfortable.png, room-settings-twicecat-comfortable.png (kept beside this file), main-call-unencrypted-twicecat-comfortable.png, main-call-connected-twicecat-comfortable.png (scratchpad only).

## Room rows, rail rows and tooltips show nothing for Encrypted and Unknown; NotEncrypted gets an open-lock mark with "Not encrypted"
Status: PROVEN (rows by image and test; tooltip text by test only)
```console
$ cargo test -p twicecat-app --lib
test screens::room_list_pane::tests::only_a_room_known_to_be_unencrypted_has_a_mark ... ok
test screens::room_list_pane::tests::a_rail_row_marks_invites_voice_rooms_and_encryption ... ok
test result: ok. 1884 passed; 0 failed; 0 ignored
```
Gallery (main-room-states): Lounge (Encrypted) and Stand-up (Unknown) rows show no lock; only "Open house" (NotEncrypted) carries a small open-lock glyph beside its time. Tooltip/rail text is asserted by the unit tests, not seen in an image.

## Room header drops the "Encrypted" tag; NotEncrypted room shows a Warning tag "Not encrypted"; Info tab states encryption in words for every state and agrees with the header
Status: PROVEN
```console
$ cargo test -p twicecat-app --lib
test screens::main_view::tests::the_context_line_and_the_header_flag_only_unencrypted_rooms ... ok
$ cargo test -p twicecat-app --test ui_gallery
test dialogs::the_header_flag_and_the_info_tab_agree_on_a_rooms_encryption ... ok
test result: ok. 193 passed; 0 failed; 1 ignored
```
main-room: header "Lounge" has no tag. room-settings (task-9-room-settings.png): header shows an orange "Not encrypted" tag under "Lounge", the Info tab says "Not encrypted: the server can read messages in this room.", and the Lounge row has the open lock. Info-tab wording for the Encrypted and Unknown states was not viewed; only the NotEncrypted/encrypted agreement is asserted by the gallery test.

## Call dock: no lock for encrypted/not-known calls; open lock + visible words "Not encrypted" for an unencrypted call
Status: PROVEN
```console
$ cargo test -p twicecat-app --lib
test screens::call::tests::the_dock_flags_only_an_unencrypted_call_and_says_so_in_words ... ok
```
main-call-unencrypted: dock reads "Voice connected" with an orange boxed open-lock "Not encrypted" at its right. main-call-connected (encrypted): no lock and no tag. Observation, not a failure: the encrypted dock's phase line still reads "Voice connected, encrypted" (words, no lock); the criterion speaks of locks only, but the caller may want to confirm that wording is intended.

## tests/calls.rs unchanged
Status: PROVEN
```console
$ git diff --stat 13a8f17..83447c7 -- crates/app/tests/calls.rs
(empty)
$ cargo test -p twicecat-app --test calls
test a_cli_call_rings_the_app_which_declines_then_answers ... ignored, needs local homeserver: scripts/stack.sh up -d --wait
test result: ok. 0 passed; 0 failed; 1 ignored
```
File untouched; its only test needs a homeserver, so it was not run.

## Not checked
Real window rendering and homeserver behaviour are human-only.

EVIDENCE: PROVEN
