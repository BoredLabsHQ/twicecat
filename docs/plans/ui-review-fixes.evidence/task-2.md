# Evidence — Task 2: top bar for the whole app
Commit: 5e6b325
Environment: CC=clang-22 CXX=clang++-22, cargo test -p twicecat-app (headless tiny-skia gallery, no real window). Screenshots kept: task-2-main-call-sharing-1280.png (twicecat theme, comfortable, 1280x800, while sharing) and task-2-main-narrow-720.png (720x800). Other variants in the scratchpad, not kept. Note: TWICECAT_GALLERY_ONLY skips the narrow cases, so the 720 image came from an unfiltered gallery run.

## One-row top bar: left app name + "Search rooms (Ctrl+K)"; middle sharing bar while sharing; right sync dot with words, labelled Settings button, account button.
Status: PROVEN (real window rendering is human-only)
```console
$ TWICECAT_GALLERY_DIR=$S/g TWICECAT_GALLERY_ONLY=main- cargo test -p twicecat-app --test ui_gallery write_the_gallery -- --ignored
test write_the_gallery ... ok   (209 files)
```
Image task-2-main-call-sharing-1280.png: one row across the top: "Twicecat" + "Search rooms (Ctrl+K)" at left; centred "Sharing your screen and its sound" with a red "Stop sharing" button; right "Sync: Running" with green dot, "Settings" button with gear, account button "@me:example.org ▾".

## Status bar removed; main column is [top bar, banner slot, panes], room pane keeps its place as banners/panels come and go.
Status: PROVEN
```console
$ cargo test -p twicecat-app the_room_pane_keeps
test the_room_pane_keeps_its_place_as_panels_bars_and_banners_come_and_go ... ok
test the_slots_follow_the_layout_at_every_width_and_the_room_pane_keeps_its_place ... ok
test result: ok. 2 passed; 0 failed
```
Images show no bottom status bar; the sidebar and room pane run to the window bottom. Tests pass for layout position stability.

## At 720 px the bar fits; below 900 px search/Settings drop their words (icon+tooltip); top_bar_compact table-tested.
Status: PROVEN
```console
$ cargo test -p twicecat-app top_bar
test screens::main_view::tests::the_top_bar_drops_its_words_below_a_width ... ok   (1920,1280,900 false; 899,720,520 true)
test video_stage::the_top_bar_fits_the_window_at_720_and_1280 ... ok
test video_stage::the_top_bar_reaches_what_the_status_bar_and_the_sharing_bar_sent ... ok
test ctrl_wheel_over_the_top_bar_never_logs_out ... ok
test result: ok. 3 passed; 0 failed
```
Image task-2-main-narrow-720.png: "Twicecat", search icon only, "Sync: Running", Settings gear icon only, account avatar + chevron, all inside the 720 px width.

## Every message the old status bar and sharing bar sent is reachable from the top bar; Stop sharing stays reachable over every overlay.
Status: PROVEN
```console
$ cargo test -p twicecat-app stop_sharing_stays
test video_stage::stop_sharing_stays_on_top_of_every_overlay ... ok
test result: ok. 1 passed; 0 failed
```
The reachability test (the_top_bar_reaches_...) above passed; the Stop sharing test passed over all overlays. Both are click-level tests on the real widget tree.
