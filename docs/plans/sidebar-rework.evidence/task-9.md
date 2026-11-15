# Evidence — Task 9: Favourites and Low priority sections, and their menu entries
Commit: dbaefb8 (Task 9 = 0faf521 + dbaefb8; base d7e1bde). Headless only; `CC=clang-22 CXX=clang++-22`. Tree clean before and after (mutations restored with `git checkout`).

## Placement tests (made-up IDs)
Status: PROVEN
`cargo test -p twicecat-app --lib` (full: 1781 passed, 0 failed). Relevant tests, all ok:
- sections::tests::tags::every_room_appears_once
- sections::tests::tags::a_room_goes_to_the_first_section_that_applies
- sections::tests::tags::the_default_order_is_starred_favourites_channels_sub_spaces_custom_direct_low_priority
- sections::tests::tags::both_sections_are_hidden_while_empty
- sections::tests::tags::a_stored_order_holding_favourites_is_kept
- sections::tests::tags::the_new_sections_follow_the_chosen_sort
- sections::tests::tags::a_collapsed_new_section_keeps_its_header_with_the_badge
- sections::tests::a_level_1_space_in_two_sub_spaces_is_placed_once, the_default_order_and_the_default_places, the_stored_order_is_kept_and_missing_keys_take_their_default_place

## Menu: one SetRoomTag, wording by flags, no sidebar file
Status: PROVEN
- sections::tests::menu::the_favourite_and_low_priority_entries_send_one_set_room_tag_and_save_no_file ... ok
- sections::tests::menu::the_tag_entries_show_the_wording_of_the_rooms_current_flags ... ok (table test)
- sections::tests::menu::a_tag_entry_on_a_card_that_is_gone_sends_nothing ... ok
- sections::tests::menu::a_tagged_room_moves_into_its_section_when_the_core_reports_the_tag ... ok
- sidebar_menu::tests::every_button_of_every_section_and_room_card_maps_to_an_operation_or_the_dialog ... ok

## Collapse and Move up/down on the two new headers
Status: PROVEN
- sections::tests::menu::collapse_and_move_on_the_new_headers_save_like_the_other_built_in_sections ... ok

## docs_sidebar
Status: PROVEN
```console
$ cargo test -p twicecat-app --test docs_sidebar
test the_keyboard_map_of_accessibility_gets_no_sidebar_row ... ok
test accessibility_names_the_options_menus_and_says_they_need_a_pointer ... ok
test accessibility_says_the_tag_entries_need_a_pointer_and_add_no_shortcut ... ok
test sidebar_md_says_an_older_twicecat_drops_the_two_section_keys_it_does_not_know ... ok
test sidebar_md_says_favourites_and_low_priority_are_kept_on_the_homeserver ... ok
test sidebar_md_names_the_menu_entries_the_app_has ... ok
test the_docs_do_not_promise_what_the_sidebar_does_not_do ... ok
test sidebar_md_states_the_caps_the_core_enforces ... ok
test sidebar_md_says_where_the_layout_is_and_for_whom ... ok
test sidebar_md_claims_no_sync_and_no_removal_at_logout ... ok
test result: ok. 10 passed; 0 failed
```

## Notice on ActionFailed { RoomTag } names no room and no error text
Status: PROVEN
- state::tests::a_failed_room_tag_notice_never_carries_an_error_text ... ok
- actions::tests::only_redact_react_and_room_tag_failures_get_a_notice ... ok

## keys.rs / deny.toml / Cargo files untouched
Status: PROVEN
```console
$ git diff d7e1bde..dbaefb8 --stat -- crates/app/src/keys.rs deny.toml Cargo.lock Cargo.toml
(empty)
```

## Gallery
Status: PROVEN (render builds; look not judged)
`cargo test -p twicecat-app --test ui_gallery` (full): 178 passed, 1 ignored; includes `space_sections_draw_every_line_whole_and_apart_in_both_densities_and_themes`. Note: the filter `space_sections_tags` matches 0 tests (0 run, 179 filtered); the scene is exercised by that test, and the PNG run is:
```console
$ TWICECAT_GALLERY_DIR=target/ui-gallery TWICECAT_GALLERY_ONLY=space-sections-tags cargo test -p twicecat-app --test ui_gallery -- --ignored write_the_gallery
test write_the_gallery ... ok
-rw-rw-r-- 138097 space-sections-tags-daylight-comfortable.png
-rw-rw-r--  69368 space-sections-tags-daylight-comfortable-rail.png
-rw-rw-r-- 116212 space-sections-tags-daylight-compact.png
-rw-rw-r--  69582 space-sections-tags-daylight-compact-rail.png
-rw-rw-r-- 136195 space-sections-tags-twicecat-comfortable.png
-rw-rw-r--  67992 space-sections-tags-twicecat-comfortable-rail.png
-rw-rw-r-- 114596 space-sections-tags-twicecat-compact.png
-rw-rw-r--  68136 space-sections-tags-twicecat-compact-rail.png
(+ index.html, 2241)
```
`target/ui-gallery` is git-ignored. Read space-sections-tags-twicecat-comfortable.png (aid only): dark theme; sidebar shows chips, then headers in order Starred ("personal" tag, Roadmap), Favourites (Random, Async questions), Channels (General selected, 3 unread), Tech (Rust help, DevOps), Hobbies (Board games). All sections have a chevron and a "..." button. The collapsed Low priority header is below the visible area of this image, so I did not see it there.

## Mutations
1. Low priority placed before custom in sections.rs: `a_room_goes_to_the_first_section_that_applies ... FAILED` (panicked at sections.rs:869, "a custom section beats Low priority"); others in `tags` passed (6 passed, 1 failed). FAILS as wanted.
2. RoomTag failure notice uses `error.to_string()` in state.rs:2019: `a_failed_room_tag_notice_never_carries_an_error_text ... FAILED`, message shows "Could not update the favourite or low priority tag: unknown room !secretroom:x". FAILS as wanted.
3. `Entry::MarkLowPriority` sends `Favourite` (sidebar_menu.rs:514): `the_favourite_and_low_priority_entries_send_one_set_room_tag_and_save_no_file ... FAILED` (sections.rs:2660); 16 passed, 1 failed. FAILS as wanted. (My first sed attempt broke the build; redone correctly.)
All files restored; `git status --short` empty.

## PROGRESS.md
"Sidebar rework human checkpoint" has the Task 9 item (line 4346): sections show only while holding rooms and in order Starred, Favourites, Channels, sub-spaces, own sections, Direct messages, Low priority; card wording by tags; "Add to favourites" moves the room and Element on the same test account shows it as a favourite; "Mark as low priority" in Element puts a room in Low priority; a failure (offline) shows the notice; collapse, expand, Move up/Down on both headers work and survive a restart; the flat list has no room card (acceptable?); drag onto these headers refused until Task 10.
The open issue is recorded: "The flat list has no room card" (conflict with L5 "every room (flat and sectioned)"), plus "The flat list and Home have no room card, so a room there cannot be tagged from the app".

## NOT PROVEN here (human checkpoint)
- Look and feel of the sections in a real window (themes, densities, motion, pointer).
- "Add to favourites moves the room into Favourites and Element on the same test account shows it as a favourite" against the local .env.test account.
- Low priority section as drawn (collapsed) not seen in the image I read.

## Verdict
EVIDENCE: PARTIAL (headless criteria all proven and all 3 mutations fail a test; real-window look/feel and the live Element round trip on the .env.test account are unproven and on the human checkpoint)
