# Evidence — Task 8: Sort the room list
Commit: c637ec1 (diff base f4977bc)
Environment: headless, CC=clang-22 CXX=clang++-22, `cargo test -p twicecat-app`. No GUI run (no display).

## order() unit tests
Status: PROVEN (unit level)
`cargo test -p twicecat-app --lib -- order sorted sections room_list`: 216 passed, 0 failed. Names that ran in room_list::tests::order:
activity_with_no_flag_is_the_identity, name_ignores_case_and_then_goes_by_the_id, name_sorts_by_the_cleaned_name, unread_first_ranks_highlight_notification_unread_then_the_rest, unread_first_keeps_the_order_within_each_group (200 rooms, so > 20), low_last_puts_low_priority_rooms_at_the_end_keeping_their_order, it_works_over_references_too.

## Sections, flat mode, Alt+Down, save once, restart (sections::tests::sorted::*, all ok)
Status: PROVEN (unit level)
starred_and_custom_sections_keep_the_users_order_under_every_setting; channels_and_dms_in_name_order_are_sorted; the_default_is_the_lists_own_order; sorting_does_not_change_which_section_a_room_is_in; unread_first_puts_unread_rooms_first_inside_a_section_only; the_flat_list_is_shown_sorted_and_low_priority_last_with_the_all_chip; a_sorted_flat_list_asks_for_one_page_per_loaded_length_until_complete; unreads_first_in_the_flat_list_loads_everything_too; alt_down_from_the_last_room_of_a_group_goes_to_the_next_room_as_shown; alt_up_and_down_walk_the_sorted_flat_list_too; choosing_a_sort_saves_the_settings_once_per_change; a_restart_restores_the_sort; a_choice_with_no_card_open_changes_nothing; the_sort_card_opens_and_esc_or_close_shuts_it.
Also room_list::tests::load_more_is_sent_once_per_loaded_length ok.
Note: I did not separately inspect that the restart test uses a temp DataPaths; it passed by name only.

## menus_ignore_the_wheel
Status: PROVEN
`cargo test -p twicecat-app --test menus_ignore_the_wheel`: 5 passed (the_scan_tells_a_wrapped_widget_from_a_bare_one, the_sidebar_cards_lists_are_pick_lists_behind_the_wheel_guard, the_sort_card_list_is_a_pick_list_behind_the_wheel_guard, every_pick_list_is_wrapped_by_the_wheel_guard, every_slider_is_wrapped_by_the_wheel_guard).

## keys.rs / deny.toml / Cargo.lock / Cargo.toml untouched
Status: PROVEN
```console
$ git diff f4977bc..c637ec1 --stat -- crates/app/src/keys.rs deny.toml Cargo.lock Cargo.toml
(empty)
```

## Gallery (room-list-sort)
Status: PROVEN that it renders (aid only, not proof of look)
`cargo test -p twicecat-app --test ui_gallery sort`: the_sort_scenario_shows_the_card_over_a_to_z_with_unreads_first ... ok.
`TWICECAT_GALLERY_DIR=target/ui-gallery TWICECAT_GALLERY_ONLY=room-list-sort ... --ignored write_the_gallery`: ok. PNGs (target/ui-gallery, git-ignored):
```
127664 room-list-sort-daylight-comfortable.png
 73439 room-list-sort-daylight-comfortable-rail.png
116458 room-list-sort-daylight-compact.png
 73577 room-list-sort-daylight-compact-rail.png
125237 room-list-sort-twicecat-comfortable.png
 71831 room-list-sort-twicecat-comfortable-rail.png
114188 room-list-sort-twicecat-compact.png
 71802 room-list-sort-twicecat-compact-rail.png
```
Read room-list-sort-twicecat-comfortable.png: dark theme; a "Sort" button right of the search field; centred card "Sort rooms" with an x, an "Order" drop-down showing "A to Z", a ticked "Unreads first" box, the hint "Starred rooms and the sections you made keep your own order.", and a Close button. Behind the dimmed list: Design review (badge 2), Lounge (badge 3), Build alerts (dot), Stand-up (dot), then Alice Example, Bob Example, Book club, Garden club, Kitchen, Open house... i.e. unread rooms first, then A to Z.

## Mutations
1. `sort_by` -> `sort_unstable_by` in `order` (room_list.rs:215): `unread_first_keeps_the_order_within_each_group` FAILED (panic at room_list.rs:445, the other 6 passed). The test uses 200 rooms (>20). Restored with git checkout.
2. Removing `|| sorting == self.sorting()` in set_sorting (sidebar_menu.rs:788): `sections::tests::sorted::choosing_a_sort_saves_the_settings_once_per_change` FAILED (assertion at sections.rs:954; 13 others passed). Restored with git checkout.
Both mutations were caught.

## Human checkpoint
PROGRESS.md "Sidebar rework human checkpoint" has one unticked Task 8 item: "(Task 8) The Sort card and the sorted list in a real window" (every theme, both densities): Sort button beside search; card opens/closes (Close, x, Esc); Order shows Activity or A to Z and list changes at once; Unreads first moves mentions then unread to top keeping order in groups; Starred and own sections do not move; low priority comes last with All chip; A to Z survives restart; many rooms fill without jump; Alt+Up/Down and Ctrl+Shift+Up/Down follow shown order; Ctrl+wheel over the card's list does not change the sort.

## NOT PROVEN here
Look and feel of the card and sorted list in a real window, and "choosing A to Z orders the list by name and survives a restart" in a running app against the local .env.test account. Both are on the human checkpoint.
