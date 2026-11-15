# Evidence — Task 5: rail reorder with sweeten
Commit: c1aa701
Environment: headless; `CC=clang-22 CXX=clang++-22`; cargo tests, cargo tree/deny, gallery writer (tiny-skia). The GUI app was not run (no display, no human).

## sweeten v0.14.0 under twicecat-app only; deny.toml unchanged; Cargo.lock adds only sweeten
Status: PROVEN

```console
$ cargo tree -p twicecat-app -i sweeten -e normal
sweeten v0.14.0
└── twicecat-app v0.1.0 (/home/galep/projects/github/BoredLabsHQ/twicecat/crates/app)

$ git diff 86b50f8..c1aa701 -- deny.toml | wc -c
0

$ git diff 86b50f8..c1aa701 -- Cargo.lock | grep '^[+-]'
--- a/Cargo.lock
+++ b/Cargo.lock
+[[package]]
+name = "sweeten"
+version = "0.14.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "8e743407da2564f19dc27f8b926214d015effd97e3eb08d08c4a39f70afe6c15"
+dependencies = [
+ "iced_core",
+ "iced_runtime",
+ "iced_widget",
+ "unicode-segmentation",
+]
+
+ "sweeten",

$ cargo deny check 2>&1 | tail -1
advisories ok, bans ok, licenses ok, sources ok
```
Only additions: the sweeten package and one `"sweeten"` line in twicecat-app's dependency list; nothing removed or changed.

## `cargo test -p twicecat-app --test ui_gallery rail` passes
Status: PROVEN

```console
$ cargo test -p twicecat-app --test ui_gallery rail
test rail_reorder_drag::a_press_a_four_pixel_move_and_a_release_selects_and_starts_no_drag ... ok
test rail_reorder_drag::a_drag_over_a_closed_folder_puts_the_space_after_it_and_saves_once ... ok
test rail_reorder_drag::a_drag_upwards_puts_the_space_before_the_one_it_is_over ... ok
test rail_reorder_drag::home_cannot_be_picked ... ok
test rail_reorder_drag::a_press_on_a_closed_folders_head_drags_the_whole_folder ... ok
test rail_reorder_drag::a_press_on_an_open_folders_head_picks_nothing_and_no_neighbour ... ok
test rail_reorder_drag::inside_an_open_folder_a_drag_reorders_only_inside_it ... ok
test rail_reorder_drag::dragging_a_space_of_an_open_folder_out_of_it_moves_it_inside_at_most ... ok
test rail_reorder_drag::the_cursor_is_grab_when_idle_and_grabbing_while_dragging ... ok
test rail_reorder_drag::with_a_file_the_app_may_not_change_nothing_is_picked_and_the_cursor_is_a_pointer ... ok
test rail_reorder_drag::esc_and_losing_the_focus_end_the_drag_and_the_release_that_follows_does_nothing ... ok
test rail_reorder_drag::leaving_the_rail_ends_the_drag_and_a_later_release_saves_nothing ... ok
test rail_reorder_drag::keys_that_changed_between_the_pick_and_the_drop_save_nothing ... ok
test rail_reorder_drag::a_click_selects_a_space_and_a_folder_head_toggles_it ... ok
test rail_reorder_drag::a_press_on_one_space_and_a_release_on_another_is_no_click ... ok
test rail_reorder_drag::a_press_over_nothing_on_the_rail_then_a_release_over_a_space_is_no_click ... ok
test rail_reorder_drag::the_rail_is_home_then_the_items_in_the_layouts_order ... ok
(+ 6 other tests whose names contain "rail": rail_marks_and_dots_never_overlap..., rail_states_draws_every_slot..., the_rail_dock_fits_72_px..., the_room_list_is_300_wide_or_a_72_px_rail..., ten_thousand_rooms_..._and_the_rail, security_screens::a_banner_keeps_its_words_whole...)
test result: ok. 23 passed; 0 failed
```
Mapping to the plan: 4.0 px no drag = `a_press_a_four_pixel_move_and_a_release_selects_and_starts_no_drag`; 4.5 px drag saves once = `a_drag_over_a_closed_folder_..._saves_once`; Home = `home_cannot_be_picked`; closed folder head = `a_press_on_a_closed_folders_head_drags_the_whole_folder`; open folder = `inside_an_open_folder_a_drag_reorders_only_inside_it`; cursor Grab/Grabbing/Pointer = the two cursor tests; Esc and WindowFocus(false) = `esc_and_losing_the_focus_...`; leaving the scrollable = `leaving_the_rail_ends_the_drag_...`; keys changed = `keys_that_changed_between_the_pick_and_the_drop_save_nothing`. (The 4.5 px test name does not itself say "4.5"; I did not read the test bodies, the mapping is by name and by PROGRESS.md.)

## App unit tests: rail_reorder::, state::tests::rail_dragging, room-list drag tests unchanged
Status: PROVEN

