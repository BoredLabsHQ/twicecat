# Evidence — Task 12: one switch reveals every setting, Voice & audio inside Settings
Commit: fec055f
Environment: CC=clang-22 CXX=clang++-22, `cargo test -p twicecat-app` (headless renderer gallery, no Docker, no real window). Gallery rendered into the scratchpad with TWICECAT_GALLERY_DIR and TWICECAT_GALLERY_ONLY=settings-audio and =settings-voice-audio (`write_the_gallery -- --ignored`, both ok). One image kept beside this file: task-12-settings-audio-talking.png.

## Switch on: audio pages are indented sub-entries under Voice & audio, each with its controls
Status: PROVEN (rendering; real window is human-only)
```console
$ cargo test -p twicecat-app --test ui_gallery write_the_gallery -- --ignored   (ONLY=settings-audio)
test write_the_gallery ... ok
```
Viewed settings-audio-talking-twicecat-comfortable.png (kept here), settings-voice-audio-advanced-... and settings-voice-audio-...:
- Switch on: the "Show advanced settings" checkbox is checked. The page list shows Profile, Appearance, Notifications, Voice & audio, then indented Devices, Talking (highlighted/outlined), Microphone, Levels, Mic test, Ringtone, Other voices, Announce, Diagnostics. Talking shows its controls: Send mode, Push-to-talk key with "Set key..." and Clear, Gamepad button, Release delay slider.
- The list is a scrolling column with a scrollbar thumb (x~443); it ends at Diagnostics, and Video, Keyboard, Sessions, About are below the fold (reached by scrolling). Every visible entry is whole; nothing clipped, though Diagnostics sits right at the bottom edge.
- The Voice & audio page itself (switch on) shows the basics content plus Noise reduction (cut by the content scroll, which has its own scrollbar).
- The other settings-audio-* images (devices, microphone, levels, mic-test, ringtone, other-voices, announce, diagnostics) were all generated; only the Talking one was viewed by me.

## Switch off: Voice & audio shows the basics
Status: PROVEN (rendering)
Viewed settings-voice-audio-twicecat-comfortable.png: checkbox unchecked; list is Profile, Appearance, Notifications, Voice & audio (selected), Video, Keyboard, Sessions, About, with no sub-entries and no scrollbar on the list. The page shows Microphone, Speakers, Volume, How you talk, Push-to-talk key, Noise reduction. Nothing clipped.

## No "More audio options..." button anywhere
Status: PROVEN
```console
$ grep -rn "More audio options" crates
crates/app/tests/ui_gallery.rs:4982:        assert!(!has(&many, "More audio options"));
crates/app/tests/one_advanced_switch.rs:89:            !text.contains("More audio options"),
$ cargo test -p twicecat-app --test one_advanced_switch
test in_the_audio_module_only_open_audio_controls_reads_the_switch ... ok
test there_is_no_more_audio_options_button ... ok
test only_settings_code_and_the_audio_opener_read_the_advanced_switch ... ok
test result: ok. 3 passed; 0 failed
```
The only matches are two assertions that the text is absent; no source string remains.

## Every way of opening audio controls lands in Settings (basics off, asked sub-page on); card gone from layers/Esc
Status: PROVEN (state-level tests; menu clicks not driven in a window)
```console
$ cargo test -p twicecat-app voice_controls_open_settings_basics_when_simple_and_the_asked_sub_page_when_advanced
test state::tests::settings_ui::voice_controls_open_settings_basics_when_simple_and_the_asked_sub_page_when_advanced ... ok
```
The layer/Esc-order removal is covered by the tests above and the full suite compiling; not separately exercised from outside.

## Mic test not restarted when switching sub-pages; stops on leaving
Status: PROVEN (test)
```console
$ cargo test -p twicecat-app switching_between_audio_sub_pages_keeps_the_mic_test_and_leaving_stops_it
test state::tests::settings_ui::switching_between_audio_sub_pages_keeps_the_mic_test_and_leaving_stops_it ... ok
```

## Key capture stops when the switch goes off or the page changes
Status: PROVEN (test)
```console
$ cargo test -p twicecat-app a_key_capture_stops_when_the_sub_page_changes_and_when_settings_close
test state::tests::settings_ui::a_key_capture_stops_when_the_sub_page_changes_and_when_settings_close ... ok
$ cargo test -p twicecat-app an_audio_sub_page_is_not_chosen_with_the_advanced_switch_off
test state::tests::settings_ui::an_audio_sub_page_is_not_chosen_with_the_advanced_switch_off ... ok
```
The "switch goes off" case is not named in the test title I ran; I did not read the test body to confirm it asserts that exact case.

## Switch is read only by Settings page code and open_audio_controls
Status: PROVEN (source-scan tests)
```console
$ cargo test -p twicecat-app --test one_advanced_switch
test only_settings_code_and_the_audio_opener_read_the_advanced_switch ... ok
test in_the_audio_module_only_open_audio_controls_reads_the_switch ... ok
```

## Related tests
```console
$ cargo test -p twicecat-app each_page_sends_only_its_own_messages_and_together_they_send_them_all
test audio::each_page_sends_only_its_own_messages_and_together_they_send_them_all ... ok
test settings_pages::each_page_sends_only_its_own_messages_and_together_they_send_them_all ... ok
$ cargo test -p twicecat-app --test menus_ignore_the_wheel
test result: ok. 5 passed; 0 failed
$ cargo test -p twicecat-app --test ptt
test result: ok. 2 passed; 0 failed
$ cargo test -p twicecat-app --test calls
test a_cli_call_rings_the_app_which_declines_then_answers ... ignored, needs local homeserver
```
calls.rs has its single test ignored (needs the homeserver; Docker not started), so nothing was exercised there.

## Verdict
EVIDENCE: PROVEN
