# Evidence — Task 9b: A room card in the flat list
Commit: ef7caab (diff base bde8625). Headless only; `CC=clang-22 CXX=clang++-22`. Tree clean before and after (mutations restored with `git checkout`). The GUI app was not run (no display).

## Right-click on a flat row (Home, a chip other than All, a search result) opens the card
Status: PROVEN (model and widget level; not in a real window)
`cargo test -p twicecat-app --lib flat`: 19 passed, 0 failed. Names: sections::tests::menu::{a_right_click_on_a_flat_row_opens_its_card_under_home_a_chip_and_a_search, a_flat_card_closes_when_the_list_turns_sectioned, the_flat_card_does_not_open_where_the_list_is_sectioned_or_a_rail_or_for_a_stranger, a_flat_card_for_a_room_that_left_the_list_sends_nothing, the_flat_card_offers_only_the_tags, the_flat_cards_entries_send_one_set_room_tag_close_it_and_save_no_file, the_flat_target_hides_the_room_id_in_debug, no_card_opens_in_the_flat_list_or_while_customizing_is_off_or_a_layer_is_up}, plus the older flat-list tests (the_flat_list_is_the_rooms_in_their_order_as_before, home_a_search_another_tab_or_the_rail_keep_the_flat_list, the_flat_list_shows_no_people, the_keys_walk_the_flat_list_when_sections_do_not_show, the_flat_list_still_pages_only_near_its_end, sorted::* flat tests, dragging::in_the_flat_list_or_without_a_layout_nothing_is_draggable).
`cargo test -p twicecat-app --lib sections::tests::menu`: 25 passed (the 8 new flat ones above, the rest are the Task 9 and earlier ones, unchanged).
ui_gallery (real events through the widgets): `a_right_click_on_a_flat_row_asks_for_its_card_and_a_click_still_selects` ok.

## Click, double-click and scrolling behave as before
Status: PROVEN (headless widget events)
Same ui_gallery test (a click and two clicks still send SelectRoom once/twice, no card; a wheel turn over a row still gives RoomListScrolled). Full `cargo test -p twicecat-app --test ui_gallery`: 181 passed, 0 failed, 1 ignored (write_the_gallery). Existing flat-list tests pass unchanged (the diff only adds to tests/ui_gallery.rs, docs_sidebar.rs, fixture.rs).
Ran incl.: a_right_click_on_a_flat_row_asks_for_its_card_and_a_click_still_selects, a_right_click_on_a_rail_row_asks_for_no_card, the_flat_card_scenario_shows_only_the_tags_in_every_theme_and_density, no_two_things_in_a_room_list_row_overlap_and_none_reaches_the_scrollbar, ten_thousand_rooms_build_only_the_rows_in_view_for_every_density_and_the_rail, the_room_list_is_300_wide_or_a_72_px_rail_and_keeps_its_five_slots, the_chips_wrap_in_the_pane_and_no_chip_is_drawn_in_the_rail, ctrl_wheel_over_the_menus_of_the_settings_and_call_screens_changes_nothing (covers the new scenario), every_screen_lays_out_in_every_variant_at_both_sizes, the_fixture_reaches_every_screen_it_names (and the rest).

## Entries follow the flags; each sends one SetRoomTag, closes the card, saves no sidebar file
Status: PROVEN
the_flat_cards_entries_send_one_set_room_tag_close_it_and_save_no_file ... ok; the_flat_card_offers_only_the_tags ... ok; the Task 9 table test (the_tag_entries_show_the_wording_of_the_rooms_current_flags) reused and ok.

## Card for a room no longer in the list sends nothing
Status: PROVEN: a_flat_card_for_a_room_that_left_the_list_sends_nothing ... ok (fails under mutation 3).

## Favourites chip: "Remove from favourites" removes the tag and the room leaves on the next room-list event
Status: PROVEN at model level (command sent with tag None; list drops the room on the next event, in the unit tests). The round trip against the local .env.test account is NOT PROVEN here.

## Narrow rail has no card
Status: PROVEN: a_right_click_on_a_rail_row_asks_for_no_card ... ok; rail refusal in the_flat_card_does_not_open_where_the_list_is_sectioned_or_a_rail_or_for_a_stranger ... ok.

## Debug of Target::FlatRoom / OpenFlatRoom redacted
Status: PROVEN: the_flat_target_hides_the_room_id_in_debug ... ok.

