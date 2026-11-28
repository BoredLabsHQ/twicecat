# UI Review Fixes Plan (PR #44 reviewer comment)
Status: APPROVED

## Goal
Address every point of the reviewer comment on PR #44 (branch `feat/ui-ux-rework`, `crates/app`, iced =0.14.0). The comment says that controls are scattered, so people do not know where to look. This plan gives each area of the window one job:
- **top bar:** the whole app (Settings, the account, global actions);
- **left:** spaces and their rooms;
- **right:** the current room (members, info and settings, threads);
- **centre:** the conversation.

It also fixes the smaller points: push-to-talk capture, the advanced-settings switch, how messages read in the timeline, room-list filters, encryption marks, a named spaces rail, and the notification menu.

Every reviewer item below is mapped to a task, or marked already satisfied with the reason. The plan keeps the decisions of `docs/plans/ui-ux-rework.md` except where a decision below (D2, D9) changes one on purpose:
- Q8 of that plan: the grouped message layout and row order stay (avatars are added on header rows only, D9);
- the layout-slot invariants (the main column changes per D2);
- the Esc order (changed per task, as each task says);
- the kit and the tokens;
- Unicode icons through `ui::icons`.

No pinned dependency changes. No new crates or iced features.

### Decisions (the human's answers to the planning questions)
- **D1 (top bar contents):** as written in Task 2: app name + Search rooms (Ctrl+K) | sharing bar | sync status, Settings, account.
- **D2 (status bar):** removed. The main column becomes `[top bar, banner, panes]`, replacing `[banner, panes, status]` from `ui-ux-rework.md`.
- **D3 (right panel):** one room panel with tabs (Members, Info, Thread) replaces the members and room-settings modals. The call participants panel stays its own slot, so `tests/calls.rs` is unchanged. A list of all threads is out of scope (no core API).
- **D4 (room-list filters):** all nine filters in one drop-down.
- **D5 (core API):** the additive `RoomSummary.encryption: RoomEncryption` (Task 8) is approved. **D5b:** the call dock also flags only unencrypted calls (Task 9).
- **D6 (spaces rail):** a wider rail, about 200 px, with the icon and the name side by side, that the user can collapse to today's 72 px icon rail, like Element's space panel. Not names under icons. How it looks and feels is a human checkpoint item; Task 10 ends by recording in PROGRESS.md and the ledger that the human must check it by hand, and the controller asks the human again once it is built.
- **D7 (push-to-talk binding):** one binding; a new capture replaces the other kind. **D7b:** the gamepad button stays a separate picker.
- **D8 (one way to reveal):** the Voice & audio card folds into Settings as sub-pages shown by the switch; the card is retired.
- **D9 (avatars):** the sender's avatar on group header rows (Task 13).
- **D10 (sizes):** `CAPTION` 12 → 13; icon glyphs stay at 16 px (`ICON_SIZE = max(BODY, 16)`, because `BODY` is 14 and would shrink them; the human decided this in the final review); Task 16 is mandatory. **D10b:** timeline cap 880 px, left-aligned; the composer stays full width.
- **D11 (commits):** AGENTS.md rule 5 as written. See "Commits on this branch" below.
- **D12 (remembering the rail choice, was Q12):** yes, the app remembers across restarts whether the spaces rail is expanded or collapsed. The choice is stored as an additive core field, `UiSettings.spaces_rail: SpacesRailChoice { Expanded, Collapsed }` (default `Expanded`), saved in `settings.json` with the other UI settings by a separate `feat(core)` task, Task 10a, placed before Task 10. The additive core change is approved. It goes in `UiSettings` (`crates/core/src/api/rooms.rs:546`, read field by field by `settings_from_map`, `crates/core/src/paths.rs:25`, written by `write_settings`, `:481`), next to `room_list`, the room list's own expanded/rail choice. It does not go in the per-account view file (`view_file` / `write_view_space`, `paths.rs:762,806`): that file holds only the selected space, under `"version": 1`, and `write_view_space` rewrites the whole object, so a second key there would need a reworked writer. Like `room_list`, the choice is per device, not per account.

