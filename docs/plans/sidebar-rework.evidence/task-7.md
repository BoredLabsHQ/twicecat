# Evidence — Task 7: filter chips for the room list
Commit: 166343b (diff base 4db39ce)
Environment: headless, CC=clang-22 CXX=clang++-22, cargo tests and the tiny-skia gallery only. The GUI app was NOT run (no display).

## Each chip sends exactly one query with its tab; the active chip sends All (search and space kept); All/DMs/Rooms/Spaces give the old queries
Status: PROVEN (unit level)
```console
$ cargo test -p twicecat-app --lib -- chip every_chip hint empty_label contrast unread
test room_list::tests::every_chip_sends_exactly_one_query_with_its_tab ... ok
test room_list::tests::clicking_the_active_chip_sends_all_and_keeps_search_and_space ... ok
test room_list::pure_tests::every_chip_has_a_label_a_title_and_a_hint ... ok
test room_list::pure_tests::empty_labels ... ok
test ui::kit::tests::a_filter_chip_reads_at_4_5_to_1_in_every_status_and_theme_and_one_step_worse_fails ... ok
test theme::tests::contrast_matches_the_wcag_reference_values ... ok
(plus 18 other name matches, all ok: reaction-chip, badge, switcher hint, sections hint, keyboard unread tests)
```
Message::ChipClicked in crates/app/src/state.rs (line ~1765) keeps search and space and maps the active tab to All; the two room_list tests assert it. I did not separately run a test showing the All/DMs/Rooms/Spaces queries equal the old tabs' queries; every_chip_sends_exactly_one_query_with_its_tab covers the per-chip query, the "same as old tabs" claim rests on it.

## Empty Unreads shows 'No unread rooms'; every chip has a hint
Status: PROVEN
`room_list::pure_tests::empty_labels` (asserts `empty_label(Unreads, "") == "No unread rooms"`, room_list.rs:1398) and `every_chip_has_a_label_a_title_and_a_hint` (table over all chips) both ok, see above.

## Gallery: wrap, no overlap or clipping, room-list layout checks, contrast >= 4.5:1
Status: PROVEN
```console
$ cargo test -p twicecat-app --test ui_gallery   (full)
test the_chips_wrap_without_overlap_or_clipping_at_every_pane_width ... ok
test the_chips_wrap_in_the_pane_and_no_chip_is_drawn_in_the_rail ... ok
test no_two_things_in_a_room_list_row_overlap_and_none_reaches_the_scrollbar ... ok
test the_room_list_is_300_wide_or_a_72_px_rail_and_keeps_its_five_slots ... ok
test dialogs::the_search_narrows_the_list_and_an_empty_result_says_so ... ok
test result: ok. 177 passed; 0 failed; 1 ignored
```
Measured chip text contrast. kit.rs had a temporary eprintln (reverted with git checkout; tree clean) in the test a_filter_chip_reads_at_4_5_to_1...; 48 values (5 themes x 4 statuses x 2 active states... plus per-status, 49 lines incl. result). Minimum ratio per theme, over every status and active/inactive, identified by the palette text colour:
```console
text=(0.039,0.031,0.071) min 9.873
text=(0.039,0.047,0.094) min 10.062
text=(0.078,0.094,0.114) min 10.089
text=(0.937,0.922,0.980) min 8.705
text=(0.661,0.648,0.717) min 5.033   (lowest overall; Pressed, active=true)
```
All >= 4.5. The test also asserts that a text mixed 85% to its background would fail (the check bites). Which theme name belongs to which text colour was not printed (the test prints none); the 5 values are test_themes() entries.

Layout reading (layout.rs): `WIDE_MIN = 1100.0`, `MEDIUM_MIN = 800.0`; `layout_for` sets `sidebar: medium = width >= 800` and the list is the user's choice only when `wide || (medium && !any)`, else a rail. I did not trace what happens below 800 in full, but the tests check no chip at 720 and wrapping in the 300 px pane at 1280, which is consistent with the reviewer's note. The note's reading that the pane cannot be shown wrapped at 720 is plausible from these constants; I confirm the constants, not every branch.

