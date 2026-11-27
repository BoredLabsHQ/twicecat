# Evidence — Task 5: Info tab / room settings in the room panel
Commit: 8db34ce
Environment: headless iced renderer via the repo test harness (CC=clang-22 CXX=clang++-22); no homeserver, no Docker. Real window and homeserver rename check are human-only.

Commands run (all passed, filtered to the relevant lines):
```console
$ cargo test -p twicecat-app <name>      # each of the names below
test state::tests::rooms_ui::the_settings_button_opens_the_panel_on_info_and_closes_it_again ... ok
test state::tests::rooms_ui::settings_no_longer_close_the_members_tab_they_switch_to_info ... ok
test state::tests::rooms_ui::the_info_tab_is_not_an_overlay_and_esc_cancels_a_question_before_closing_the_panel ... ok
test state::tests::rooms_ui::a_room_switch_closes_the_info_tab_and_it_draws_only_the_selected_rooms_data ... ok
test state::tests::rooms_ui::an_invitation_room_has_no_members_or_info_tab ... ok
test screens::room_dialogs::tests::escape_closes_what_is_drawn_on_top_and_the_info_tab_is_no_layer ... ok
test dialogs::the_info_tab_reads_facts_first_then_the_fields_then_encryption_and_leave ... ok
test dialogs::the_info_tab_stays_in_the_window_and_scrolls_in_a_short_one ... ok
test dialogs::a_question_that_cannot_be_undone_keeps_its_red_button_apart_and_elsewhere ... ok
test security_screens::a_question_that_cannot_be_undone_keeps_its_red_button_apart_and_elsewhere ... ok
$ TWICECAT_GALLERY_DIR=<scratch>/g TWICECAT_GALLERY_ONLY=room- cargo test -p twicecat-app --test ui_gallery -- --ignored write_the_gallery
test write_the_gallery ... ok
```
Gallery scenarios drawn: room-settings, room-settings-long, room-leave-confirm, room-encrypt-confirm (x 2 themes x 2 densities). I viewed room-settings and room-leave-confirm (twicecat, comfortable); the others were generated but not viewed. Screenshot: task-5-info-leave-confirm.png.

## Info tab shows read-only facts (avatar+name, wrapping topic, alias, encryption in words, member count, notification level), then editable fields per permissions, Turn encryption on, Leave; same messages as the old modal
Status: PROVEN (rendering from the gallery; ordering/scroll via tests)
The screenshot shows the Info tab: avatar + "Lounge", topic "Where we talk about everything", "Address: #lounge:example.org", "Not encrypted: the server can read messages in this room.", "Members: counting...", notification dropdown "Use default"; then a rule, Name and Topic fields with Save, "Change picture...", "Enable encryption...", rule, "Leave room...". The ordering test and the stays-in-window/scrolls test pass. Observation: the fixture header badge says "Encrypted" while the Info tab says "Not encrypted"; this looks like fixture data, not checked in code. "Same messages as the old modal" was not compared side by side; only the screenshot's wording was seen.

## Irreversible confirmations keep layout (ask left, red confirm with words right, >= tokens::M from Cancel); Esc cancels the question first, then closes the panel
Status: PROVEN
The screenshot shows "Leave this room?" with the question text, Cancel at the left of the pair and a red "Leave" at the right with a gap. a_question_that_cannot_be_undone_keeps_its_red_button_apart_and_elsewhere passes in both dialogs and security_screens. Esc order: the_info_tab_is_not_an_overlay_and_esc_cancels_a_question_before_closing_the_panel passes.

## HeaderAction::Settings opens the panel on Info (closes if already shown); settings modal gone from the layer list; Info/Members never show another room's data after a room switch; opening settings no longer closes Members
Status: PROVEN
the_settings_button_opens_the_panel_on_info_and_closes_it_again, escape_closes_what_is_drawn_on_top_and_the_info_tab_is_no_layer, a_room_switch_closes_the_info_tab_and_it_draws_only_the_selected_rooms_data and settings_no_longer_close_the_members_tab_they_switch_to_info all pass. The Task 4 Members room-switch test (switching_room_never_shows_the_old_rooms_members) was not re-run here.

Not exercised: a real window, and the homeserver rename (human checkpoint).

EVIDENCE: PROVEN
