# Evidence — Task 16: raise the smallest type sizes
Commit: 36c91d1
Environment: `CC=clang-22 CXX=clang++-22 cargo test -p twicecat-app ...` on the existing target dir; gallery written with `TWICECAT_GALLERY_DIR=<scratchpad>/after TWICECAT_GALLERY_ONLY=main-room ... --test ui_gallery -- --ignored`. No Docker, no window.

## tokens::CAPTION is 13 (was 12); nothing else in the type scale changes
Status: PROVEN
```console
$ cargo test -p twicecat-app --lib -- tokens::tests icons room_rows_fit
test ui::tokens::tests::the_scales_are_the_documented_ones ... ok
test ui::tokens::tests::density_sets_the_row_heights_and_trims_only_the_vertical_padding ... ok
test result: ok. 9 passed; 0 failed
```
The scale test (updated in the diff to CAPTION = 13) passes; the diff touches only CAPTION and ICON_SIZE in tokens.rs.

## Icon glyphs never smaller than before (ICON_SIZE = max(BODY,16) = 16; deviation from "to BODY" recorded for the human)
Status: PROVEN
```console
test ui::icons::tests::icon_glyphs_are_never_below_body_size_and_never_smaller_than_before ... ok
test ui::icons::tests::the_old_marks_keep_their_glyphs ... ok
```

## Text-scale setting still multiplies everything; room rows fit their preview line in both densities at 100% and 200%
Status: PROVEN
```console
$ cargo test -p twicecat-app --test ui_gallery room_rows_fit
test room_rows_fit_their_text_in_both_densities_at_100_and_200_percent_text_scale ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 206 filtered out
```

## The 10,000-room test passes
Status: PROVEN
```console
$ cargo test -p twicecat-app --lib ten_thousand
test room_list::pure_tests::ten_thousand_rooms_build_only_the_visible_rows_in_every_density_and_the_rail ... ok
```

## estimate_height within +-25% (tests/timeline_layout.rs)
Status: PROVEN
```console
$ cargo test -p twicecat-app --test timeline_layout
test decorated_rows_are_estimated_within_a_quarter_in_both_densities ... ok
test estimates_at_1920_px_stay_within_a_quarter_of_the_capped_rows ... ok
test estimates_stay_within_a_quarter_of_the_laid_out_rows_for_both_densities ... ok
test only_the_visible_rows_of_10000_items_are_laid_out ... ok
test result: ok. 11 passed; 0 failed
```

## Nothing clips (full gallery layout tests pass)
Status: PROVEN
```console
$ cargo test -p twicecat-app --test ui_gallery
test result: ok. 206 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 76.03s
```
The ignored test is the PNG writer, run separately below.

## Human before/after comparison
Status: NOT PROVEN (before images skipped)
`df` showed 7.3 GB free on `/` (100% used), below the 30 GB threshold for a second build, so no worktree was made at 439ecde and there are no "before" images. Only the "after" image exists: `docs/plans/ui-review-fixes.evidence/task-16-after.png` (main-room-twicecat-comfortable at 36c91d1; the other variants are in the scratchpad `after/` dir).

What the after image shows (viewed): the room list previews, timestamps (03:59, Yesterday), the unread badges and the tags (Muted, Voice, Invite) are all whole and unclipped. Preview lines truncate at the row edge, as they did before. Timeline captions (file size lines, "Sending...", "Failed to send...") and the header icons (gear, members, thread) are whole. No overlap or cut-off glyphs were seen. A size difference against the old build cannot be judged without a before image.

## Verdict
All test-checkable criteria pass. The before/after visual comparison is left for the human, who can run the gallery at 439ecde.

## Observations from the diff (not in the criteria)
- `crates/app/src/screens/sessions.rs`: the content column spacing changed from `tokens::L` to `tokens::M`. The criteria do not mention this. It is presumably there to keep the sessions screen from clipping.
- `crates/app/src/ui/kit.rs`: icon buttons drew their glyph at `CAPTION` (12 before), and now draw at `ICON_SIZE` (16). Those icons grow by 4 px, not just "never shrink".
