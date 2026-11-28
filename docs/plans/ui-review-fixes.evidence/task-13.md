# Evidence — Task 13: clearer message groups with the sender's avatar and name
Commit: 959f50a (fixup 8d24e3f; both fix a11ef16)
Environment: headless iced renderer gallery and cargo tests, CC=clang-22 CXX=clang++-22. No Docker, no real window.
Gallery: `TWICECAT_GALLERY_DIR=<scratch> TWICECAT_GALLERY_ONLY=main-room cargo test -p twicecat-app --test ui_gallery -- --ignored` -> `test write_the_gallery ... ok` (24 PNGs).
Viewed: main-room-twicecat-comfortable.png (copied beside this file: task-13-main-room-comfortable.png) and main-room-daylight-compact.png.

## Header: sender name BODY_LARGE SemiBold, readable per-sender colour, muted timestamp after it
Status: PROVEN (visual, plus test for contrast on all eight surfaces)
```console
$ cargo test -p twicecat-app the_header_name_is_readable_on_every_surface_in_every_theme
test screens::timeline_pane::tests::the_header_name_is_readable_on_every_surface_in_every_theme ... ok
```
Comfortable dark image: "Cy Example" pink, "Alice Example" blue-violet, "Me" blue, each larger and bolder than the body text, with a small grey time ("02:07", "02:10", "02:49", "02:55") right after. Daylight compact: same, darker colours on the light surface (Cy magenta, Alice/Me blue). Per-sender colours differ and are legible. Font weight is judged by eye only (names look bold); the test covers contrast, not weight.

## More space above a group header (>= L comfortable, >= M compact); continuation rows keep their pad
Status: PROVEN
```console
$ cargo test -p twicecat-app a_group_header_has_room_above_it_and_a_continuation_keeps_its_pad
test timeline::tests::a_group_header_has_room_above_it_and_a_continuation_keeps_its_pad ... ok
```
In both images the group boundaries (Cy / Alice / Me / Me) have visibly larger gaps than the gaps between rows inside a group (e.g. "clip.mp4" -> "This one did not go through." is tight, no new header). The group gap is visibly larger in comfortable than in compact.

## Message hover background contrast >= 1.15:1 against the window background in every family
Status: PROVEN (test only; no scenario shows a hovered row)
```console
$ cargo test -p twicecat-app the_hover_background_shows_against_the_window_in_every_family_and_mode
test theme::tests::the_hover_background_shows_against_the_window_in_every_family_and_mode ... ok
$ cargo test -p twicecat-app an_overlay_too_close_to_the_window_has_its_own_reason
test theme::tests::an_overlay_too_close_to_the_window_has_its_own_reason ... ok
```
The second is the theme refusal test (a theme whose overlay is too close to the window is refused with its own reason).

## 36 px sender avatar on header rows only, not a Tab stop; header row at least as tall as the avatar
Status: PROVEN
```console
$ cargo test -p twicecat-app header_rows_have_a_sender_avatar_and_continuations_have_none
test screens::timeline_pane::tests::header_rows_have_a_sender_avatar_and_continuations_have_none ... ok
$ cargo test -p twicecat-app the_header_avatar_is_a_plain_container_without_tooltip_or_press
test screens::timeline_pane::tests::the_header_avatar_is_a_plain_container_without_tooltip_or_press ... ok
$ cargo test -p twicecat-app a_header_row_is_at_least_as_tall_as_the_avatar
test timeline::tests::a_header_row_is_at_least_as_tall_as_the_avatar ... ok
```
Images: every group header has a round avatar at the left (initials "CE", "AE", "M" on coloured discs; no pictures in the fixture); the continuation rows ("clip.mp4", "This one did not go through.") have none and are indented to the text column. Nothing clipped or overlapping; header text aligns with the avatar. Not-a-Tab-stop is proven by the plain-container test.

## estimate_height within +-25%; 10,000-item test passes
Status: PROVEN
```console
$ cargo test -p twicecat-app --test timeline_layout
running 8 tests
test an_empty_timeline_has_only_two_empty_spacers ... ok
test a_thumbnail_arriving_changes_no_laid_out_row_height ... ok
test decorated_rows_are_estimated_within_a_quarter_in_both_densities ... ok
test upload_progress_changes_no_laid_out_row_height ... ok
test estimates_stay_within_a_quarter_of_the_laid_out_rows_for_both_densities ... ok
test a_compact_timeline_lays_out_shorter_rows_on_average ... ok
test only_the_visible_rows_of_10000_items_are_laid_out ... ok
test a_window_in_the_middle_covers_the_visible_range ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

## Avatar requests bounded (cache cap; evicted not re-requested; members asked once)
Status: PROVEN (tests; no real homeserver)
```console
$ cargo test -p twicecat-app --lib   (filtered)
test avatars::tests::the_cache_cap_holds ... ok
test avatars::tests::an_evicted_id_is_not_requested_again ... ok
test avatars::tests::clear_lets_every_id_be_requested_again ... ok
test rooms::tests::duplicate_user_ids_keep_the_first_avatar ... ok
test state::tests::rooms_ui::the_avatars_of_the_senders_on_screen_are_asked_for_once_and_no_others ... ok
test state::tests::rooms_ui::a_message_asks_for_the_members_once_so_headers_can_show_pictures ... ok
test screens::timeline_pane::tests::many_members_and_header_rows_use_the_index_and_give_the_same_pictures ... ok
```
All pass (1 passed each when run by name). The 5000-member index test is many_members_and_header_rows_use_the_index_and_give_the_same_pictures (name from the diff; I did not read its body to confirm the 5000 figure).

## Not checked
Real window, a busy room, hover appearance on screen, font weight measured: human-only.
