# Evidence — Task 4: Members tab in the room panel
Commit: 65c53fd
Environment: headless iced renderer via the repo's test harness (CC=clang-22 CXX=clang++-22); no server needed. Real window is human-only.

Commands run (all passed):
```console
$ cargo test -p twicecat-app members
test state::tests::rooms_ui::the_members_button_opens_the_panel_switches_to_members_and_closes_it ... ok
test state::tests::rooms_ui::the_members_tab_is_not_an_overlay_and_esc_closes_the_panel ... ok
test state::tests::rooms_ui::leaving_the_members_tab_forgets_its_state_and_coming_back_asks_again ... ok
test state::tests::rooms_ui::switching_room_never_shows_the_old_rooms_members ... ok
test state::tests::a_members_answer_for_another_room_is_ignored ... ok
test screens::members::tests::a_name_that_reads_like_a_tag_is_quoted_and_another_is_not ... ok
test dialogs::members_are_in_sections_with_counts_in_the_order_admins_moderators_members_invited ... ok
test dialogs::the_members_tab_stays_in_the_window_and_keeps_invite_in_view ... ok
test result: ok. 22 passed; 0 failed   (lib; 2 passed in ui_gallery; list trimmed to the relevant lines)
$ TWICECAT_GALLERY_DIR=<scratch>/g TWICECAT_GALLERY_ONLY=members cargo test -p twicecat-app --test ui_gallery -- --ignored write_the_gallery
test write_the_gallery ... ok
```
Also `cargo test -p twicecat-app --test ui_gallery` (full, 192 passed, 1 ignored). Gallery produced members, members-call-trust, members-search, members-none, members-loading, members-many x 2 themes x 2 densities (+rail).

## The member list is the Members tab of the room panel (not a modal), keeping search, sections, trust and "In the call" tags, look-alike quoting, member_page windowing; Invite still opens the invite dialog as a modal
Status: PROVEN (rendering via headless gallery; windowing/quoting via tests)
Screenshot task-4-members-tab.png (members-call-trust): the right-hand room panel has Members/Info tabs, "Members (8)", a search field, sections Admins (2), Moderators (1), Members (4), Invited (1); tags Creator, Verified, Not verified, Identity changed, "In the call"; look-alike names shown quoted ("Admin (75)", "Verified") with localparts; Invite field at the panel foot. The tests the_members_tab_stays_in_the_window_and_keeps_invite_in_view, a_name_that_reads_like_a_tag_is_quoted..., members_are_in_sections_with_counts... pass. Invite-opens-modal is not shown by a separate test I ran by name; the invite dialog path is unchanged in the diff (not independently exercised).

## HeaderAction::Members opens the panel on Members, or closes it when already shown
Status: PROVEN
the_members_button_opens_the_panel_switches_to_members_and_closes_it ... ok

## Members modal gone from the room-dialog layer; Esc closes the panel
Status: PROVEN
the_members_tab_is_not_an_overlay_and_esc_closes_the_panel ... ok; diff removes the Members arm from screens/room_dialogs.rs (-6 lines).

## Switching room never shows the old room's members
Status: PROVEN
switching_room_never_shows_the_old_rooms_members ... ok; a_members_answer_for_another_room_is_ignored ... ok; leaving_the_members_tab_forgets_its_state_and_coming_back_asks_again ... ok

## Real window (not exercised)
Status: NOT PROVEN (human-only checkpoint)