### Reviewer items and where they go (checked against the code at `6cd1433`)
| # | Reviewer item | What the code does today | Task |
|---|---|---|---|
| 1 | App settings hard to find | Settings is only an entry of the account `pick_list` in the status bar, at the bottom right (`main_view.rs:1108-1149`) | Task 2 |
| 2 | Room members, info and settings hard to find | Two unlabelled `icon_button`s in the room header (`main_view.rs:329-336`). Both open modal dialogs (`screens/members.rs` and `screens/room_settings.rs` use `kit::dialog`) | Tasks 3, 4, 5, 6 |
| 3 | Log out next to the account switcher | A `secondary_button("Log out")` beside the account `pick_list` (`main_view.rs:1139`). It was kept out of the `pick_list` on purpose: Ctrl + wheel picks a closed pick list's last entry (`main_view.rs:1044-1063`) | Task 1 |
| 4 | Push-to-talk keyboard and mouse set in two places | **Set key…** already accepts a mouse button (`AudioMessage::CapturedMouse`, `audio_settings.rs:1578-1588`) and saves it to `push_to_talk.mouse_button`. But the key row only shows `push_to_talk.key` (`screens/audio_settings.rs:554-589`). The card's Talking page also has a separate "Mouse button" `pick_list` (`:496`). So a captured mouse button looks as if it was not taken | Task 11 |
| 5 | "Show advanced settings" does not show everything | With the switch on, the Settings Voice & audio page shows only the basics plus a **More audio options…** button (`screens/settings.rs:458-474`). That button opens a separate Voice & audio card with nine pages (`AudioPage`, `audio_settings.rs:1118`), and the card ignores the switch. The Video page already inlines everything (`screens/video_settings.rs:80`) | Task 12 |
| 6 | Messages blend together | The group header is a SemiBold name at body size in the default text colour, with a caption timestamp (`timeline_pane.rs:843-854`). There is no avatar, although `INDENT_PX` (64) leaves room for one (`timeline.rs:84`). A hover background exists (`row_background`, `message_hover`) but is faint | Task 13 |
| 7 | One job per area | No top bar exists; the main column is `[banner, panes, status]`. Threads and the call panel are right-side slots; members and room settings are modals; actions and filters sit above the room list | Tasks 2, 3–6, 7 (umbrella; no task of its own) |
| 8 | Text and icons too small | **Already satisfied (setting exists):** Settings → Appearance → Text size, 80–200 % in steps of 10, plus Ctrl+= / Ctrl+- / Ctrl+0 (`screens/settings.rs:302-370`, `App::scale_factor`, `settings.rs:1245`). It is hard to find, which Task 2 fixes. The header icons get words in Task 6. The default small sizes are raised in Task 16 (D10) | Task 16 |
| 9 | Messages stretch the full width; markers far away | No max width: the rows are `width(Fill)` with fixed side padding (`timeline_pane.rs:976-997`) | Task 14 |
| 10 | Reply quote shows sender but no text | The core leaves `body` empty for any original that is not a message (`core/src/timeline.rs:376-379`). `reply_body` keeps only the first line, so a body starting with a newline is empty (`:433-440`; test at `:1038`). The app then shows "(no text)" (`timeline_pane.rs:928-932`). The reviewer saw no line at all, so a second cause, in layout or contrast, may exist. Not reproduced yet | Task 15 |
| 11 | Room-list filters take three rows | Rows: New room / Direct message / Join (`room_list_pane.rs:80-95`), then nine wrapping filter chips (`TABS`, `:116`; `chips`, `:300`), then search + Sort | Task 7 |
| 12 | Lock icon on every room | The lock is drawn for every encrypted room (`encrypted_mark`, `room_list_pane.rs:180`). Flagging unencrypted rooms instead needs a third state. The core folds the SDK's `EncryptionState::{Encrypted, NotEncrypted, Unknown}` (`matrix-sdk-base-0.19.1/src/room/encryption.rs:35-50`) into `is_encrypted: bool` (`core/src/room_list.rs:291`). Unknown would then look like "not encrypted" (comment at `room_list_pane.rs:177-179`) | Tasks 8, 9 |
| 13 | Spaces rail has no labels | 72 px rail (`SIDEBAR_WIDTH`, `spaces_sidebar.rs:55`), 48 px slots (`:58`), name only in the tooltip (`:716`). `room_pane_width` subtracts the fixed width (`main_view.rs:174`) | Tasks 10a, 10 |
| 14 | Unlabelled "All messages" menu in the header | A `pick_list` of `ModeChoice` whose only hint is a "Notifications" tooltip (`main_view.rs:416-429`) | Task 6 |

### Rules for every task
- **AGENTS.md rule 2:** any iced API not cited here is read in `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced*-0.14.*` before use, and cited file:line in the ledger. Checked while planning:
  - `Container::max_width` (`iced_widget-0.14.2/src/container.rs:133`), `center_x` (`:145`) and `align_x` (`:188`);
  - `Column::max_width` (`column.rs:121`);
  - `MouseArea::on_enter` / `on_exit` / `on_right_press` (`mouse_area.rs:106,120,68`).
- **Rule 3:** no version changes, no new crates, and iced features unchanged.
- **Rule 7:** `crates/app` reaches the core only through `twicecat_core::api`. `scripts/check-app-deps.sh` and `scripts/check-core-api.sh` pass. The core changes are Task 8 (additive, approved, D5), Task 10a (additive, approved, D12) and the `reply_body` / `map_reply` fix of Task 15. No other task touches `crates/core`; Task 10 reads and saves the rail choice only through `UiSettings` from `twicecat_core::api`.
- **Rule 5:** see "Commits on this branch" below. Every task commit updates `PROGRESS.md` and the ledger `docs/plans/ui-review-fixes.progress.md` in the same commit.
- **Gate:**
  - `cargo fmt --all`;
  - `cargo clippy --workspace --all-targets -- -D warnings`, and again with `--all-features`;
  - `cargo test --workspace`;
  - `scripts/check-app-deps.sh`, `scripts/check-core-api.sh`, `scripts/check-font-assets.sh` and `cargo deny check`;
  - `scripts/check-commits.sh 6cd1433`. The base is this plan's start (the head of PR #44 when planned), not `main`: `scripts/check-commits.sh main` already fails on older commits of the branch (`e0e41d1`, `8d82443`, `04da1aa`, and others; checked 2026-11-27) and on the merge `342cd3b`, none of which this plan rewrites. The ledger records this.
- **Invariants kept:**
  - every `pick_list` goes through `kit::menu` (`tests/menus_ignore_the_wheel.rs`);
  - contrast tests for every family;
  - `tests/timeline_layout.rs` (`estimate_height` within ±25 %) and the 10,000-room and 10,000-item tests;
  - `tests/calls.rs` and `tests/ptt.rs` unchanged;
  - Tab order;
  - the keys tests.

  Where a task changes a slot list or a layer of the Esc order, it says so. It then updates the doc comment in `main_view.rs`, the gallery slot test and `docs/accessibility.md` in the same commit.
- **Rules 6 and 9:**
  - fixtures use made-up `example.org` names;
  - tests only against the local homeserver with `.env.test` users;
  - no tokens or bodies in logs;
  - how things look and feel is a human checkpoint item. The gallery PNGs help the review but do not prove the look.

### Commits on this branch (AGENTS.md rule 5, D11)
This replaces the "new commits on top, no rebase, no fixup" practice of `sidebar-rework.md` for the work of this plan.
- **Types:** `feat/ui-ux-rework` is not `main`, so only `feat`, `fix` and `docs` commits are made. A task is `feat(app)` or `feat(core)`. The plan, the ledger and doc-only changes are `docs(plan)`, and the wrap-up (Task 17) is `docs(app)`. `fix` is only for a bug in code that is already on `main` (Task 15 checks this). No `refactor`, `test`, `ci`, `build` or `chore` commit is made: such a change belongs to the task commit it serves.
- **One commit per task**, message `<type>(<scope>): <summary>` with the trailer `Step: uirf.N`, ending with the attribution lines.
- **Fixes after the commit:** a bug, review fix, test or refactor for a task that is already committed is folded into that task's commit, never added as a new commit on top:
  1. `git commit --fixup=<sha of the task commit>`;
  2. `GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash 6cd1433`. The local git is 2.43, where `git rebase --autosquash` without `-i` does not squash (non-interactive autosquash arrived in 2.44). `GIT_SEQUENCE_EDITOR=:` accepts the todo list as is, so nothing interactive runs. The base `6cd1433` keeps every rewrite after the merge `342cd3b` and the earlier branch history;
  3. the gate runs again, including `scripts/check-commits.sh 6cd1433`, which fails on any leftover `fixup!` commit.
