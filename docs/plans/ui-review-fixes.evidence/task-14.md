# Evidence — Task 14: cap the timeline's width at 880 px, left-aligned
Commit: 635b5b0 (fixup of c7e96f9)
Environment: CC=clang-22 CXX=clang++-22; cargo tests, plus a full `write_the_gallery` run (-- --ignored, 71.8 s, TWICECAT_GALLERY_DIR in the scratchpad). No Docker, no real window (human-only).

## Rows sit in a column capped at 880 px, left-aligned; action bar, reactions, "Seen by" inside the column
Status: PROVEN (layout test + rendered image; reactions/Seen by by test only)
```console
$ cargo test -p twicecat-app --test timeline_layout
test the_timeline_column_is_capped_and_its_rows_stay_inside_it ... ok
(10 passed; 0 failed)
```
The test lays out 1920 px with a hovered row (action bar), asserts column width <= 880, no descendant's right edge past the column, and x == 0. The fixture's visible rows in the image do not include reactions or "Seen by", so those are proven by the test only.

Image: docs/plans/ui-review-fixes.evidence/task-14-main-room-wide.png (main-room-wide, 1920x800, twicecat/comfortable). Viewed: the timeline rows (avatars at x~415, text from x~446, the file/audio cards, the "New messages" rule ending at x~1246) sit in a column hugging the left of the room pane, from x~398 to about 1246. Right of that, to the scrollbar at x~1905, is empty. The header (title left, Notifications/Members/Info/Video/Call at the far right edge) and the composer (x 382 to 1912, attach icon left, emoji and send at the right end) span the whole room pane. Normal width (main-room-twicecat-comfortable, 1280x800): same left edge and the same "New messages" rule end at x~1246, so rows are unchanged below the cap.

## Composer, typing line and bars stay full width
Status: PROVEN
```console
$ cargo test -p twicecat-app --test ui_gallery the_composer_spans_the_room_pane_in_a_wide_window
test the_composer_spans_the_room_pane_in_a_wide_window ... ok
```
Asserts at 1920 px the editor width > 880 and equals the pane column width, as does the header. The image shows the same. The typing line was not separately visible; it is not asserted by this test.

## estimate_height uses min(pane width, cap); a 1920 px estimate case is within tolerance
Status: PROVEN
```console
$ cargo test -p twicecat-app --test timeline_layout
test estimates_at_1920_px_stay_within_a_quarter_of_the_capped_rows ... ok
test estimates_stay_within_a_quarter_of_the_laid_out_rows_for_both_densities ... ok
test decorated_rows_are_estimated_within_a_quarter_in_both_densities ... ok
```

## anchor_bottom / offset_from_bottom unchanged
Status: PROVEN (diff inspection + tests)
The diff of timeline_pane.rs only adds `.max_width(TIMELINE_MAX_WIDTH)` to the column; `.anchor_bottom()` is untouched, and timeline.rs does not touch offset_from_bottom. The 10 timeline_layout tests pass.

## A resize above the cap keeps measured heights
Status: PROVEN
```console
$ cargo test -p twicecat-app --lib a_resize_above_the_column_cap
test timeline::tests::a_resize_above_the_column_cap_keeps_the_measurements ... ok
```
The test checks that 1920 -> 1900 keeps the measurement and that 1920 -> 800 and then 800 -> 700 clear it.

## Verdict
EVIDENCE: PROVEN
