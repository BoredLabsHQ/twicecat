# Evidence — Task 11: Discord categories: headers and channel rows in a space
Commit: c962a04 (Task 11 = e8ef543 + c962a04; diff base f19a575)
Environment: headless; `CC=clang-22 CXX=clang++-22`; cargo tests only (no GUI run, no display). Images stay in `target/ui-gallery` (git-ignored).

## channel_glyph: call room Speaker, DM Avatar, else Hash
Status: PROVEN (pure function, run through its unit test)
`cargo test -p twicecat-app --lib` (1805 passed, 0 failed): `room_list::pure_tests::a_channels_glyph_is_a_speaker_for_a_call_room_an_avatar_for_a_dm_else_a_hash ... ok`

## channel_emphasis: unread/notifications/mark = Unread; Mute = Muted even with unread; else Read
Status: PROVEN
`room_list::pure_tests::a_channels_emphasis_follows_unread_state_and_mute_mode ... ok`; `screens::room_list_pane::tests::emphasis_differs_in_colour_unread_from_read_and_muted_from_read ... ok`
Mutation 1 (skip the Mute check when `unread_notifications > 0`, room_list.rs line 203), reverted with `git checkout`:
```console
test room_list::pure_tests::a_channels_emphasis_follows_unread_state_and_mute_mode ... FAILED
panicked at crates/app/src/room_list.rs:1643:13: assertion `left == right` failed
  left: Unread
 right: Muted
```
The plan's mutation fails its test.

## Line heights unchanged in both densities
Status: PROVEN
Names that ran and passed: `room_list::pure_tests::row_heights_follow_the_density_and_the_rail_is_fixed`, `ui::tokens::tests::density_sets_the_row_heights_and_trims_only_the_vertical_padding`, `state::tests::an_arriving_thumbnail_changes_no_row_height`, `state::tests::settings_ui::a_row_height_change_keeps_the_place_in_the_list`. The `--lib` run filtered to `line_offsets` / `line_height` shows no test with those words in its name in the lib output I grepped; the line_offsets tests are in the 1805 passed (full lib run 0 failed). `git diff f19a575..c962a04 -U0 -- crates/app/src/room_list.rs` has hunks only at +10 (import), +152 (new fns) and +1603 (new tests): `line_height` and `line_offsets` are not edited. (I did not isolate a test literally named `line_offsets`; honest gap in the filter, covered by the whole lib suite passing.)

## Badge of a channel row equals badge_for (table test)
Status: PROVEN
`screens::room_list_pane::tests::the_badge_of_a_channel_row_is_badge_for_of_the_same_room ... ok`; also `channel_rows_and_headers_build_in_every_state ... ok`

## Gallery space-categories: layout and overlap at 1280 and 720, both densities
Status: PROVEN
```console
$ cargo test -p twicecat-app --test ui_gallery
test result: ok. 183 passed; 0 failed; 1 ignored; 0 measured; 183 filtered out
```
`space-categories` is in the scenario list (ui_gallery.rs lines 416 and 1696, selected by name at 1800) and runs inside `space_sections_draw_every_line_whole_and_apart_in_both_densities_and_themes` (layout/overlap at 1280 and 720, both densities, rail and expanded). Also passing: `space_categories_draw_headers_in_capitals_and_channels_without_previews`, `a_channel_row_alone_is_row_height_tall_with_its_glyph_slot`, `a_room_dragged_onto_a_section_header_moves`.

## Contrast in every theme::test_themes()
Status: PROVEN
Test: `screens::room_list_pane::tests::unread_names_read_at_4_5_and_read_names_and_headers_at_3_in_every_theme ... ok` (also asserts read <= unread and muted <= read on every ground; hovered header >= 4.5) and `the_contrast_probe_fails_a_worse_colour ... ok`.
Measured with a temporary eprintln in the test (reverted with `git checkout`; tree clean), minimum over the three row grounds (list, hover, selection):
```
Twicecat  unread 13.70  read 6.40  muted 3.13  header 6.40
Daylight  unread 13.08  read 5.58  muted 3.34  header 5.58
Duo       unread 13.70  read 6.25  muted 3.13  header 6.25
Faint     unread  6.70  read 5.94  muted 3.28  header 11.79
Paper     unread 13.90  read 6.42  muted 3.28  header 6.42
```
Unread >= 4.5 and read, muted, header >= 3 hold in all five. On Faint, read (5.94) is below unread (6.70), as the follow-up requires.
Mutation 3 (`Emphasis::Read => theme::semantic(theme).muted`), reverted:
```console
test ...unread_names_read_at_4_5_and_read_names_and_headers_at_3_in_every_theme ... FAILED
panicked at room_list_pane.rs:1955:17: Faint: read 14.61 > unread 8.30
```