## Collapsed rail draws no chip
Status: PROVEN (layout test)
`the_chips_wrap_in_the_pane_and_no_chip_is_drawn_in_the_rail ... ok`. Gallery image room-list-chips-twicecat-comfortable-rail.png (read) shows +, person, chevron, search, avatars: no chip.

## `git diff 4db39ce..166343b --stat -- crates/app/src/keys.rs deny.toml Cargo.lock Cargo.toml`
Status: PROVEN
```console
$ git diff 4db39ce..166343b --stat -- crates/app/src/keys.rs deny.toml Cargo.lock Cargo.toml
$ 
```
Empty.

## Gallery PNGs (aid only, not proof)
```console
$ TWICECAT_GALLERY_DIR=target/ui-gallery TWICECAT_GALLERY_ONLY=room-list-chips cargo test -p twicecat-app --test ui_gallery -- --ignored write_the_gallery
test write_the_gallery ... ok
-rw-rw-r-- 120459 room-list-chips-daylight-comfortable.png
-rw-rw-r--  61146 room-list-chips-daylight-comfortable-rail.png
-rw-rw-r-- 106342 room-list-chips-daylight-compact.png
-rw-rw-r--  61387 room-list-chips-daylight-compact-rail.png
-rw-rw-r-- 118482 room-list-chips-twicecat-comfortable.png
-rw-rw-r--  59714 room-list-chips-twicecat-comfortable-rail.png
-rw-rw-r-- 104320 room-list-chips-twicecat-compact.png
-rw-rw-r--  59920 room-list-chips-twicecat-compact-rail.png
```
target/ui-gallery is git-ignored. Read twicecat-comfortable.png (1280x800, 300 px pane): chips in order All, Unreads, Mentions, DMs, Rooms (line 1), Favourites, Low priority, Invites, Spaces (line 2). That is nine chips on two wrapped lines. Unreads is the active one (lilac fill and outline). Search field below; no clipping seen. The rail PNG (72 px rail) shows no chips. "Narrow" is the rail, per the layout note; no image at 300/240/180 px was produced, those widths are covered by the layout test only.

## Mutations
1. Removing only `.wrap()` from chips() does not compile (`.vertical_spacing` exists only on the wrapping row), so no test result. Removing `.wrap()` and `.vertical_spacing(...)`:
```console
test the_chips_wrap_without_overlap_or_clipping_at_every_pane_width ... FAILED   (All in 300 px: 5 leaves)
test the_chips_wrap_in_the_pane_and_no_chip_is_drawn_in_the_rail ... FAILED      (rail-states twicecat-comfortable at 1280: 5 leaves)
test result: FAILED. 2 passed; 2 failed
```
Caught. Restored with git checkout.
2. ChipClicked made not to toggle (`let tab = tab;`):
```console
test room_list::tests::clicking_the_active_chip_sends_all_and_keeps_search_and_space ... FAILED
assertion `left == right` failed: Unreads: exactly one command  left: 0  right: 1
```
Caught. Restored with git checkout.

## PROGRESS.md "Sidebar rework human checkpoint"
Status: PROVEN. A "(Task 7) The filter chips in a real window" item (unchecked) lists: every shipped theme, both densities; chips wrap onto more lines in the 300 px pane without clipping and look right when narrow; the active chip is clear (fill and outline) in every theme including high contrast; hover and press feel right; clicking the active chip goes back to All; no chip in the collapsed rail; the Favourites chip lists a room favourited in Element on the same test account.

## NOT PROVEN here
Look and feel in a real window, and the Favourites chip listing a room favourited in Element against the local .env.test account. Both are on the human checkpoint.

## Verdict
EVIDENCE: PARTIAL (headless criteria proven; real-window look and Element-favourite listing unproven, on the human checkpoint)
