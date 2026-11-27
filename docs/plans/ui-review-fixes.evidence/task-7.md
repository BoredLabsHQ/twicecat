# Evidence — Task 7: room-list header in one row
Commit: e2617f1
Environment: CC=clang-22 CXX=clang++-22, cargo test (headless renderer); gallery written to a scratchpad dir with
`TWICECAT_GALLERY_DIR=<scratch> TWICECAT_GALLERY_ONLY=main-room cargo test -p twicecat-app --test ui_gallery -- --ignored write_the_gallery`
(test ok). Screenshot: docs/plans/ui-review-fixes.evidence/task-7-main-room-300px.png (1280x800, list at 300 px).

## One header row: filter drop-down ("All" first), search field with clear button, Sort, "+"
Status: PROVEN (layout by PNG and test; clear button only visible with text, covered by test)
```console
$ cargo test -p twicecat-app -- the_expanded_header_is_one_row_and_the_list_starts_right_below_it
test the_expanded_header_is_one_row_and_the_list_starts_right_below_it ... ok
```
PNG: one row at the top of the list pane reading `[All v] [Search rooms] [Sort] [+]`.
Search field width at 300 px: about 102 px (x ~188 to ~290 of the window; the pane is x 73-373).
The placeholder "Search rooms" fits fully on one line without clipping, so it is usable for
short queries (roughly 12-14 characters visible once typing, and the clear slot adds more
squeeze when text is present). It is narrow but not unusable; the dropdown (~103 px) is as wide as the field.
Not seen in a real window (human-only).

## Picking a filter sets the tab; re-picking the active one keeps it
Status: PROVEN (unit test)
```console
$ cargo test -p twicecat-app -- choosing_a_tab_from_the_drop_down_sets_it_and_never_toggles
test state::tests::choosing_a_tab_from_the_drop_down_sets_it_and_never_toggles ... ok
```

## "+" opens a card with New room / Direct message / Join a room; closes on a choice and Esc
Status: PROVEN (unit test; the card itself is not in the gallery PNG)
```console
$ cargo test -p twicecat-app -- the_add_card_opens_closes_with_esc_covers_the_bar_and_closes_after_each_choice
test state::sidebar_menu_flow::the_add_card_opens_closes_with_esc_covers_the_bar_and_closes_after_each_choice ... ok
```

## Drop-down is a kit::menu pick_list behind the wheel guard
Status: PROVEN
```console
$ cargo test -p twicecat-app --test menus_ignore_the_wheel
test the_sidebar_cards_lists_are_pick_lists_behind_the_wheel_guard ... ok
test the_sort_card_list_is_a_pick_list_behind_the_wheel_guard ... ok
test every_pick_list_is_wrapped_by_the_wheel_guard ... ok
test every_slider_is_wrapped_by_the_wheel_guard ... ok
test the_scan_tells_a_wrapped_widget_from_a_bare_one ... ok
test result: ok. 5 passed; 0 failed
```

## Pane column keeps five slots; list top is about one control height plus gaps below pane top; rail unchanged
Status: PROVEN
```console
$ cargo test -p twicecat-app -- the_expanded_header_is_one_row_and_the_list_starts_right_below_it
test ... ok
```
The test asserts column.children().len()==5, empty chips/search slots (height < 0.5), header at y=0, and list top <= header height + 3*gap
(for expanded and rail variants). PNG: list's first row starts ~30 px below the header (y ~110 vs header y ~56-78), instead of three rows.
Rail (left strip x 0-72) is visually unchanged in the PNG; rail variants are checked by the same test.

## Verdict
EVIDENCE: PROVEN