```console
$ cargo test -p twicecat-app --lib | grep -E "rail_reorder|rail_dragging|list_drop|sections::tests::dragging|test result"
rail_reorder::tests:: 7 tests ok (larger_target.., stored_space_the_core_does_not_report.., sweeten_event_becomes_ours.., equal_indices.., inside_a_three_space_folder.., the_keys_of_a_column.., on_a_four_item_rail..)
state::tests::rail_dragging:: 18 tests listed, all ok (e.g. a_canceled_drag_saves_nothing_and_ends, esc_and_losing_the_focus_cancel_flip_the_generation_and_save_nothing, keys_that_changed_between_the_pick_and_the_drop_save_nothing, a_closed_folder_is_dragged_as_one_item, inside_an_open_folder_a_drag_reorders_only_inside_it ...)
list_drop::tests:: 8 ok
sections::tests::dragging:: 11 ok (incl. a_drag_is_no_click, esc_and_losing_the_focus_cancel_and_save_nothing, the_same_target_is_planned_once_...)
test result: ok. 1724 passed; 0 failed
```
Whole app lib suite: 1724 passed, 0 failed. Note: the lines above are abbreviated by me from the real output (the first listing was filtered with grep and head); the pass/fail totals are verbatim. I did not diff the room-list tests across commits myself; PROGRESS.md says they only changed `Press` to `RailPressed` and `rail_drag` to `rail_press` in a few older tests.

## `grep -rn "onto its middle to make a folder" docs/` finds only the plan's acceptance line
Status: PROVEN

```console
$ grep -rn "onto its middle to make a folder" docs/
docs/plans/sidebar-rework.md:407:  - `grep -rn "onto its middle to make a folder" docs/` finds nothing.
```
Only the plan's own line (this evidence file contains the phrase now too, as a quoted criterion; run before it was written).

## Gallery image
Status: PROVEN (image written; a visual aid only)

```console
$ TWICECAT_GALLERY_DIR=target/ui-gallery TWICECAT_GALLERY_ONLY=rail-dragging cargo test -p twicecat-app --test ui_gallery -- --ignored write_the_gallery
test write_the_gallery ... ok
$ ls -l target/ui-gallery/rail-dragging-*.png
121033 rail-dragging-daylight-comfortable.png
 72451 rail-dragging-daylight-comfortable-rail.png
106992 rail-dragging-daylight-compact.png
 72683 rail-dragging-daylight-compact-rail.png
119597 rail-dragging-twicecat-comfortable.png
 71412 rail-dragging-twicecat-comfortable-rail.png
105483 rail-dragging-twicecat-compact.png
 71618 rail-dragging-twicecat-compact-rail.png
```
Read `/home/galep/projects/github/BoredLabsHQ/twicecat/target/ui-gallery/rail-dragging-twicecat-comfortable-rail.png` (1280x800, untracked): the 72 px rail at left shows Home (with a badge), then items HH, QL, B (3). The item "H" is drawn lifted: a rounded outlined frame and larger circle, with the outline spilling onto the row above and below. The next item "SH" has a rounded highlighted frame, which looks like the ghost/placeholder slot. Neighbours below (a folder of small tiles with a 99+ badge, another folder with badge 1, G, K (1)) are in their rows. Whether the image's gaps are "slid aside" or merely the layout I cannot tell from one still. This is an aid, not proof of how the drag feels or animates.

## Mutation 1: drop the `rail_press == Some(key)` check
Status: PROVEN (re-run by me)

The check is in `SpacesMessage::Release` in crates/app/src/spaces.rs line 389 (`let clicked = self.rail_press.as_ref() == Some(&key);`). I replaced it with `let clicked = true;`:

```console
$ cargo test -p twicecat-app --test ui_gallery rail
test rail_reorder_drag::a_press_over_nothing_on_the_rail_then_a_release_over_a_space_is_no_click ... FAILED
test rail_reorder_drag::a_press_on_an_open_folders_head_picks_nothing_and_no_neighbour ... FAILED
test rail_reorder_drag::esc_and_losing_the_focus_end_the_drag_and_the_release_that_follows_does_nothing ... FAILED
test rail_reorder_drag::keys_that_changed_between_the_pick_and_the_drop_save_nothing ... FAILED
test result: FAILED. 19 passed; 4 failed
$ git checkout crates/app/src/spaces.rs   # restored; git status --short empty
```
Matches the four gallery failures PROGRESS.md records. The `state.rs` failures PROGRESS.md mentions (2) I did not confirm: my `--lib` run output was cut by `head`, so I only confirmed the gallery failures.

## Not proven here (human checkpoint)
Status: NOT PROVEN (needs a real window and a human)

Not proven: how the drag feels (lift, slide, settle), the Grab/Grabbing/Pointer cursor in a real window (only the cursor value is tested in the widget tree), and Esc/alt-tab during a live drag. "Sidebar rework human checkpoint" exists in PROGRESS.md (line 4204), unticked, with these items:
- Task 5 rail drag in a real window: feel (1.05 lift, others sliding, ghost, all themes and densities, settle); cursor Grab/Grabbing/Pointer; drag starts after more than 4 px, wobbly click still selects; Esc and alt-tab end the drag at once and save nothing, leaving the rail or releasing outside the window leaves no stuck drag; right-click opens the card, right-click during a drag ends it; folder behaviour (inside an open folder, closed folder by its head, open folder head picks nothing); folders only via the menu (acceptability); touch does not reorder.
- Task 5 room list drag unchanged (line, header highlight, refusal).

## Verdict
EVIDENCE: PARTIAL (all headless criteria proven; drag feel, real-window cursor and Esc during a live drag are unproven and sit on the human checkpoint)
