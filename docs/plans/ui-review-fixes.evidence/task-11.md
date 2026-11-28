# Evidence — Task 11: One "Set key…" for keyboard and mouse
Commit: ded8d5e
Environment: no server; library/UI surface. `CC=clang-22 CXX=clang++-22 cargo test -p twicecat-app`. Gallery written to the scratchpad with `TWICECAT_GALLERY_DIR` (not the repo), `TWICECAT_GALLERY_ONLY=audio-talking` then `settings-voice-audio`, `-- --ignored write_the_gallery`. Images were viewed. No Docker, no real window, no Wayland portal.

## One "Push-to-talk key" row (Settings page and card Talking page) shows the key label / mouse words / both joined by "or" / "None"
Status: PROVEN (key and long-key states in gallery; mouse, both, None by unit test only; no gallery scenario for them)

```console
$ cargo test -p twicecat-app --lib -- capturing_one_kind clear_clears_both mouse_buttons_read unbindable every_audio
test audio_settings::tests::talking::mouse_buttons_read_in_words_and_in_the_key_row ... ok
test audio_settings::tests::talking::clear_clears_both_kinds ... ok
test audio_settings::tests::talking::capturing_one_kind_replaces_the_other ... ok
test result: ok. 3 passed; 0 failed
```
`mouse_buttons_read_in_words_and_in_the_key_row` asserts `binding_words(None,None)=="None"`, `(None,Some("back"))=="Back side button (mouse 4)"`, `(Some("ctrl+shift+KeyT"),None)=="Ctrl+Shift+T"`, `(Some("F13"),Some("middle"))=="F13 or Middle button (wheel click)"`.

Gallery images viewed (twicecat-comfortable; the daylight/compact variants were also written):
- `audio-talking-twicecat-comfortable.png` (card, Talking page, key F13 bound): row "Push-to-talk key  F13  [Set key...] [Clear]"; below it the separate "Gamepad button" pick_list ("None"); no "Mouse button" row; no mouse note.
- `audio-talking-long-key-...png`: the row "Push-to-talk key" shows a very long key wrapped over eight lines with an ellipsis, buttons stay beside it.
- `audio-talking-capturing-...png`: row shows "Press a key..." field, "Cancel", disabled "Clear"; hint "Press the key together with the modifiers you want (Ctrl, Shift, Alt), or a mouse button other than left and right. Esc cancels."
- `settings-voice-audio-twicecat-comfortable.png` (Settings page): the same row "Push-to-talk key  F13  [Set key...] [Clear]" under "How you talk"; no Mouse button row.
- Scenarios that do NOT exist in the gallery: mouse-button-bound, both bound, nothing bound. Those states are covered only by the unit test above, not by an image.


## Set key… accepts a key or a mouse button; capturing a key clears the mouse button and vice versa; Clear clears both; refused-button note
Status: PROVEN (update tests)

```console
$ cargo test -p twicecat-app --lib -- audio_settings::tests
test audio_settings::tests::talking::capturing_one_kind_replaces_the_other ... ok
test audio_settings::tests::talking::clear_clears_both_kinds ... ok
test audio_settings::tests::talking::set_key_also_takes_a_mouse_button_but_not_a_click ... ok
test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 1836 filtered out
```
The UNBINDABLE_MOUSE note is asserted at `crates/app/src/audio_settings.rs:2853` (`capture_note == Some(UNBINDABLE_MOUSE)`) inside `set_key_also_takes_a_mouse_button_but_not_a_click`, which passed. A real mouse button press in a window is human-only (NOT PROVEN here).

## Separate Mouse button pick_list gone; mouse_note under the key row only while a mouse button is bound; Mouse/MouseChoice removed
Status: PROVEN

```console
$ grep -rn "MouseChoice\|AudioMessage::Mouse\b" crates/app/src
(no matches)
$ sed -n 587,589p crates/app/src/screens/audio_settings.rs
    if bindings.mouse_button.is_some() {
        rows = rows.push(note(mouse_note(app.ptt.keyboard())));
    }
```
The grep for the removed names is empty (the Talking-page image has no Mouse button row), and mouse_note is pushed only under `mouse_button.is_some()`. No gallery image has a mouse button bound, so the note being visible is not shown by an image.

## Every AudioMessage reachable test; gamepad picker unchanged
Status: PROVEN

```console
test audio_settings::tests::pages::every_message_belongs_to_a_page_and_the_pages_cover_them_all ... ok
test audio_settings::tests::talking::the_gamepad_menu_offers_every_button_and_keeps_an_unknown_one ... ok
```
Gamepad picker is the "Gamepad button" pick_list showing "None" in both images.

## tests/ptt.rs passes unchanged
Status: PROVEN

```console
$ cargo test -p twicecat-app --test ptt
running 2 tests
test without_a_delay_a_release_stops_at_once_and_voice_activity_ignores_presses ... ok
test a_release_keeps_transmitting_for_the_delay_then_stops ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
`git diff 90d8212..ded8d5e --stat` lists no change to `tests/ptt.rs`.

## Verdict
EVIDENCE: PROVEN