## Headers uppercase and capped
Status: PROVEN
`room_list_pane::tests::a_header_that_grows_in_capitals_is_still_within_the_cap_and_the_mark_limit ... ok`; `space_categories_draw_headers_in_capitals_and_channels_without_previews ... ok`; PNGs show STARRED, CHANNELS, TECH, HOBBIES, VOICE.

## A lone channel_row is row_height tall with a non-zero glyph slot
Status: PROVEN
`a_channel_row_alone_is_row_height_tall_with_its_glyph_slot ... ok` (both densities, call or not, selected or not).
Mutation 2 (height + 6 px in channel_row), reverted:
```console
test a_channel_row_alone_is_row_height_tall_with_its_glyph_slot ... FAILED
Comfortable call=false selected=false: 70 high
```
I did not separately run the zero-width-glyph-slot mutation (PROGRESS.md records it failing; not re-run here).

## Flat and Home code paths untouched
Status: PROVEN
```console
$ git diff f19a575..c962a04 -U0 -- crates/app/src/screens/room_list_pane.rs | grep '^@@'
@@ -32,2 +32,3 @@ use ...            (import)
@@ -548,2 +549,2 @@ fn scroller      (scroller, not in the list)
@@ -576,7 +577,8 @@ fn sectioned_list
@@ -821,0 +824,6 @@ fn section_header   (and 4 single-line hunks at 825/829/832 in section_header)
@@ -1122,0 +1131,246 @@ fn room_row_in  (pure insertion after room_row_in, which ends at line 1128; new fns)
@@ -1673,0 +1928,189 @@ mod tests
```
Definitions are at windowed_list 475, room_row 1003, room_row_in 1029 (ends 1128), rail_row 1400 (post-change numbering). No hunk falls inside those four function bodies (hunks are at 32, 548, 576, 821-832, 1122 pure add after room_row_in's closing brace, 1673), so they are unchanged. `windowed_list` and `rail_row` are not touched at all. Note `section_header` and `sectioned_list` do change (expected).
```console
$ git diff f19a575..c962a04 --stat -- crates/app/src/keys.rs deny.toml Cargo.lock Cargo.toml docs/plans
(empty, 0 bytes)
```

## docs_sidebar
Status: PROVEN: `cargo test -p twicecat-app --test docs_sidebar`: 13 passed, incl. `the_docs_describe_the_discord_category_look_and_its_words`.

## Gallery PNGs (aid only)
`TWICECAT_GALLERY_DIR=target/ui-gallery TWICECAT_GALLERY_ONLY=space-categories ... write_the_gallery`: ok. Written: space-categories-{twicecat,daylight}-{comfortable,compact}[-rail].png, 8 files, 63,691 to 112,829 bytes (twicecat-comfortable 103,834; daylight-compact 111,495... see `ls`; daylight-compact 112,829).
Read: `space-categories-twicecat-comfortable.png` (dark): capital muted headers STARRED (with "personal" tag and dot), CHANNELS, TECH (badge 2), HOBBIES (11), VOICE, each with a left arrow and "..."; "#" rows: General in bold white with a small left pill and a "3" badge, Random dimmed with a Muted tag and a dot, Announcements read; selected voice room "vocal" with a speaker glyph, lock and call count 3, highlighted; in this 800 px-high view the VOICE category sits at the bottom and its members are cut off by the call bar. `space-categories-daylight-compact.png` (light): same structure, compact rows, headers in capitals, General bold, Random lighter with Muted tag, selected "vocal" in green tint, members Rhaenyra, Septimus (muted mic), galep under it, and "coworking" with one member (corin). The speaker glyph is visibly small beside "#".
These are aids, not proof of the look.

## NOT PROVEN
How the categories look against Discord in a real window (hover, real fonts, every shipped theme) is NOT proven here; it is on the human checkpoint. No GUI was run.

## PROGRESS.md
"Sidebar rework human checkpoint" (line 4412) has one Task 11 item (line 4435) covering: capital headers/hover/badge, "#" and speaker, no preview/time, unread bold with bar, muted dimmed with tag and badge, selected/hover, dimming readability, "the comfortable rows are 64 px, taller than Discord's: acceptable?", flat/Home unchanged. The Voice-below-first-screen and the speaker-icon-size points are NOT in that checkpoint item; they are recorded only under Step 11 "Open issues" with: no real-window verification; 64 px comfortable rows and the voice category below the first screen at 800 px high with the call bar open; small `Icon::Speaker` at 14 px; Task 12 not done.

## Verdict
Working tree clean after all mutations and probes (`git status --short` empty; only this file added).
