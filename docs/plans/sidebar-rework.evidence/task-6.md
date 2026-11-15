# Evidence — Task 6: separator under Home and sub-spaces under their space on the rail
Commit: 34eeb41 (Task 6 = 04893c1 + 34eeb41; base 4dd4865)
Environment: headless, CC=clang-22 CXX=clang++-22, cargo tests only; GUI app NOT run (no display).
Amended criteria: an EXPANDED parent is not draggable (picks nothing, lifts no neighbour), moves via its card; separator alpha is the least alpha (5% steps from 20%) giving >= 3:1 per theme.

## Unit tests (`cargo test -p twicecat-app --lib`)
Status: PROVEN
```console
test result: ok. 1741 passed; 0 failed
```
Relevant names that ran ok: spaces::tests::{sub_spaces_are_the_level_1_spaces_in_the_parents_rooms_in_the_cores_order, a_level_1_space_a_missing_space_and_a_space_with_none_have_no_sub_spaces, a_room_a_level_2_space_and_a_duplicate_are_not_sub_spaces, a_parent_holds_the_selected_item_only_through_its_sub_spaces};
state::tests::space_tree::{a_click_on_the_chevron_toggles_the_tree_and_saves_nothing, logout_and_session_expiry_empty_the_tree, a_login_of_another_account_with_no_logout_empties_the_tree, selecting_a_sub_space_filters_the_list_to_it_and_sections_it, a_selected_sub_space_survives_its_tree_closing, the_card_entries_show_and_hide_the_tree_and_save_nothing, the_card_offers_one_tree_entry_only_to_a_space_that_has_sub_spaces, the_tree_toggle_ends_a_rail_drag_and_forgets_a_press, a_space_with_no_sub_spaces_and_an_unknown_space_do_not_toggle, a_sub_spaces_hover_survives_while_its_parent_is_open_and_goes_when_it_closes};
sections::tests::{a_selected_sub_space_lists_its_own_rooms_in_channels_direct_messages_and_its_sections, a_level_1_space_in_two_sub_spaces_is_placed_once, sub_space_rows_are_not_rows_and_a_level_2_room_lands_in_its_level_1_section, starred_beats_custom_beats_sub_space_beats_dm_beats_channels}.

## Separator >= 3:1 against the rail in every test theme
Status: PROVEN
`screens::spaces_sidebar::tests::the_separator_is_at_least_3_to_1_against_the_rail_in_every_test_theme ... ok`
The test asserts the ratio and that one 5% step less would fail (least alpha). It prints nothing, so I added a temporary eprintln (reverted with git checkout, tree clean) and ran it; real output, one line per theme in test_themes() order (identified by the palette text colour):
```console
THEME text=(0.937,0.922,0.980) alpha=0.4  ratio=3.376
THEME text=(0.078,0.094,0.114) alpha=0.5  ratio=3.395
THEME text=(0.937,0.922,0.980) alpha=0.4  ratio=3.376
THEME text=(0.039,0.047,0.094) alpha=0.45 ratio=3.090
THEME text=(0.661,0.648,0.717) alpha=0.55 ratio=3.171
THEME text=(0.039,0.031,0.071) alpha=0.45 ratio=3.149
```
All >= 3.0; alphas 40-55%, matching PROGRESS.md.

