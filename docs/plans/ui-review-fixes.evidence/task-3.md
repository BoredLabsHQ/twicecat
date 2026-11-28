# Evidence — Task 3: a room panel on the right
Commit: e8de685
Environment: CC=clang-22 CXX=clang++-22; `cargo test -p twicecat-app` (lib) and the gallery test with TWICECAT_GALLERY_ONLY=main-thread. No real window (human-only).

## Panel view state, five slots, thread opens the panel on the Thread tab
Status: PROVEN (tests + gallery)
```console
$ cargo test -p twicecat-app --lib spaces_and_threads
test state::tests::spaces_and_threads::opening_a_thread_opens_the_room_panel_on_the_thread_tab ... ok
test state::tests::spaces_and_threads::the_panes_row_keeps_five_slots_and_the_room_pane_its_tree ... ok
test state::tests::spaces_and_threads::the_participants_panel_takes_its_width_from_the_room_pane ... ok
test state::tests::spaces_and_threads::closing_the_room_panel_closes_the_thread_too ... ok
$ cargo test -p twicecat-app --lib   (whole lib)
test result: ok. 1867 passed; 0 failed
```
Gallery `TWICECAT_GALLERY_ONLY=main-thread` wrote 8 main-thread-* PNGs (test ok). Screenshot: task-3-main-thread.png shows the right-hand panel at the old thread slot position with the thread content.

## Header: tab strip (Members, Info, Thread only while a thread is open) + close button; tab switch keeps thread
Status: PROVEN (screenshot + test)
```console
test state::tests::spaces_and_threads::switching_tabs_keeps_the_open_thread ... ok
```
Screenshot header shows "Members | Info | Thread" tabs (Thread selected) and an x close button. The "Thread tab absent without a thread" case was not separately observed in a screenshot.

## layout_for(room_panel_open), last opened wins with call panel, Esc closes, no panel without a room
Status: PROVEN (tests)
```console
$ cargo test -p twicecat-app --lib layout::
test layout::tests::a_width_that_is_not_a_number_is_treated_as_the_narrowest ... ok
test layout::tests::medium_windows_force_the_rail_only_while_a_panel_is_open ... ok
test layout::tests::narrow_windows_drop_the_sidebar_use_the_rail_and_give_a_panel_the_room_pane ... ok
test layout::tests::wide_windows_apply_the_choice_and_show_every_open_panel ... ok
test layout::tests::with_both_open_the_one_opened_last_is_shown_and_the_other_waits ... ok
test result: ok. 12 passed (layout:: filter incl. sidebar_layout state tests)
$ cargo test -p twicecat-app --lib   (relevant, all ok)
test state::tests::spaces_and_threads::esc_closes_the_room_panel_on_any_tab ... ok
test state::tests::spaces_and_threads::the_room_panel_cannot_be_open_without_a_room ... ok
test state::tests::narrow_window::esc_closes_the_shown_panel_first_and_returns_to_the_room_pane ... ok
test state::tests::narrow_window::esc_closes_the_thread_first_when_it_is_the_one_shown ... ok
test state::tests::keyboard::esc_closes_the_thread_panel ... ok
```

## Verdict
EVIDENCE: PROVEN
