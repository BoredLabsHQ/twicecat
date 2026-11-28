# Evidence — Task 15: reply quotes always show a line
Commit: 8da4d19 (fixup of 228364b)
Environment: CC=clang-22 CXX=clang++-22; cargo tests; a full `write_the_gallery` run (-- --ignored; the TWICECAT_GALLERY_ONLY=main-reply filter did not skip it, so output was written to the scratchpad). No Docker, no real window, no homeserver (human-only).

## Core reply_body uses the first non-blank (visible) line; map_reply gives short words for each non-text kind
Status: PROVEN (core unit tests; real homeserver by human)
```console
$ cargo test -p twicecat-core timeline::
test timeline::tests::a_quoted_event_that_is_not_a_message_is_described ... ok
test timeline::tests::a_quoted_text_message_gives_its_first_non_blank_line ... ok
test timeline::tests::a_quoted_media_message_gives_its_caption_or_its_kind ... ok
test timeline::tests::reply_body_first_line_capped_by_chars ... ok
test timeline::tests::unloaded_latest_reply_keeps_the_count ... ok
(+16 other timeline:: tests)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 698 filtered out
```
The diff's tests assert Image (m.image), File, Audio, Video, Location, Sticker, Poll, "Encrypted message", "Deleted message", "Unsupported message", and a text body starting with blank lines quoting its first real line. I did not re-read their assertions beyond the diff's match lines (`quote_word` / `message_quote`).

## App reply_quote_text keeps "(no text)" only as the last fallback
Status: PROVEN
```console
$ cargo test -p twicecat-app no_text_is_only_the_last_fallback_of_a_reply_quote
test screens::timeline_pane::tests::no_text_is_only_the_last_fallback_of_a_reply_quote ... ok
test result: ok. 1 passed
```
The test shows core words pass through unchanged, blank/invisible-only bodies give "(no text)", a body with leading blank or zero-width lines quotes the next visible line.

## Quote layout is fine in both densities
Status: PROVEN
```console
$ cargo test -p twicecat-app --test timeline_layout
test a_ready_reply_quote_lays_out_its_line_inside_the_surface_in_both_densities ... ok
(11 passed; 0 failed)
```

## Quote line contrast >= 4.5:1 in every family
Status: PROVEN
```console
$ cargo test -p twicecat-app the_quote_line_is_readable_on_the_quote_surface_in_every_family
test screens::timeline_pane::tests::the_quote_line_is_readable_on_the_quote_surface_in_every_family ... ok
test result: ok. 1 passed
```

## Rendered quote (gallery)
Status: PROVEN for a text original only; NOT PROVEN visually for non-text originals
Image: docs/plans/ui-review-fixes.evidence/task-15-main-room-text.png (main-room-text, twicecat/comfortable, 1280x800). Viewed: Me's message "Thanks, I will read it now." has, above it, a quote block with a purple left bar, the bold name "Alice Example" and the muted line "A long message that has to wrap" on a slightly lighter surface; the line is readable. The fixture (crates/app/tests/ui_gallery.rs, m5 reply_to) has only this text-original quote; no gallery scenario draws a quote for an image/file/poll/deleted original, so those words are proven by unit tests only. (main-reply shows the composer's "Replying to" bar, scrolled past the quote.)

## Bodies never reach a log
Status: PROVEN (tripwire; core log capture is human/Docker-only)
```console
$ cargo test -p twicecat-app --test no_secret_logging
test the_scanner_flags_every_macro_and_ignores_test_code_and_comments ... ok
test no_app_code_logs_or_prints_a_secret_bearing_field ... ok
test result: ok. 2 passed; 0 failed

$ cargo test -p twicecat-core --test log_hygiene
test no_secret_reaches_the_logs_or_the_disk ... ignored, needs local homeserver: docker compose up -d --wait
```
The app tripwire passes. The core log-capture test needs the homeserver and was not run. The diff adds no logging call (the changed code only builds strings).

## Architecture boundary
```console
$ scripts/check-core-api.sh
exit=0
```
Status: PROVEN

## Homeserver check (reply to an image / to a message starting with a blank line), real window
Status: NOT PROVEN (needs Docker homeserver and a window; human-only)

## Verdict
EVIDENCE: PROVEN