## docs and wheel guard
`cargo test -p twicecat-app --test docs_sidebar`: 11 passed, incl. the_docs_say_the_flat_list_has_a_tag_card_and_the_narrow_rail_has_none, accessibility_says_the_tag_entries_need_a_pointer_and_add_no_shortcut, the_docs_do_not_promise_what_the_sidebar_does_not_do, sidebar_md_names_the_menu_entries_the_app_has (and 7 others).
`cargo test -p twicecat-app --test menus_ignore_the_wheel`: 5 passed (every_pick_list_is_wrapped_by_the_wheel_guard, every_slider_is_wrapped_by_the_wheel_guard, the_sidebar_cards_lists_are_pick_lists_behind_the_wheel_guard, the_sort_card_list_is_a_pick_list_behind_the_wheel_guard, the_scan_tells_a_wrapped_widget_from_a_bare_one). Unchanged file; the flat card holds no list.
Status: PROVEN

## No change to keys.rs, deny.toml, Cargo.lock, Cargo.toml, docs/plans
```console
$ git diff bde8625..ef7caab --stat -- crates/app/src/keys.rs deny.toml Cargo.lock Cargo.toml docs/plans
$
```
Status: PROVEN (empty output).

## Gallery PNGs (aid only, not proof)
`TWICECAT_GALLERY_DIR=target/ui-gallery TWICECAT_GALLERY_ONLY=room-list-card cargo test -p twicecat-app --test ui_gallery -- --ignored write_the_gallery`: 1 passed. Written (target/ui-gallery, untracked, not in git):
- room-list-card-daylight-comfortable.png 123865; -daylight-comfortable-rail.png 69364; -daylight-compact.png 111604; -daylight-compact-rail.png 69599
- room-list-card-twicecat-comfortable.png 121331; -twicecat-comfortable-rail.png 67665; -twicecat-compact.png 109282; -twicecat-compact-rail.png 67850
Read room-list-card-twicecat-comfortable.png: the dimmed Home screen (chips, search, Sort, rooms Lounge, Design review ...) with a centred card titled "Kitchen", a close "x", two full-width buttons "Remove from favourites" and "Mark as low priority", and a "Close" button at the bottom right. Nothing else offered.

## Mutations (each restored with git checkout)
1. tag_change: MarkLowPriority => Favourite. FAILED 2: the_flat_cards_entries_send_one_set_room_tag_close_it_and_save_no_file and the_favourite_and_low_priority_entries_send_one_set_room_tag_and_save_no_file (23 passed, 2 failed). The labels-only table test (the_tag_entries_show_the_wording_of_the_rooms_current_flags) did NOT fail, as expected: it checks wording, not commands.
2. flat_room_row returns the bare row (no mouse_area): ui_gallery a_right_click_on_a_flat_row_asks_for_its_card_and_a_click_still_selects FAILED (a_right_click_on_a_rail_row_asks_for_no_card still ok, as it should).
3. flat_room_entry's flat_room_drawn re-check disabled (`if false && ...`): a_flat_card_for_a_room_that_left_the_list_sends_nothing FAILED (assertion failed: tag_commands(&mut rx).is_empty()); 24 passed, 1 failed.
All three mutations were caught.

## PROGRESS.md
"Sidebar rework human checkpoint" has the new item "(Task 9b) The room card of the flat list in a real window": right-click in Home, under each chip other than All, and on a search result opens the card; centred and well placed; the two buttons read by the room's tags and nothing else; left-click, double-click and wheel unchanged; "Add to favourites" in Home puts the room under the Favourites chip and Element shows it as a favourite; under Favourites "Remove from favourites" makes it leave; offline failure shows the notice; Esc and Close close it; the narrow rail has no card (acceptable?); keyboard-only users cannot open it (A1, documented). The Task 9 item was reworded to point to it.
Open issues recorded in Step 9b: (a) no visible hint that a right-click does anything on a flat row; keyboard-only cannot tag; (b) both remove entries send tag None and the core removes m.favourite first, so "Remove low priority" on a room with both tags removes the favourite (needs a core API change). Both present.

## NOT PROVEN here (human checkpoint)
A right-click on a flat row in a real window (placement, platform pointer behaviour) and the Favourites chip round trip against the local .env.test account.

## Verdict
EVIDENCE: PARTIAL