- **Pushing:** PR #44 is open and under review. Plain `git push` is used while the push only adds commits. A push after an autosquash rewrites published commits and needs `git push --force-with-lease` on `feat/ui-ux-rework` only, never on `main`. **The controller asks the human for a go-ahead before each force-push** and records the answer in the ledger. Until the go-ahead, folded commits stay local.
- A bug found in a task's own code before its commit is fixed in that commit, as before.

### Task 1: Account menu card with Log out inside
Language: rust
Files: `crates/app/src/screens/main_view.rs` (status bar, `AccountAction`), `crates/app/src/state.rs` (open/close state, Esc), `crates/app/src/keys.rs` (Esc order), `crates/app/src/ui/kit.rs` (a `menu_card` / `menu_entry` helper if the Sort card's pattern is not reusable as is), `docs/accessibility.md`
Depends on: none
Acceptance criteria:
- The account `pick_list` and the separate Log out button are replaced by one account button: the avatar, the account label from `account_label` and a chevron. It opens a card with these entries, in order:
  1. Settings;
  2. Voice & audio;
  3. Sessions;
  4. a divider;
  5. **Log out**, as a `danger_button`-styled entry.

  The card follows the pattern of the Sort card (`room_list_pane::sort_card`, `SidebarMenu::Sort`). It is not a `pick_list`, so Ctrl + wheel can never pick Log out (test: a wheel event with Ctrl held while the card is closed or open sends no message).
- Choosing Log out sends `Message::LogoutRequested`, so the existing warning flow is unchanged (`state.rs:1584-1591`). Every other entry sends the same message as today's `AccountAction::message`. The update tests cover each entry and the card closing after a choice.
- Esc closes the card. Its place in the Esc order (right after the room switcher, like the sidebar cards) is in `keys.rs` and covered by a keys test.
- No Log out button remains outside the card (the test in `main_view.rs` is updated: `log_out_message` is reachable only from the card).
- One `feat(app): …` commit with `Step: uirf.1`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `main-account-menu-*.png` (new scenario). `cargo run -p twicecat-app -- --ephemeral --homeserver http://localhost:8008`, log in as a `.env.test` user: Log out appears only after you open the account menu (human checkpoint).

### Task 2: Top bar for the whole app
Language: rust
Files: `crates/app/src/screens/main_view.rs` (`view`, `main_column`, `status_bar` → `top_bar`), `crates/app/src/screens/video_settings.rs` / `stage.rs` (the sharing bar, which lives in the status bar's middle today), `crates/app/tests/ui_gallery.rs` (slot test), `docs/accessibility.md`
Depends on: Task 1 (D1, D2)
Acceptance criteria:
- A top bar of one row, built with the kit:
  - left: the app name, then a search button "Search rooms (Ctrl+K)" that sends the switcher's existing open message;
  - middle: the screen-sharing bar while sharing (moved from the status bar, same messages);
  - right: the sync dot plus words (`sync_text`, never colour alone), a labelled **Settings** button (gear icon plus the word "Settings", sends `SettingsMessage::Open`), and the account button from Task 1.
- The status bar is removed (D2). The main column becomes `[top bar, banner slot, panes]`, always three slots built with `from_vec`. The `main_view.rs` doc comment and the gallery slot test change with it. The test checks that the room pane keeps its place in the tree when a banner, the thread panel, the call panel or the dock come and go.
- At 720 px the top bar fits without clipping. Below a width constant, the search button and the Settings button drop their words and keep icon + tooltip. This is a pure `top_bar_compact(width) -> bool`, table-tested, and the gallery lays it out at 720 and 1280 px.
- A unit test lists every message the old status bar could send, plus the sharing bar's messages, and checks that each is reachable from the top bar.
- One `feat(app): …` commit with `Step: uirf.2`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `main-*.png` at 1280 and 720 px. In the app, Settings is visible at the top right without opening anything (human checkpoint).

### Task 3: A room panel on the right (generalise the thread slot)
Language: rust
Files: `crates/app/src/layout.rs` (`layout_for`, `SidePanel`), `crates/app/src/state.rs`, `crates/app/src/threads.rs`, `crates/app/src/screens/thread_panel.rs` → `crates/app/src/screens/room_panel.rs` (new; the thread view moves in), `crates/app/src/screens/main_view.rs` (`main_row`), `crates/app/src/keys.rs`
Depends on: none (D3)
Acceptance criteria:
- `RoomPanel { tab: RoomPanelTab {Members, Info, Thread}, open: bool }` is view state in the app. The main row keeps five slots, `[sidebar, list, room pane, room panel, call panel]`: the thread slot is renamed, with the same position and width (`threads::PANEL_WIDTH`). Opening a thread opens the panel on the Thread tab, and the thread view, its composer and its `TimelineView` behave as today. All existing `threads.rs` and `thread_panel` tests pass, with their paths moved.
- The panel header shows the tab strip (Members, Info, plus Thread only while a thread is open) and a close `icon_button`. Switching tabs keeps the open thread, and an update test covers it.
- `layout_for` takes `room_panel_open` in place of `thread_open`. Its whole table test passes with the rename, and the "last opened wins" rule between the room panel and the call panel is kept.
- Esc closes the room panel where it closed the thread panel. The keys tests pass, renamed.
- The panel is empty-safe: with no room selected it cannot be open (test).
- One `feat(app): …` commit with `Step: uirf.3`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: none visible yet beyond the thread, which looks as before inside the new panel frame (gallery `main-thread-*.png`). Members and Info arrive in Tasks 4–5.

### Task 4: Members in the room panel
Language: rust
Files: `crates/app/src/screens/members.rs`, `crates/app/src/rooms.rs` (`MembersPanel` open/close), `crates/app/src/screens/room_panel.rs`, `crates/app/src/state.rs` (layer list), `crates/app/src/keys.rs`
Depends on: Task 3
Acceptance criteria:
- The member list moves from the modal (`kit::dialog`) into the Members tab of the room panel. It keeps:
  - search;
  - the Admins, Moderators, Members and Invited sections;
  - the trust and "In the call" tags;
  - the quoting of names that look like tags;
  - `member_page` windowing;
  - **Invite**, which still opens the invite dialog as a modal.
- `HeaderAction::Members` opens the panel on Members, or closes it when Members is already shown. The update test covers open, switch and close.
- The members modal is gone from the room-dialog layer (`state.rs` layer list). The Esc order loses that entry, and the keys test is updated.
- Every existing `members.rs` and `rooms.rs` member test passes. Only layout-only assertions about the modal width change, and the ledger lists them.
- One `feat(app): …` commit with `Step: uirf.4`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `room-panel-members-*.png`. In the app, the Members button shows the list beside the conversation and the composer keeps its focus (human checkpoint).

### Task 5: Room info and settings in the room panel
Language: rust
Files: `crates/app/src/screens/room_settings.rs`, `crates/app/src/rooms.rs` (`SettingsPanel`), `crates/app/src/screens/room_panel.rs`, `crates/app/src/state.rs`, `crates/app/src/keys.rs`
Depends on: Task 3
Acceptance criteria:
- The Info tab shows, read-only:
  - the room avatar and name;
  - the topic, in full and wrapping;
  - the alias;
  - the encryption state in words;
  - the member count;
  - the notification level (the same control as the header, Task 6).

  Below them are the editable fields `RoomDetails.permissions` allows (name, topic, picture), then Turn encryption on, then Leave. These are the same controls and messages as the modal they replace.
- The two confirmations that cannot be undone (turn encryption on, leave) keep their layout rule: the ask button at the left, and the red confirm button with words at the right, at least `tokens::M` from Cancel. Esc cancels the question first, then closes the panel (update and keys tests).
- `HeaderAction::Settings` opens the panel on Info. The room-settings modal leaves the layer list.
- Every existing `room_settings.rs` test passes, except assertions that are only about the modal frame, which the ledger lists.
- One `feat(app): …` commit with `Step: uirf.5`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `room-panel-info-*.png`. Against the local homeserver, renaming a room from the Info tab updates the header (human checkpoint for the look).

### Task 6: Room header: labelled panel toggles and a labelled notification menu
Language: rust
Files: `crates/app/src/screens/main_view.rs` (`header`, `notification_menu`), `crates/app/src/notifications.rs` (a header-only display wrapper)
Depends on: Task 4, Task 5
Acceptance criteria:
- The two unlabelled icons become toggle buttons with an icon and a word: **Members** (with the joined count when known) and **Info**. Each is drawn active while its tab is shown, by colour and outline, and the words are always there. In the stacked (narrow) header they wrap, and below the compact width they keep the icon and a tooltip only. This is the pure `header_labels(width) -> bool`, table-tested.
- The notification `pick_list` shows a label. A wrapper `HeaderMode(ModeChoice)` displays as "Notifications: All messages", "Notifications: Mentions", and so on. It still goes through `kit::menu` and still sends `Message::RoomModeChosen` with the same `ModeChoice` (update test). A test checks the label text for all four choices.
- The header test that lists every reachable message is updated and still covers `CallMessage::TogglePanel`, every `CallButton::press`, Accept, Decline and Verify.
- One `feat(app): …` commit with `Step: uirf.6`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `main-room-*.png` header. In the app, the header reads "Members", "Info" and "Notifications: …" (human checkpoint).

### Task 7: Room-list header in one row
Language: rust
Files: `crates/app/src/screens/room_list_pane.rs` (`room_actions`, `chips`, `search_field`, `view`), `crates/app/src/state.rs` (the "+" card), `crates/app/src/keys.rs`, `crates/app/src/sections.rs` (only if the sectioned-mode condition reads the tab)
Depends on: none (D4)
Acceptance criteria:
- The expanded list has one header row: a filter drop-down, then the search field, then Sort, then a "+" `icon_button`.
  - The drop-down is a `pick_list` of `TABS` through `kit::menu`, showing "All" and the others with their `tab_label`. Choosing one sends `Message::ChipClicked(tab)`, so the logic is unchanged.
  - The search field is the existing one, with its clear button.
  - The "+" button opens a card with New room, Direct message and Join a room. It sends the same `RoomsMessage::Open*`, closes on a choice and on Esc, and has a keys test.
- The pane column keeps its five slots `[actions, chips, search, list, call bar]`. The chips slot is an empty space in both modes, so the list keeps its tree position. The slot test passes.
- A layout test at the default width (300 px) and density shows the list's top edge at most one control height plus the pane padding below the top of the pane. Today it is three rows.
- The rail keeps its icon column (`rail_actions`) and gains no filter (unchanged).
- The empty states and the sectioned-mode condition (All tab, blank search) behave as before (existing tests).
- One `feat(app): …` commit with `Step: uirf.7`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `main-*.png` room list. In the app, choosing "Unreads" in the drop-down filters the list (human checkpoint).

### Task 8: The core reports a room's encryption as known or not (core)
Language: rust
Files: `crates/core/src/api/types.rs` (`RoomSummary`), `crates/core/src/api/mod.rs` (re-export), `crates/core/src/room_list.rs:291`, `crates/core/src/bin/twicecat-cli.rs` (fixture)
Depends on: none (D5: the API addition is approved)
Acceptance criteria:
- Additive only. `RoomSummary` gains `encryption: RoomEncryption`, with `RoomEncryption { Encrypted, NotEncrypted, Unknown }`, mapped from `matrix_sdk_base::EncryptionState` (`encryption.rs:35-50`, `is_unknown` `:74`). The SDK enum's feature-gated `StateEncrypted` maps to `Encrypted`. `is_encrypted` stays, equal to `encryption == Encrypted`, so no app code breaks.
- A unit test maps each SDK state. The `Default` for fixtures is `Unknown`, and `RoomSummary` keeps its derives.
- `scripts/check-core-api.sh` passes. PROGRESS.md records the addition and the human's approval (D5, given while planning).
- The commit is `feat(core): …` with `Step: uirf.8`; later fixes to it are folded in with `--fixup`.

Evidence: `cargo test -p twicecat-core room_list::` passes the mapping test; `twicecat-cli` lists rooms with `encryption: NotEncrypted` for a room created unencrypted on the local homeserver.

### Task 9: Flag unencrypted rooms, not encrypted ones
Language: rust
Files: `crates/app/src/screens/room_list_pane.rs` (`encrypted_mark`, `RailMark`, `rail_marks`, `rail_tip`), `crates/app/src/ui/icons.rs` (`Icon::Unlocked`, words "Not encrypted"), `crates/app/src/screens/main_view.rs` (header tags, `context_line`), `crates/app/src/screens/call.rs` (dock lock), `crates/app/src/calls.rs` (if the dock's lock rule lives there)
Depends on: Task 8
Acceptance criteria:
- Room rows, rail rows and their tooltips show nothing for `Encrypted` and `Unknown`. For `NotEncrypted` they show an `Icon::Unlocked` mark with the tooltip or tag "Not encrypted". Table test over the three states. The rail's `RailMark::Encrypted` is replaced by `RailMark::NotEncrypted`.
- In the room header, the "Encrypted" tag is dropped. A `NotEncrypted` room shows a `severity_tag(Warning, "Not encrypted")`, so the warning is in words, not colour alone. The Info tab (Task 5) still states the encryption in words for every state.
- The call dock is inverted the same way (D5b): no lock for an encrypted call or one whose state is not known, and an `Icon::Unlocked` mark with the words "Not encrypted" (tag or tooltip, never the icon alone) for an unencrypted call. The dock's state comes from the room's `RoomEncryption`, or from the call's own flag if the dock already reads one; the ledger records which, cited file:line. Table test over the three states; `tests/calls.rs` passes unchanged.
- The icon module test still passes: the new glyph is defined only in `ui/icons.rs`.
- One `feat(app): …` commit with `Step: uirf.9`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `main-*.png` with a fixture room in each state. Against the local homeserver, a room created without encryption shows "Not encrypted" and an encrypted one shows no mark (human checkpoint for the look).

### Task 10a: The core remembers the spaces rail choice (core)
Language: rust
Files: `crates/core/src/api/rooms.rs` (`SpacesRailChoice`, `UiSettings.spaces_rail`, `Default`, the doc test), `crates/core/src/api/mod.rs` (re-export), `crates/core/src/paths.rs` (`settings_from_map`, `write_settings`, tests)
Depends on: none (D12: the additive core change is approved)
Acceptance criteria:
- Additive only. A new `SpacesRailChoice { Expanded, Collapsed }` with the same derives as `RoomListChoice` (`Clone, Copy, Debug, Default, PartialEq, Eq`; `#[default] Expanded`) and doc comments. `UiSettings` gains `pub spaces_rail: SpacesRailChoice`, and `UiSettings::default()` sets `Expanded`. `UiSettings` keeps its derives and stays `Copy` (the `is_copy::<UiSettings>()` test passes). `SpacesRailChoice` is re-exported from `twicecat_core::api` beside `RoomListChoice`.
- `write_settings` writes the key `"spaces_rail"` as `"Expanded"` or `"Collapsed"`, built by hand like the other keys (the serializer guard in `session::tests` passes).
- `settings_from_map` reads it field by field like `room_list`: `"Collapsed"` → `Collapsed`, `"Expanded"` → `Expanded`, and a missing key, an unknown string or a non-string value → the default `Expanded`. No other field changes.
- Backward compatibility, unit tests in `paths.rs`:
  - a `settings.json` written before this task (every current key, no `spaces_rail`) reads with `spaces_rail == Expanded` and every other field as before;
  - `"spaces_rail": "Sideways"` and `"spaces_rail": 3` read as `Expanded` and the file is not renamed `.bad`;
  - a round trip of `Collapsed` and of `Expanded` (extending `settings_round_trip_atomically`);
  - `settings_ignore_unknown_fields_and_fill_missing_ones` still passes, extended with the new key.
- The `UiSettings` doc test asserts `s.spaces_rail == SpacesRailChoice::Expanded`. Every struct literal of `UiSettings` in the workspace still compiles (all use `..UiSettings::default()`; checked while planning: `notifications.rs:502`, `state.rs:10834,10843,13575,14055`, `tests/ui_gallery.rs:93`), so no app code changes in this task.
- No secret or content in the new field; the `assert_no_phase5_secret` test over `UiSettings` (`api/mod.rs:1956`) passes. `scripts/check-core-api.sh` and `scripts/check-app-deps.sh` pass. PROGRESS.md records the addition and the human's approval (D12, given while planning).
- The commit is `feat(core): …` with `Step: uirf.10a`; later fixes to it are folded in with `--fixup`.

Evidence: `cargo test -p twicecat-core paths::` passes the old-file, unknown-value and round-trip tests; `cargo test -p twicecat-core --doc UiSettings` passes; a `settings.json` written by the app before this task still loads with the same settings. Nothing changes in the app until Task 10.

### Task 10: A wider, collapsible spaces rail with names
Language: rust
Files: `crates/app/src/screens/spaces_sidebar.rs` (`SIDEBAR_WIDTH` → `RAIL_COLLAPSED_WIDTH` and `RAIL_EXPANDED_WIDTH`, `space_item`, `folder_view`, the collapse button), `crates/app/src/layout.rs` (`PaneLayout`, `layout_for`), `crates/app/src/screens/main_view.rs` (`room_pane_width`, `main_row`), `crates/app/src/state.rs` (the toggle message), `crates/app/src/settings.rs` (saving through `save_local`), `crates/app/src/spaces.rs` (if the rail's messages live there), `crates/app/src/rail_drop.rs` and `crates/app/src/rail_reorder.rs` (only if a zone reads the rail width), `crates/app/tests/ui_gallery.rs` (slot test, scenarios), `docs/accessibility.md`, `docs/sidebar.md`
Depends on: Task 10a (D6, D12)
Acceptance criteria (D6, like Element's space panel):
- **Two widths.** `RAIL_EXPANDED_WIDTH` = 200 logical px and `RAIL_COLLAPSED_WIDTH` = 72 px (today's `SIDEBAR_WIDTH`, renamed; every user of the old name moves to the width shown). Expanded is the default.
- **Expanded rail.** Each item is one row: the icon in today's 48 px slot (`SLOT_HEIGHT` unchanged), then the name in `BODY`, one line, cleaned by `one_line_capped` and ellipsised to the space left. The unread and mention badges stay on the item and keep their words for screen readers. Home reads "Home". A folder reads its name, or "Folder", and its spaces are indented under it by `NESTED_INDENT`. The sub-space band keeps `SUB_SLOT_HEIGHT` and shows names the same way. The tooltip keeps the full name.
- **Collapsed rail.** Exactly today's 72 px icon rail, with names in tooltips only. The gallery PNG of the collapsed rail matches today's layout.
- **The collapse control.** A button at the bottom of the rail, built with the kit: in the expanded rail a chevron icon plus the word "Collapse"; in the collapsed rail the chevron with the tooltip "Expand spaces". It sends one message (for example `Message::SpacesRailToggled`) that flips the choice. It is a Tab stop after the last rail item; the Tab order test is updated. The Esc order is unchanged. Update test: the message flips the choice and nothing else.
- **Width rule.** A pure `rail_expanded_shown(window_width, expanded_choice, list, room_panel_open, call_panel_open) -> bool` (or a new `PaneLayout.sidebar_expanded` set by `layout_for`): the expanded rail is shown only when the choice is expanded, the window is at least `WIDE_MIN` (1100 px), and the room pane would keep at least `ROOM_PANE_MIN_BESIDE_WIDE_RAIL` = 480 px with it. Otherwise the collapsed rail is shown, and the saved choice does not change (the same rule as the room list's rail). Table test, including: 1280 px, expanded list, no side panel → expanded (1280 − 201 − 301 = 778 px left); 1280 px with the room panel open → collapsed; 1920 px with the room panel and the call panel → expanded; 1099 px → collapsed; NaN → collapsed. The existing `layout_for` table passes unchanged for the other fields.
- **Width accounting.** `room_pane_width` subtracts the rail width shown, not a constant (unit test for both widths). The main row keeps its five slots, and the slot test checks that the room pane keeps its tree position when the rail is toggled.
- **Dragging.** The slot heights are unchanged, so every `rail_drop` and `rail_reorder` zone is unchanged: the existing tests pass, and a new test runs the same drop-index cases with the rail at both widths and gets the same answers. The drag ghost tile is drawn at the width shown.
- **Untrusted names.** A test checks that bidi controls, newlines and a 500-character name are cleaned and capped in the expanded label.
- **Remembering the choice (D12).** The choice is `ui_settings.spaces_rail` (Task 10a), not separate app state. At start the app shows the rail from the `UiSettings` it reads, so a user who collapsed it sees the collapsed rail after a restart; a missing or old `settings.json` gives the expanded rail. The toggle message flips `ui_settings.spaces_rail` and saves it with `save_local()`, the same path as `SettingsMessage::RoomList` (`settings.rs:976-983`), so a failed save shows the existing save error. Update tests (with the test `SettingsStore`): the toggle saves `Collapsed`, then `Expanded`; toggling twice saves twice; an app started with `spaces_rail: Collapsed` lays out the 72 px rail. The width rule collapsing the rail by itself never saves anything (test: opening and closing the room panel at 1280 px sends no save). Logout leaves the choice as it is, like `room_list`.
- One `feat(app): …` commit with `Step: uirf.10`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").
- **The task ends with a manual check.** In the same commit, PROGRESS.md and the ledger record that Task 10 is built but **not verified**: the human must check the wide rail by hand (the checkpoint item below), and the step stays open in PROGRESS.md's human checkpoint list until they sign it off (AGENTS rules 1 and 9). The controller asks the human again once the task is built; the task is not reported as done before that.

Evidence: gallery `main-rail-expanded-*.png` and `main-rail-collapsed-*.png` at 1280 and 1920 px, plus `main-rail-panel-open-1280-*.png` (the rail collapsed by the width rule). In the app (`cargo run -p twicecat-app -- --ephemeral --homeserver http://localhost:8008`, a `.env.test` user in a few local spaces): each space is named without hovering, **Collapse** / the chevron switch between the two rails, and after collapsing it and restarting without `--ephemeral` (a local data dir) the rail opens collapsed (human checkpoint).

### Task 11: One "Set key…" for keyboard and mouse
Language: rust
Files: `crates/app/src/screens/audio_settings.rs` (`key_row`, Talking page, `basics`), `crates/app/src/audio_settings.rs` (`AudioMessage::Captured`, `CapturedMouse`, `ClearKey`, `Mouse`)
Depends on: none (D7, D7b)
Acceptance criteria (D7, one binding):
- The key row is titled **Push-to-talk key** and shows the binding:
  - the key label (`key_label`) when a key is bound;
  - the mouse words (`mouse_words`, for example "Mouse button 4") when a mouse button is bound;
  - both, joined by "or", when an older file has both;
  - "None" when neither is bound.

  It is the same row on the Settings page (`basics`) and on the card's Talking page. **Set key…** accepts a key or a mouse button. The hint already says so (`CAPTURING_HINT`).
- Capturing a key sets `push_to_talk.key` and clears `mouse_button`. Capturing a mouse button sets `mouse_button` and clears `key`. **Clear** clears both. The update tests cover each path, the refused-mouse-button note (`UNBINDABLE_MOUSE`) and a file with both set.
- The separate "Mouse button" `pick_list` is removed from the Talking page. `mouse_note` moves under the key row and is shown only while a mouse button is bound. `AudioMessage::Mouse` and `MouseChoice` are removed if nothing else sends them (grep test), else kept unreachable-from-UI with a ledger note. The "every `AudioMessage` is reachable from a page" test is updated to match.
- The gamepad button stays its own picker, unchanged (D7b).
- `tests/ptt.rs` passes unchanged, because the saved `AudioSettings` fields are the same.
- One `feat(app): …` commit with `Step: uirf.11`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `audio-talking-*.png` and `settings-voice-audio-*.png`. In the app, Settings → Voice & audio → Set key… then a side mouse button shows "Mouse button 4" in the row (human checkpoint; Wayland portal capture is human-only).

### Task 12: One switch reveals every setting (Voice & audio inside Settings)
Language: rust
Files: `crates/app/src/screens/settings.rs` (`voice_audio_page`, page list), `crates/app/src/settings.rs` (`SettingsPage`, sub-pages), `crates/app/src/screens/audio_settings.rs` (pages become sections callable from Settings), `crates/app/src/audio_settings.rs` (`open_audio_controls`, `open_audio_card`, `audio_inline`), `crates/app/src/state.rs` (layer list), `crates/app/src/keys.rs`, `crates/app/src/screens/call.rs` (dock "Audio settings"), `docs/accessibility.md`
Depends on: Task 1, Task 11 (D8)
Acceptance criteria (D8, the card folds into Settings):
- With **Show advanced settings** on, the Settings page list shows the audio pages as indented sub-entries under Voice & audio: Devices, Talking, Microphone, Levels, Mic test, Ringtone, Other voices, Announce and Diagnostics. Each shows that page's existing controls. With the switch off, Voice & audio shows the basics, as today. There is no **More audio options…** button left (grep test on the string).
- Every way of opening audio controls lands in Settings: the account menu, the dock's Audio settings and the talk line (`AudioMessage::Open` / `OpenAt(page)`). With the switch off they open the basics page; with it on they open the asked sub-page. Update tests cover each origin and level.
- The separate Voice & audio card leaves the layer list and the Esc order, and the keys tests are updated. `audio_inline` (live controls, mic test stop, capture stop) runs for any audio sub-page on show. Its existing tests pass, and a test covers switching between audio sub-pages without restarting the mic test.
- The test "the `AudioMessage`s each page can send together cover all of them" now runs over the Settings audio sub-pages.
- A test checks that `advanced_settings` is read only by Settings page code and by `open_audio_controls`, not by any other card, so there is one switch and one place.
- One `feat(app): …` commit with `Step: uirf.12`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `settings-audio-<page>-*.png`. In the app, ticking Show advanced settings makes the audio sub-pages appear in the list at once, with nothing else to click (human checkpoint).

### Task 13: Clearer message boundaries in the timeline
Language: rust
Files: `crates/app/src/screens/timeline_pane.rs` (`header`, `framed`), `crates/app/src/timeline.rs` (`top_pad`, estimates), `crates/app/src/theme.rs` and `crates/app/src/themes.rs` (`message_hover` per family)
Depends on: none (D9)
Acceptance criteria (the grouping and row order of `ui-ux-rework.md` Q8 are unchanged, `is_continuation` with its tests):
- The group header name is `BODY_LARGE` SemiBold in the sender's colour (`readable_on`, on all eight backgrounds; the existing contrast test is extended to the header size). The timestamp stays `muted` `CAPTION` after it.
- The space above a group header (`top_pad(false, density)`) is at least `tokens::L` in Comfortable and `tokens::M` in Compact. A continuation keeps today's pad. Unit test for both densities.
- `message_hover` differs from `background` by a contrast ratio of at least 1.15:1 in every family and mode, and stays under the text contrast floors (test). The row hover background also shows on pointer hover, not only while the action bar is visible, if that is not already so; the existing `bar_visible` path is checked first.
- The sender's avatar (D9): a 36 px `kit::avatar` sits in `INDENT_PX` on header rows only, top-aligned with the name. Continuation rows are unchanged, and the estimate accounts for the avatar height (a header row is at least the avatar tall). A test checks that continuation rows have no avatar and header rows have one. The avatar has no tooltip or message of its own (it is not a new Tab stop), so the Tab order tests pass unchanged.
- `estimate_height` stays within ±25 % of measured for both densities (`tests/timeline_layout.rs`), and the 10,000-item test passes.
- One `feat(app): …` commit with `Step: uirf.13`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `main-room-*.png` with consecutive senders. In the app, a busy local-homeserver room reads with clear groups (human checkpoint).

### Task 14: A maximum width for the timeline
Language: rust
Files: `crates/app/src/ui/tokens.rs` (`TIMELINE_MAX_WIDTH`), `crates/app/src/screens/timeline_pane.rs` (`view`, `framed`, `with_actions`, `reactions_row`, `receipts_row`), `crates/app/src/timeline.rs` (`estimate_height` takes the capped width)
Depends on: Task 13
Acceptance criteria:
- The timeline's rows sit in a column capped at `TIMELINE_MAX_WIDTH` = 880 logical px (`Column::max_width`, `column.rs:121`), left-aligned in the room pane (D10b). The composer, the typing line and the room's bars stay full width (D10b; a layout test at 1920 px checks the composer still spans the room pane). The thread panel's timeline is narrower than the cap, so it is unchanged.
- The action bar, the reactions row and "Seen by" are laid out inside the capped column. A layout test at 1920 px checks that their right edge is at most the column's right edge, and the column's width is at most the cap.
- `estimate_height` gets `min(pane width, cap)`, and `tests/timeline_layout.rs` adds a 1920 px case within ±25 %. Scroll anchoring (`anchor_bottom`) and `offset_from_bottom` are unchanged (existing tests).
- One `feat(app): …` commit with `Step: uirf.14`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery `main-room-wide-*.png` at 1920 px (new size). In the app on a wide window, lines wrap at a readable width and the reactions sit under their message (human checkpoint).

### Task 15: Reply quotes always show a line
Language: rust
Files: `crates/core/src/timeline.rs` (`map_reply`, `reply_body`), `crates/app/src/screens/timeline_pane.rs` (`reply_quote`, `reply_quote_text`), `crates/app/tests/timeline_layout.rs`
Depends on: none
Acceptance criteria:
- **Root cause first:**
  - a headless layout test builds a reply row with a Ready quote and checks that the quote's second text node has height > 0 and lies inside the quote surface's bounds, in both densities;
  - a contrast test checks the quote line colour against `quote_surface` in every family (4.5:1).

  If either fails, fix that. If both pass, the gap is in the data, so stop and ask the human for the quoted event's type (never its body) from the reported room. Until then, do the fixes below.
- **Core:** `reply_body` uses the first non-blank line, not the first line. Its test changes at `timeline.rs:1038`: `"\nsecond"` → `"second"`. `map_reply` gives a short description when the original is not a text message, using words already used for placeholders where they exist: "Image", "File", "Sticker", "Poll", "Encrypted message", "Deleted message". Unit tests cover each kind. The bodies never reach a log, and `no_secret_logging` passes.
- **App:** `reply_quote_text` keeps "(no text)" only as the last fallback, and its tests are extended.
- The commit is `feat(core): …` if only the core changes, else `feat(app): …`, with `Step: uirf.15`. Per AGENTS rule 5 it is `fix` only if the defect is in code already on `main`: check `git log main -- crates/core/src/timeline.rs` and record the finding in the ledger. Later fixes to it are folded in with `--fixup`.

Evidence: `cargo test -p twicecat-core timeline::` and `cargo test -p twicecat-app --test timeline_layout` pass. On the local homeserver, replying to an image and to a message starting with a blank line shows a quote line (human checkpoint).

### Task 16: Raise the smallest type sizes (D10)
Language: rust
Files: `crates/app/src/ui/tokens.rs` (`CAPTION`), `crates/app/src/ui/icons.rs` (icon size), `crates/app/src/timeline.rs` (estimates), `crates/app/src/room_list.rs` (row heights if needed)
Depends on: Task 13, Task 14
Acceptance criteria:
- `CAPTION` goes from 12 to 13 and the icon glyph size is `ICON_SIZE = max(BODY, 16)`, which is 16 px: icons stay at 16 px, not `BODY` (14), as the human decided in the final review. Nothing else in the type scale changes. The text-scale setting still multiplies everything (unchanged).
- The room-list row heights still fit the preview line at both densities (layout test at 100 % and 200 %), and the 10,000-room test passes.
- `estimate_height` stays within ±25 % (timeline layout tests).
- One `feat(app): …` commit with `Step: uirf.16`; later fixes to it are folded in with `--fixup` (see "Commits on this branch").

Evidence: gallery PNGs before and after, side by side in `index.html` (human checkpoint).
Note: the upload card keeps tagging encrypted rooms "Encrypted" on purpose (it confirms where files go); every other place stays silent for encrypted and unknown.

### Task 17: Wrap-up: docs, gallery and checkpoint
Language: rust
Files: `docs/accessibility.md` (layout areas, new Esc order, keyboard map), `PROGRESS.md` ("UI review fixes" section and human checkpoint), `docs/plans/ui-review-fixes.progress.md`, `crates/app/tests/ui_gallery.rs` (scenario list)
Depends on: Tasks 1–16, including Task 10a
Acceptance criteria:
- `docs/accessibility.md` describes the four areas and their jobs. It also covers the new Esc order (account card, "+" card, room panel; no members, room-settings or Voice & audio modal) and the zoom and text-size path. `accessibility_doc_lists_every_shortcut` passes.
- A full gallery run is recorded (count and path), including the new scenarios of Tasks 1, 2, 4, 5, 7, 9, 10, 12, 14.
- PROGRESS.md lists the human checkpoint items below and maps each reviewer item to its task or its "already satisfied" reason. The Task 10 item stays open until the human signs it off.
- A reply on PR #44 summarising the mapping is drafted in the ledger for the human to post (the agent does not post it).
- `scripts/check-commits.sh 6cd1433` passes: no commit after the plan's start is outside feat/fix/docs, and no `fixup!` commit is left.
- The ledger lists every force-push of the branch with the human's go-ahead for it, or states there was none.
- The commit is `docs(app): …` with `Step: uirf.17`; later fixes to it are folded in with `--fixup`.

Evidence: `scripts/check-commits.sh 6cd1433` exits 0; `TWICECAT_GALLERY_DIR=target/ui-gallery cargo test -p twicecat-app --test ui_gallery -- --ignored` writes the PNGs listed in the ledger.

## Human checkpoint (added to PROGRESS.md by the task named)
- (Tasks 1, 2) The top bar and the account menu on Linux, Windows and macOS. Log out is never hit by accident, and Ctrl + wheel over the account button does nothing.
- (Tasks 3–6) The room panel beside the conversation, with the members, info and thread tabs, at 1280 px, below 1100 px and below 800 px, with and without a call. The composer keeps focus and the timeline does not jump.
- (Task 7) The one-row room-list header, the filter drop-down and the "+" card.
- (Task 9) "Not encrypted" on a real unencrypted room and in a call in it, and no mark on encrypted rooms and calls.
- (Task 10, must be checked by hand before the step counts as done) The wide spaces rail: names readable at a glance, with long, RTL and emoji space names; collapsing and expanding it; the rail collapsing by itself when a side panel leaves too little room, and coming back when it closes; dragging spaces, folders and sub-spaces at both widths; whether 200 px feels right at 1280 and 1920 px; the collapsed or expanded choice coming back after a restart (D12).
- (Task 11) Binding a side mouse button and a key on X11, Wayland (portal), Windows and macOS. Push-to-talk works with each in a real call.
- (Task 12) Ticking Show advanced settings reveals every audio option, and the mic test keeps running across sub-pages.
- (Tasks 13, 14, 16) A busy room on a large screen at 100 % and 150 %: groups are clear, and lines and reactions stay near the text.
- (Task 15) The quote line under replies to text, images and edited messages.

## Questions
The planning questions Q1–Q12 are answered; the answers are the decisions D1–D12 above (Q12, remembering the rail choice across restarts, is D12: yes, through Task 10a). No question is open.
