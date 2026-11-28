# Evidence — whole feature: UI review fixes (PR #44)
Commit: 56cfae4 (plan started at 6cd1433)
Environment: headless. `CC=clang-22 CXX=clang++-22`. No Docker, no real window. Gallery written to `<scratchpad>/gal` with `TWICECAT_GALLERY_DIR`. Images are the twicecat-comfortable variants, viewed one by one (1280x800 unless noted). Disk before: 50 GB free; gallery 187 MB.

## Gate and scripts
Status: PROVEN
```console
$ cargo fmt --all --check            -> exit 0
$ cargo clippy --workspace --all-targets -- -D warnings  -> exit 0
$ cargo test --workspace             -> exit 0; 65 "test result" lines, total 3189 passed, 0 failed, 187 ignored
$ cargo check --workspace --lib --bins --locked  -> Finished, exit 0
$ scripts/check-core-api.sh          -> exit 0
$ scripts/check-app-deps.sh          -> exit 0
$ scripts/check-commits.sh 6cd1433   -> exit 0
$ TWICECAT_GALLERY_DIR=<scratch>/gal cargo test -p twicecat-app --test ui_gallery write_the_gallery -- --ignored
test write_the_gallery ... ok   (82 s)
$ git status --short  -> clean before this file
```

## Images viewed (what I saw)
- main-room (1280): top bar "Twicecat | Search rooms (Ctrl+K) ... Sync: Running | Settings | @me:example.org v". Spaces rail (collapsed, 3 icons), room list with one header row "All v | Search rooms | Sort | +", conversation with header "Lounge, Notifications: Use default v, Members, Info, Video, Call". Messages have avatars, coloured bold names with timestamp, spacing between groups. No right panel in this image.
- members: right panel with tabs "Members | Info" and close x, titled "Members (5)", search box, groups Admins/Moderators/Members/Invited, invite field. Header button "Members (5)" is highlighted. (No "Thread" tab visible in this image; Thread tab is shown by main-thread, see task-3 evidence.)
- room-settings (Info tab, copy: final-room-panel-info.png): panel "Info" tab selected; Lounge, topic, address, "Not encrypted: the server can read messages", notifications, Name/Topic fields with Save, Change picture, Enable encryption, Leave room. Header shows "Not encrypted" flag under the room name.
- main-account-menu: card "@me:example.org" with Settings, Voice & audio, Sessions, then a divider and a red "Log out" last; Close. Top bar behind has no Log out button.
- main-add-menu: card "Add" with New room, Direct message, Join a room (opened by the "+" in the list header).
- main-rail-expanded-1280: rail widened (~200 px) with labels "Home", "Home", "Work" and a "< Collapse" button at the bottom; list and conversation shrink to fit.
- main-room-wide (1920): the timeline column stays about 880 px wide at the left of the pane; the composer spans the full width; timestamps stay beside names (no far-away markers).
- main-room-text: reply quote shows a bar, sender "Alice Example" and the line "A long message that has to wrap" above "Thanks, I will read it now."; unencrypted flag only on Lounge/Open house list rows (tiny mark), others have no lock.
- settings-voice-audio-advanced: Settings modal, "Show advanced settings" ticked; Voice & audio page now has a sub-nav (Devices, Talking, Microphone, Levels, Mic test, Ringtone, Other voices, Announce, Diagnostics) and inlines devices, volume, "How you talk", one "Push-to-talk key  F13  [Set key...] [Clear]" row, noise reduction.
- settings-audio-talking (copy: final-settings-talking.png): Talking page: Send, one "Push-to-talk key F13 [Set key...][Clear]" row, then Gamepad button and Release delay. No "Mouse button" row.
- main-narrow-room (720): top bar collapses to icon buttons (search, gear, avatar menu); room list collapses to an avatar strip; header keeps the labels Notifications, Members, Info, Video, Call.
- Stale files: `audio-talking-*`, `audio-devices-*` etc. (76 files, dated 00:57) in the scratchpad dir were left by an older run and are NOT written by the current gallery. The old image audio-talking-twicecat-comfortable.png shows a separate "Mouse button" row and the old "Voice & audio" card; I did not use it as evidence. The current `settings-audio-talking` is the live equivalent. The card page no longer exists (no "More audio options" string in crates/app/src).

## The 14 reviewer items
| # | Item | Status | Shown by |
|---|---|---|---|
| 1 | Settings reachable from the top bar | PROVEN | main-room, main-narrow-room (gear in top bar) |
| 2 | Members/Info/Notifications labelled; room panel with Members/Info/Thread tabs | PROVEN (Thread tab by task-3 image/test only) | main-room header, members, room-settings; task-3-main-thread.png |
| 3 | Log out only inside the account menu card | PROVEN | main-account-menu; test `main_view::tests::the_account_card_lists_four_entries_with_log_out_last` ok |
| 4 | One push-to-talk key row for key and mouse | PROVEN (mouse-bound state by unit test only) | settings-audio-talking, settings-voice-audio-advanced; tests in `audio_settings::tests::talking` (task-11) |
| 5 | One "Show advanced settings" switch reveals all audio pages | PROVEN | settings-voice-audio-advanced (sub-nav, inline); test `state::tests::settings_ui::the_advanced_switch_starts_off_is_saved_and_is_one_switch_for_every_page` ok |
| 6 | Clearer message groups with avatars | PROVEN | main-room, main-room-text |
| 7 | One job per area (umbrella) | PROVEN | main-room: top bar / rail+list / conversation / panel |
| 8 | Type size 13 | PROVEN | `tokens::CAPTION = 13.0` (tokens.rs:26); scale test in gate |
| 9 | Capped timeline width | PROVEN | main-room-wide; `TIMELINE_MAX_WIDTH = 880` |
| 10 | Reply quotes always show a line | PROVEN | main-room-text; `timeline_layout::a_ready_reply_quote_lays_out_its_line_inside_the_surface_in_both_densities` ok |
| 11 | Room-list header in one row, filter drop-down, "+" card | PROVEN | main-room, main-add-menu |
| 12 | Only unencrypted rooms flagged | PROVEN | room-settings header flag, main-room-text list marks; tests `room_list_pane::tests::only_a_room_known_to_be_unencrypted_has_a_mark`, `main_view::tests::the_context_line_and_the_header_flag_only_unencrypted_rooms` ok |
| 13 | Wider collapsible spaces rail, persisted | PROVEN (image + state tests; persistence is the saved UiSettings write, not a real file reload) | main-rail-expanded-1280; `state::tests::spaces_rail::the_toggle_saves_collapsed_then_expanded_and_changes_nothing_else`, `the_rail_starts_expanded` ok |
| 14 | Labelled "Notifications" menu in header | PROVEN | main-room header "Notifications: Use default v" |

Minor observation (not a failure): in the 1280 members image the header wraps onto three rows (name, then Notifications/Members/Info, then Video/Call) when the panel is open; in main-room (no panel) it is one row.

## NOT PROVEN (human only)
- The real window (rendering on a display, fonts, scaling, Ctrl+= / Ctrl+- in a live window).
- A real mouse-button capture by "Set key..." in a window.
- The local homeserver checks (Docker not started): sending, replies against a real room, encryption state `Unknown` vs `NotEncrypted` from a real SDK.
- Wayland capture / hotkeys, rail setting persisted to disk and reloaded across a real restart.
- Posting the PR #44 reply (drafted in the ledger).

## Verdict
EVIDENCE: PROVEN
