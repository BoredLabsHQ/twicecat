# Evidence — Task 12: voice after text; a collapsed category keeps the open room and the call
Commit: aa3bde9 (task = 2fe58b4 + aa3bde9; diff base fede4c8)
Environment: headless, `CC=clang-22 CXX=clang++-22`, cargo tests and the tiny-skia gallery only. The GUI was not run (no display).

## Scope statement
"Collapsing the category of the open room keeps that room in view" in a real window is NOT PROVEN here. It is on the PROGRESS.md human checkpoint (Task 12 item). Everything below is unit/gallery level.

## sections::build tests (`cargo test -p twicecat-app --lib sections`)
Status: PROVEN (139 passed, 0 failed)
Ran, among others:
- categories: text_rooms_come_first_in_channels_under_every_sort, every_section_the_user_does_not_order_puts_voice_after_text, low_priority_puts_voice_after_text_under_two_sorts, a_custom_section_keeps_the_users_order_with_a_voice_room_first, starred_keeps_the_users_order_with_a_voice_room_first, a_collapsed_section_with_the_selected_room_draws_its_header_and_that_room_only, with_this_devices_call_in_it_a_collapsed_section_draws_the_voice_room_and_its_people, a_collapsed_section_with_neither_room_draws_the_header_only, the_header_badge_counts_every_room_of_the_section_kept_or_not, an_unread_room_that_is_not_open_is_not_kept, someone_elses_call_keeps_nothing_in_a_collapsed_section, a_collapsed_custom_section_keeps_the_open_room_in_the_users_order, kept_rooms_are_ordinary_lines_for_heights_and_spots
- stepping_a_collapsed_category: alt_down_skips_the_hidden_rooms_and_reaches_the_kept_one, alt_up_skips_the_hidden_rooms_and_reaches_the_kept_call_room
- dragging (kept rooms, hidden order unchanged): dragging_a_kept_room_in_a_collapsed_custom_section_keeps_the_hidden_rooms_in_place, dragging_the_kept_room_of_a_collapsed_starred_keeps_the_hidden_rooms_in_place, dropping_on_a_collapsed_custom_section_loses_no_room
Last line: `test result: ok. 139 passed; 0 failed; 0 ignored; 0 measured; 1684 filtered out`

## ui_gallery and docs_sidebar
Status: PROVEN
- `cargo test -p twicecat-app --test ui_gallery`: `test result: ok. 184 passed; 0 failed; 1 ignored` (the ignored one is write_the_gallery).
- space-categories-collapsed is named in `the_fixture_reaches_every_screen_it_names` (ok) and in `space_sections_draw_every_line_whole_and_apart_in_both_densities_and_themes` (the layout/overlap test, ok); `a_collapsed_category_still_draws_the_open_room_and_the_call` (new) also ok.
- `cargo test -p twicecat-app --test docs_sidebar`: 14 passed, 0 failed (includes the_docs_say_voice_follows_text_and_a_collapsed_category_keeps_the_open_room_and_the_call).

## space-sections-collapsed test unchanged
Status: PROVEN
`git diff fede4c8..aa3bde9 -- crates/app/tests/ui_gallery.rs`: two hunks adding `"space-categories-collapsed",` to two name lists (lines ~414 and ~1695) plus one new test `a_collapsed_category_still_draws_the_open_room_and_the_call`. No line of the `space-sections-collapsed` test body is touched.

## Files that must not change
Status: PROVEN
`git diff fede4c8..aa3bde9 --stat -- crates/app/src/keys.rs crates/app/src/state.rs deny.toml Cargo.lock Cargo.toml docs/plans` printed nothing (empty). (This file is added afterwards under docs/plans.)

## Gallery (aid only, not proof)
```console
$ TWICECAT_GALLERY_DIR=target/ui-gallery TWICECAT_GALLERY_ONLY=space-categories-collapsed cargo test -p twicecat-app --test ui_gallery -- --ignored write_the_gallery
test write_the_gallery ... ok
```
Eight PNGs written (target/ is git-ignored, nothing tracked): space-categories-collapsed-{daylight,twicecat}-{comfortable,compact}.png (about 106-108 KB each) and the same with `-rail` (about 64-65 KB each).
Read space-categories-collapsed-twicecat-comfortable.png: capital headers with arrows: STARRED ("personal" and a dot), CHANNELS badge 3 with only "General" kept under it (selected, bold, lock, own badge 3), TECH 2, HOBBIES 11, VOICE (collapsed) with the kept voice room "vocal" and its three members (Rhaenyra with ring, Septimus with muted icon, galep), DIRECT MESSAGES with a dot; no other room is drawn; the call bar is below. Real rendering is on the checkpoint.

## Mutations (each restored with `git checkout`; `git status` clean afterwards)
1. Keep every unread room in a collapsed section.
   - First attempt used `room.unread_notifications > 0`: the Q3 test did NOT fail (it sets `unread_messages`/`highlights`, not `unread_notifications`); only the_header_badge_counts_every_room_of_the_section_kept_or_not failed. That was my mistake in picking the field, not a test gap.
   - Redo with `|| room.unread_messages > 0 || room.highlights > 0` in the keep condition: `an_unread_room_that_is_not_open_is_not_kept ... FAILED` (panic at sections.rs:4586), 12 passed, 1 failed. FAILS as the plan requires.
2. Channels left out of the voice-after-text partition: `text_rooms_come_first_in_channels_under_every_sort` FAILED and `with_this_devices_call_in_it_a_collapsed_section_draws_the_voice_room_and_its_people` FAILED (11 passed, 2 failed). FAILS.
3. Collapsed section keeps nothing (`if false && (...)`): 5 categories tests failed (selected room only, kept_rooms_are_ordinary_lines, custom order, badge, call room) and in the gallery `a_collapsed_category_still_draws_the_open_room_and_the_call` FAILED (ui_gallery.rs:1892). FAILS.

## PROGRESS.md
Status: PROVEN
- "Sidebar rework human checkpoint" has the "(Task 12) A collapsed category keeps the open room and the call, and voice follows text, in a real window" item (unchecked).
- The Task 11 item lists the two added points: "with the call bar open at about 800 px high, the Voice category is below the first screen in the comfortable density, and its call members are cut off: acceptable?" and "the speaker icon (14 px) is small beside the '#': acceptable?".
- Task 12 open issues are recorded: opening another room hides the previously kept one at once; the call room is kept while the call is Joining, Connected or Reconnecting (any phase); the badge still counts the kept room's unreads. Also recorded: collapsing the category of the open room in a real window is not verified here.

## Verdict
EVIDENCE: PARTIAL (all headless criteria proven; collapsing the open room's category in a real window, a live call, hover and click are NOT proven and are on the human checkpoint)