## Gallery drag/layout tests (`cargo test -p twicecat-app --test ui_gallery rail`)
Status: PROVEN
```console
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 140 filtered out (36 passed)
```
Names include: a_press_on_a_sub_space_a_10_px_move_and_a_release_start_no_drag; an_expanded_space_is_not_lifted_by_its_head_and_no_neighbour_is; the_bottom_padding_of_an_expanded_spaces_band_lifts_nothing_with_a_neighbour_below; a_collapsed_space_with_sub_spaces_still_drags_by_its_head; a_click_on_the_chevron_toggles_the_tree_and_selects_and_saves_nothing; a_press_on_the_chevron_then_a_move_starts_no_drag; a_click_on_a_sub_space_selects_it_and_saves_nothing; a_sub_space_cannot_be_dragged_even_onto_another_item; the_expanded_space_moves_with_its_card_and_the_cursor_over_it_is_a_pointer; a_closed_chevron_shows_on_hover_only_and_a_drag_never_starts_from_it; a_click_on_the_parent_selects_it_and_does_not_open_or_close_its_tree; the_parent_below_an_open_tree_can_be_dragged_up_over_it; a_press_on_an_open_folders_head_picks_nothing_and_no_neighbour; a_press_on_a_closed_folders_head_drags_the_whole_folder; home_cannot_be_picked; rail_tree_draws_the_separator_and_every_slot_whole_and_apart_at_every_width (1280/1000/720, both densities); rail_states_draws_every_slot_whole_and_apart_at_every_width; rail_marks_and_dots_never_overlap_each_other; the_room_list_is_300_wide_or_a_72_px_rail_and_keeps_its_five_slots.

## Gallery PNGs (an aid, NOT proof)
Status: PROVEN (generated); look is NOT PROVEN
`TWICECAT_GALLERY_DIR=target/ui-gallery TWICECAT_GALLERY_ONLY=rail-tree ... write_the_gallery` -> ok. rail-tree-* PNGs (target/ui-gallery is git-ignored):
rail-tree-daylight-comfortable.png 115075, -rail 66422; daylight-compact 100929, -rail 66700; twicecat-comfortable 113381, -rail 65070; twicecat-compact 99109, -rail 65279 bytes.
Read rail-tree-twicecat-comfortable-rail.png (1280x800, tiny-skia): Home (light circle, badge 7) at top; a thin grey separator line under it; an orange space "Q" with badge 3 and a chevron (down) at its corner, with a darker band below holding two small square sub-spaces: "D" (teal, light ring = selected) and "E" (pink, badge 3); then a closed folder (green "B" with right chevron) with two mini tiles R, V in a rounded dark tile. Nothing overlaps. Rendered by a software renderer: no pointer, motion or real font/GPU look; an aid only.

## Keys/deps untouched
Status: PROVEN
```console
$ git diff 4dd4865..34eeb41 --stat -- crates/app/src/keys.rs deny.toml Cargo.lock Cargo.toml
(empty, rc=0)
```

## Mutation: sub-space slot does not capture the press
Status: PROVEN for the click/select test; see caveat
Spot (follow-up did not move it): crates/app/src/screens/spaces_sidebar.rs:704 `area.on_press(select).into()` in the `(Kind::Sub, RailKey::Space(id))` arm. Replaced with `{ let _ = select; area.into() }`:
```console
test rail_reorder_drag::a_click_on_a_sub_space_selects_it_and_saves_nothing ... FAILED
assertion `left == right` failed: the press   (ui_gallery.rs:7887)
test result: FAILED. 28 passed; 1 failed
```
Restored with `git checkout` (tree clean). CAVEAT: `a_press_on_a_sub_space_a_10_px_move_and_a_release_start_no_drag` did NOT fail under this mutation; only the click test caught it. So the "no drag from a sub-space" test is not mutation-sensitive to this particular change (possibly because the press is not routed via the real column/MouseArea in that test). Flagged as a weakness.

## PROGRESS.md "Sidebar rework human checkpoint" (line 4238+)
Status: PROVEN
Task 6 item at line 4249 lists: separator strength (40-55%, not 20%) and spacing; chevron hit area (16 px mark, legible over avatars, no flicker); sub-space look (32 px icons, indent, tint, pill, badge, tooltip, selection shows room list); closed parent with selected sub-space looks selected; expanded space not dragged, Move up/down on card work, collapsed still drags; menu entries Show/Hide sub-spaces and tree closed after restart. "Band spacing" has no separate bullet; it is covered under the band bullet (indent) and the separator's "spacing".

## NOT PROVEN here (on the human checkpoint)
How it looks and feels in a real window, pointer/motion, and the chevron showing sub-spaces in a running app against the local .env.test account: the GUI was not run.

## Verdict
EVIDENCE: PARTIAL
