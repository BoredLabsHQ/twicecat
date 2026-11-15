# Evidence — Task 10: dragging a room onto Favourites or Low priority
Commit: fb46155 (diff base 1deffaf)
Environment: headless, `CC=clang-22 CXX=clang++-22`, cargo test only; no GUI, no homeserver. Gallery PNGs in target/ui-gallery (git-ignored, not committed).

## Table test: each tag drop gives the same command as the matching Task 9 menu entry
Status: PROVEN
```console
$ cargo test -p twicecat-app --lib list_drop      # 13 passed, 0 failed
list_drop::tests::every_tag_drop_gives_the_command_of_the_matching_menu_entry ... ok
$ cargo test -p twicecat-app --lib dragging       # 37 passed (sections::tests::dragging + state::tests::rail_dragging)
sections::tests::dragging::every_tag_drop_sends_the_command_of_the_matching_card_entry ... ok
sections::tests::dragging::every_kind_of_drop_gives_the_layout_of_the_matching_card_operation ... ok
```
Both the verdict-level and the state-level (sent command == card entry's command) tables pass. Mutation 1 below shows they bite.

## Already in Favourites dropped on Favourites: nothing, no ring, no refusal mark
Status: PROVEN (headless, by tests)
```console
list_drop::tests::a_room_already_in_the_tag_section_dropped_on_it_does_nothing ... ok
sections::tests::dragging::a_room_already_in_favourites_dropped_on_favourites_does_nothing ... ok
```

## Starred room on Low priority: refused, drawn as not allowed
Status: PROVEN (state; drawing in a real window is NOT PROVEN)
```console
list_drop::tests::a_starred_room_is_refused_on_low_priority ... ok
sections::tests::dragging::a_starred_room_on_low_priority_is_refused_and_sends_nothing ... ok
```

## Tagged room dropped on its default section removes the tag
Status: PROVEN
```console
list_drop::tests::a_tagged_room_dropped_on_its_default_section_loses_its_tag_and_an_untagged_one_stays ... ok
sections::tests::dragging::a_tag_drop_keeps_a_target_ring_and_a_default_drop_of_an_untagged_room_stays_nothing ... ok
```

## Existing list_drop and space-sections-dragging* tests pass unchanged
Status: PROVEN
```console
$ cargo test -p twicecat-app --test ui_gallery dragging
rail_reorder_drag::the_cursor_is_grab_when_idle_and_grabbing_while_dragging ... ok
rail_reorder_drag::dragging_a_space_of_an_open_folder_out_of_it_moves_it_inside_at_most ... ok
test result: ok. 2 passed; 180 filtered out
$ cargo test -p twicecat-app --test ui_gallery     # full file
test result: ok. 181 passed; 0 failed; 1 ignored
```
The scenarios space-sections-dragging, -line, -refused, -header and -tag are not separate test names: they are entries in the required-names list (ui_gallery.rs lines 417-421, 1696-1700) checked by the layout/overlap and "fixture reaches every screen" tests, which are in the 181 that pass. The diff to ui_gallery.rs is +2 lines (the new name only). Existing tests in list_drop.rs: the diff is additive in tests (13 tests all pass).

## Exactly one SetRoomTag per drop and no sidebar file written
Status: PROVEN (by tests)
The state-level table test asserts exactly the one command and no store write; mutation 3 (send twice) fails it.

## Docs test
```console
$ cargo test -p twicecat-app --test docs_sidebar   # 12 passed
the_docs_say_a_room_can_be_dragged_onto_favourites_or_low_priority_like_the_menu_entries ... ok
accessibility_says_the_tag_entries_need_a_pointer_and_add_no_shortcut ... ok
the_keyboard_map_of_accessibility_gets_no_sidebar_row ... ok
accessibility_names_the_options_menus_and_says_they_need_a_pointer ... ok
sidebar_md_* (8 tests), the_docs_say_the_flat_list_has_a_tag_card_and_the_narrow_rail_has_none, the_docs_do_not_promise_what_the_sidebar_does_not_do ... ok
```

## Untouched files
Status: PROVEN
```console
$ git diff 1deffaf..fb46155 --stat -- crates/app/src/keys.rs deny.toml Cargo.lock Cargo.toml docs/plans
(empty)
```

## Gallery render (aid only)
```console
$ TWICECAT_GALLERY_DIR=target/ui-gallery TWICECAT_GALLERY_ONLY=space-sections-dragging-tag cargo test -p twicecat-app --test ui_gallery -- --ignored write_the_gallery
test write_the_gallery ... ok
space-sections-dragging-tag-{daylight,twicecat}-{comfortable,compact}.png  (116-139 KB each)
space-sections-dragging-tag-{daylight,twicecat}-{comfortable,compact}-rail.png  (68-70 KB each), index.html
```
Read space-sections-dragging-tag-twicecat-comfortable.png (1280x800): the Favourites header (Random, Async questions below it) has a light-purple outline and a lighter fill, the target ring; Starred, Channels, Tech and Hobbies headers are plain. No pointer is drawn. Aid only, not proof.

## Mutations (each restored with git checkout; tree clean afterwards)
1. `SectionKey::LowPriority` drop mapped to `Entry::AddFavourite` in list_drop.rs: FAILED as wanted: `list_drop::tests::every_tag_drop_gives_the_command_of_the_matching_menu_entry` (lib list_drop: 12 passed 1 failed) and `sections::tests::dragging::every_tag_drop_sends_the_command_of_the_matching_card_entry`.
2. "already in Favourites" arm returns `tag_drop(room, AddFavourite)` instead of Nothing: FAILED as wanted: `list_drop::tests::a_room_already_in_the_tag_section_dropped_on_it_does_nothing` and `sections::tests::dragging::a_room_already_in_favourites_dropped_on_favourites_does_nothing`.
3. spaces.rs `Verdict::Tag` arm sends `SetRoomTag` twice (the arm already returns early, so falling through to the sidebar save cannot be constructed without restructuring; double send is the realistic variant): FAILED as wanted: `every_tag_drop_sends_the_command_of_the_matching_card_entry` and `a_tag_drop_keeps_a_target_ring_and_...`. Not constructed: the fall-through to a sidebar save variant.

## PROGRESS.md
Checkpoint "Sidebar rework human checkpoint" has one unchecked Task 10 item: "(Task 10) Dragging a room onto Favourites or Low priority in a real window": ring over Favourites/Low priority headers like a custom section; drop tags the room and it moves a moment later; drag a room onto Low priority on the local account and see it low priority in Element (same for Favourites); already-in-Favourites shows no ring and no refusal mark; Starred over Low priority shows red outline and "Not allowed"; a tagged room dropped on its default header loses the tag; ring and refusal readable in every theme; a favourite dragged into a custom section stays under Favourites: acceptable?
Step 10 entry records the open issues: both-tags core issue (remove sends None, core removes m.favourite first); a favourite dragged into a custom section stays drawn in Favourites; a low-priority-only room dropped back on its default section takes RoomToDefault, keeps its tag and is drawn in Low priority again; the only member of its default section leaves the header hidden (no drop target). All four confirmed.

## NOT PROVEN here (human checkpoint)
Drag feel, the ring and the refusal mark in a real window, and "dragging a room onto Low priority tags it and Element shows it" against the local .env.test account. No display and no homeserver were used.

## Verdict
EVIDENCE: PARTIAL
