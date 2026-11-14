# Sidebar Rework Plan (Discord and Element)
Status: APPROVED

## Goal
Make twicecat's left side look and work more like Discord and Element, on top of the sidebar that already exists (sidebar customization plan, `docs/plans/sidebar-customization.md`, steps sbc.1 to sbc.13). The user chose three areas:
1. **The room list.**
   - Element: filter chips, the Favourites and Low priority sections, unreads first, and ordering by activity or by name.
   - Discord: a space's room list is drawn as categories of text and voice channels, each of which can be collapsed.
2. **Drag and drop with the `sweeten` crate wherever sweeten can express it.** The hand-built `drag.rs` stays wherever sweeten cannot.
3. **The space rail.** Discord's server rail with folders, and Element's tree of spaces and sub-spaces.

The `Design` section below pins down exactly what is in scope. Every behaviour and accessibility promise of the sidebar customization plan stays, unless this plan names it as replaced. In particular:
- every move is still in the options menus;
- no keyboard chord is added or changed;
- the layout file stays local, one per account;
- names and IDs never reach a log or a `Debug` output.

**The short answer on sweeten:** sweeten 0.14.0 can only reorder the children of **one** `Column` or `Row` by index (see the "What sweeten 0.14.0 can do" section below). In this plan it carries **only the rail's reordering**: the top-level items, and the spaces inside an open folder. Everything that drops *into* something, or moves *between* containers, stays with `drag.rs` or the menus:
- a room into a section;
- a room onto Favourites or Low priority;
- a section header onto another header;
- a space into or out of a folder.

The room list cannot use sweeten at all, because it is virtualized.

**Where the work goes:** new commits on top of `feat/ui-ux-rework` (head `342cd3b`). Each is a normal `feat(core)`, `feat(app)` or `docs(app)` commit with a `Step: sbr.N` trailer. There is **no rebase, no force-push and no fixup of an earlier commit**; the user decided this. A bug found in a task's own code before its commit is fixed in that commit. A bug found after the commit is fixed in a new commit on top. The ledger is `docs/plans/sidebar-rework.progress.md`.

### What exists today (surveyed at `342cd3b`)
- **The rail** (`screens/spaces_sidebar.rs`):
  - 72 px wide. Home comes first, then `rail_items(layout)`: spaces and folders in the user's order, in one stock `Column` inside a `scrollable`.
  - Each item is a `mouse_area` slot with the pill, the circle or squircle, the badge, a tooltip after 500 ms, and a right-click options card.
  - An open folder is `column![head, band]`, and its spaces sit in an inner `Column`.
  - Dragging uses `drag.rs` (`Drag<RailKey>`) with zones (before, into, after) through `rail_drop::plan`.
  - Sub-spaces are not on the rail (sidebar plan Q5); they are sections of the room list.
- **The room list** (`screens/room_list_pane.rs`):
  - Segmented tabs `All | DMs | Rooms | Spaces` (`RoomListTab`, `room_list_pane.rs:123-140`), and a search field.
  - Flat mode is windowed with a fixed row height. Sectioned mode (`sections.rs`) shows only with a space selected, the All tab, a blank search and the expanded list.
  - Sections are Starred (local, per space), Channels, one per level-1 sub-space, the user's own custom sections, and Direct messages. They are windowed with variable heights (`line_offsets`, `visible_lines`).
  - Rooms keep the SDK's order inside a section: latest local event, then recency, then name (`matrix-sdk-ui-0.19.1/src/room_list_service/room_list.rs:171-181`). Starred and custom sections use the user's order.
  - Voice rooms list their members (`Line::Participant`, `Line::More`).
  - Rooms and headers are dragged with `drag.rs` (`Drag<ListKey>`, `list_drop::plan`), with refusal feedback.
- **The core:**
  - `RoomSummary` (`crates/core/src/api/types.rs:304-341`) has **no tag field**. Sidebar plan Q10 decided that `m.favourite` is not used.
  - `RoomListQuery { tab, search, space }`. `filter_spec` and `build_filter` (`crates/core/src/room_list.rs:77-122`) map a tab to the SDK filters.
  - `UiSettings` (`api/rooms.rs:534-560`) has no sort field.
  - `SectionKey::{Starred, Channels, Direct, SubSpace, Custom}` lives in `api::sidebar` and in the version 1 sidebar file.
- **Accessibility:** iced 0.14 buttons take no keyboard focus. The options menus need a pointer, and the keyboard-only limit is accepted and documented (`docs/accessibility.md`, `docs/sidebar.md`, PROGRESS.md "Sidebar customization: what was deferred").
- **Gate note:** `scripts/check-commits.sh main` already fails on this branch because of the merge commit `342cd3b` ("disallowed merge commits"; checked 2026-11-14). The new commits are checked with `scripts/check-commits.sh 342cd3b` instead, which covers only the commits after the merge. The ledger records this.

### What the pinned crates offer (rule 2; registry sources of the locked versions)
- **Room tags, matrix-sdk 0.19.1 and matrix-sdk-base 0.19.1:**
  - `Room::set_is_favourite(bool, Option<f64>)` (`matrix-sdk-0.19.1/src/room/mod.rs:1752`) also removes `m.lowpriority`;
  - `Room::set_is_low_priority(bool, Option<f64>)` (`:1776`) also removes `m.favourite`;
  - `BaseRoom::is_favourite()` and `is_low_priority()` (`matrix-sdk-base-0.19.1/src/room/tags.rs:40`, `:48`) are reachable from `RoomListItem` through `Deref<Target = Room>` (`matrix-sdk-ui-0.19.1/src/room_list_service/room_list.rs:458-459`).
  - **How a tag change reaches the list:** a tag event updates `notable_tags` (`response_processors/account_data/room.rs:81-89`, `room_info.rs:546-557`). A room info update with no reason is sent as `RoomInfoNotableUpdateReasons::NONE` (`room_info.rs:127-135`), and the room list merges those updates (`room_list.rs:167`). So a tag change should re-emit the room. Task 1 proves this against the local homeserver; if it does not happen, stop and ask (rule 8).
- **List filters, matrix-sdk-ui 0.19.1** (`src/room_list_service/filters/mod.rs:87-104`):
  - `new_filter_favourite`, `new_filter_low_priority`, `new_filter_invite`;
  - `new_filter_read_receipts(ReadReceiptsCategory::{Messages, Notifications, Mentions})`, which matches "`num_unread > 0 || is_marked_unread`" (`read_receipts.rs:68-78`);
  - `new_filter_any` and `new_filter_not`.
  - **The SDK's sort cannot be changed** (it is fixed in `room_list.rs:171-181`), so any other order is applied by the app.
- **iced 0.14:** `Row::wrap()` (`iced_widget-0.14.2/src/row.rs:158`) for the chip row; `Font` weights `BODY_SEMIBOLD` and `BODY_BOLD` already bundled (`crates/app/src/ui/fonts.rs:15-31`); no new font asset.

### What sweeten 0.14.0 can do (rule 2; crate source, git `61351f74` of `airstrike/sweeten`; findings of the earlier probe, re-read for this plan)
- **Source:** `sweeten-0.14.0.crate`, SHA-256 `8e743407da2564f19dc27f8b926214d015effd97e3eb08d08c4a39f70afe6c15`. MIT licence, `rust-version = "1.88"`.
- **Dependencies:** `iced_core`, `iced_widget` and `iced_runtime` `0.14` (resolve to the locked 0.14.0, 0.14.2 and 0.14.0) and `unicode-segmentation`. On a scratch copy of the workspace, the earlier probe saw `Cargo.lock` change by exactly one package, `cargo deny check` pass with `deny.toml` unchanged, and `scripts/check-app-deps.sh` still pass.
- **The API** (`src/widget/drag.rs:11-29`): `DragEvent::{Picked { index }, Dropped { index, target_index }, Canceled { index }}`. It derives only `Debug` and `Clone`.
- **`Column`:**
  - `Column::on_drag(Fn(DragEvent) -> Message)` (`src/widget/column.rs:271`);
  - `deadband_zone(f32)` (`:197`, default 5.0 at `:43`);
  - `style(Fn(&Theme) -> Style)` (`:233`, with `Style { scale, moved_item_overlay, ghost_border, ghost_background }` at `:1492`);
  - `Row` has the same `on_drag` and `deadband_zone` (`row.rs:263`, `:189`).
- **How a drag runs:**
  - Children get each event first. If a child captured it, the column does nothing (`column.rs:636-650`).
  - A left press over the column picks the child whose **centre is nearest** the cursor (`:280-300`) and captures the press (`:698`).
  - A move of **more than** the deadband publishes `Picked` (`:709-730`).
  - A release publishes `Dropped` with the nearest child again. A move or release with no cursor position publishes `Canceled` (`:795-884`).
  - While dragging, the whole column's cursor is `Grabbing` (`:917-919`).
- **What follows for this plan:**
  - Items must **not** capture the press. A `mouse_area` with `on_press` captures it, so the column never picks (probe 1). A click therefore needs a press spy (decision S2).
  - There is no "into", no drop across containers (probe 8, nested columns), no refused place, and no hover event for the app. Feedback is sweeten's own: the item is lifted, the others slide, and a ghost marks the origin.
  - There is **no cancel API**: Esc and focus loss are ignored (probe 5). Changing the widget type that wraps the column resets sweeten's tree and so cancels the drag (probe 7; `iced_core-0.14.0/src/widget/tree.rs:63-67`). A `container` does not reset it, because it forwards its content's tag.
  - Leaving the rail's `scrollable` cancels the drag, because the cursor then has no position (probe 4).
  - **A virtualized list cannot use it.** Only the built lines are children, so a drop index would not be a place in the list. The room list therefore keeps `drag.rs`.

### Design (what "like Discord and Element" means here)
Each rule is concrete and testable. Anything not listed is out of scope. Rules marked **(Qn)** follow the default of that question.

**Rail (Discord's server rail)**
- R1. Home stays first. Below it is a horizontal separator, 32 × 2 px in `Palette.text` at 20 % alpha, centred, with 4 px above and below. It is outside the reorderable column.
- R2. The pill, squircle, badge, tooltip and the folder's 2 × 2 preview stay as sbc.5 drew them. Folders get no colour (Q9).
- R3. Dragging on the rail reorders through sweeten:
  - the top-level spaces and folders reorder in one column;
  - the spaces inside an open folder reorder in that folder's own column;
  - a closed folder is dragged as one item, by its head.
- R4. **Replaced (accepted by the user):** making a folder by dropping a space on a space, putting a space into a folder, and taking a space out of a folder are **menu only** (Move to folder, New folder, Remove from folder). The "into" ring is gone.
- R5. Esc, or the window losing focus, ends a rail drag and saves nothing (decision S3). Leaving the rail ends it too.

**Space tree (Element) (Q4)**
- T1. A top-level space that has level-1 sub-spaces shows a 12 px disclosure chevron at the bottom left of its icon, on hover and while expanded. Clicking the chevron expands or collapses it, and the space's options card gains "Show sub-spaces" and "Hide sub-spaces".
- T2. An expanded space has a tinted band under it (the open folder's `band_style`) listing its sub-spaces:
  - 32 px icons, indented 8 px;
  - in the core's level-1 order;
  - each with its own pill, badge (from `SpaceUnread`) and tooltip.
- T3. Clicking a sub-space selects it. The room list then shows that sub-space: sections Channels, Direct messages and its custom sections, where the layout is keyed by the sub-space's ID, as today for any space. A parent whose sub-space is selected gets the "contains the selected item" look, as a closed folder does today.
- T4. Sub-spaces in the band cannot be dragged and are not drop places. Their order is the server's.
- T5. Whether a space is expanded is kept **in memory only**: not in the file, and reset at logout and restart (Q4). Level-2 spaces stay flattened, because the SDK gives no filter for them (sidebar plan Q5).

**Room list (Element)**
- L1. **Filter chips** replace the segmented tabs, with single choice:
  - `All · Unreads · Mentions · DMs · Rooms · Favourites · Low priority · Invites · Spaces` (Q11);
  - the chips wrap onto more lines when the pane is narrow;
  - clicking the active chip goes back to All;
  - All, DMs, Rooms and Spaces keep their current meaning. Unreads, Mentions, Favourites, Low priority and Invites are new core filters (Task 2);
  - in the collapsed rail mode the chips are not drawn, as the tabs are not today.
- L2. **Sort** is in a "Sort" options card opened from a button in the list's header:
  - "Activity" (the default, the SDK's order) or "A to Z" (case-insensitive, by the cleaned name, then by room ID);
  - plus "Unreads first" (off by default), a stable partition that puts highlight, then notification, then unread or marked rooms before the rest;
  - it is kept in `settings.json` (Q5). It applies to the flat list and to every section whose order is not the user's. Starred and custom sections keep the user's order;
  - with a sort other than the default in flat mode, the app loads every page (`LoadMoreRooms`), as sectioned mode does today.
- L3. **Low priority last (Q6):** in the flat list with the All chip, rooms tagged `m.lowpriority` come after all others, in the chosen sort.
- L4. **Favourites and Low priority sections in a space (Q1):**
  - Favourites shows the space's rooms that carry the Matrix `m.favourite` tag. Low priority shows those that carry `m.lowpriority`.
  - The local Starred section stays as it is.
  - A room is placed in the first section that applies: Starred, Favourites, custom, Low priority, sub-space, Direct messages, Channels.
  - The default section order is Starred, Favourites, Channels, sub-spaces, custom, Direct messages, Low priority. Both new sections are hidden while empty, can be collapsed, and can be moved with Move up and Move down like the built-in sections.
- L5. **Menus** for every room (flat and sectioned): "Add to favourites" or "Remove from favourites", and "Mark as low priority" or "Remove low priority". These send the core command of Task 1, and the tag is kept on the homeserver, so Element shows it too. A failure shows the existing notice for failed actions.
- L6. **Dragging a room** (`drag.rs`) onto the Favourites header adds the tag, and onto the Low priority header marks it. Onto its default section it removes the tag (an extension of "Back to its default section"). Every such drop matches a menu entry. Other refusals are unchanged.

**Categories (Discord), in a space's sectioned list only**
- C1. **Headers:** uppercase name, 12 px, `BODY_SEMIBOLD`, muted text colour, with the chevron on the left; on hover the name turns to the text colour; the "⋯" stays. The rolled-up badge of a collapsed header is unchanged.
- C2. **Rows (Q2)**, with today's row heights in both densities:
  - a text room draws a "#" glyph (plain text, `BODY` font), a voice room the existing `Icon::Speaker`, and a DM keeps its avatar;
  - there is no message preview and no time;
  - an unread room draws its name in `BODY_SEMIBOLD` and the text colour, with a 4 × 8 px pill at the list's left edge;
  - a read room uses the muted colour, and a muted room is dimmed as today;
  - the badges keep `badge_for`'s priority (highlight, count, dot), so no information is lost;
  - the flat list and Home keep today's Element-style rows.
- C3. **Voice after text (Q7):** inside every section whose order is not the user's, text rooms come first and voice rooms after, each group in the chosen sort.
- C4. **A collapsed category (Q3)** still draws the selected room, and the voice room this device's call is in, with its member lines. Other rooms stay hidden, and the header's badge counts them. Alt+Up and Alt+Down walk the drawn rooms, as today.

**Out of scope (recorded so nothing is implied)**
- **Discord:**
  - "Add a server" and "Explore" buttons;
  - creating a channel from a category ("+");
  - folder colours (Q9);
  - per-category permissions;
  - streaming or video icons on voice members;
  - the user panel;
  - dragging a room into a space (that would edit `m.space.child` on the server).
- **Element:**
  - the activity centre;
  - a Mentions or Invites *section*;
  - historical rooms;
  - the "Show message previews" toggle;
  - sections in Home ("All chats") (Q12);
  - writing `m.space_order` (sidebar plan Q14);
  - syncing the layout.
- **Drag:**
  - sweeten anywhere in the room list (Q8);
  - auto-scroll while dragging;
  - sweeten-style animation for `drag.rs` drags;
  - dragging sub-spaces in the tree.
- **Keyboard:** focus for sidebar items through sweeten's focusable `button` (`sweeten-0.14.0/src/widget/button.rs:10`, `:256-266`). That would be a separate plan (Q10).

### Decisions
- **S1, the sweeten rail columns:**
  - Home and the separator sit in a plain iced `Column` above everything else.
  - The top-level items go in one `sweeten::widget::Column` with `deadband_zone(4.0)` and `on_drag(|e| Message::Spaces(SpacesMessage::RailReorder(RailColumn::Top, e.into())))`.
  - Each open folder's band holds its own sweeten column, `RailColumn::Folder(id)`. An expanded space's sub-space band is a stock column whose slots capture the press (`on_press`), so the outer column never picks from it (probe 6), and clicks there work as plain clicks.
  - `on_drag` is set only in `SidebarState::Loaded | Full`.
  - The style comes from `theme::semantic`: `focus` for the ghost border, `selection` for the overlay, and scale 1.05.
  - The app maps `DragEvent` into its own `ReorderEvent`, which derives `PartialEq` as `SpacesMessage` needs.
- **S2, clicks:**
  - Rail slots lose `on_press`.
  - A new `ui/press_spy.rs` (on `iced::advanced`, about 60 lines) publishes `RailPressed(key)` on a left press over its bounds **without** capturing it.
  - A `Release(key)` is a click only if `rail_press == Some(key)` and no sweeten drag runs. `Picked` and `Exit` clear `rail_press`. This also swallows the release that comes just before `Dropped` (probe 2d).
  - The right press stays `on_right_press`.
- **S3, cancel:**
  - `Picked` stores `rail_reorder: Some(RailReorder { column, keys })`.
  - Esc (in `rail_escape`, before the other Esc layers, as today) or `WindowFocus(false)` sets it to `None` and flips `rail_generation`. Each flip changes whether the rail column is wrapped in a stock `column![..]`, and that change resets sweeten's tree (probe 7).
  - A `Dropped` or `Canceled` with no `rail_reorder` is ignored.
- **S4, a drop does what the menu does:**
  - `rail_reorder::target(keys, index, target_index)`: equal indices or an index out of range give `None`; a larger target gives `After` that key, a smaller one `Before` it.
  - The result goes through the existing `rail_drop::plan` and `apply_all_known`, so it uses the same `MoveSpace` and `MoveFolder` ops as the menu and saves once.
  - If the keys changed between `Picked` and `Dropped`, the drop is ignored.
- **S5, what stays:**
  - `drag.rs` stays for the room list only (`Drag<ListKey>`), and its module doc says so.
  - `rail_drag`, `SpacesMessage::{Press, Over}` and the rail half of the window listener go.
  - `rail_drop::plan` stays because the menus use it; its `Zone::Into` paths stay covered by the menu-equivalence tests.
  - The threshold becomes "more than 4 px from the press point" instead of "4 px or more". The plan accepts this and the tests use 4.0 px (a click) and 4.5 px (a drag).
- **K1, tags through the core:**
  - `RoomSummary` gains `is_favourite: bool` and `is_low_priority: bool`.
  - `Command::SetRoomTag { room_id, tag: Option<RoomTagView> }`, where `RoomTagView::{Favourite, LowPriority}`. `Some(Favourite)` calls `set_is_favourite(true, None)`, `Some(LowPriority)` calls `set_is_low_priority(true, None)`, and `None` removes whichever of the two is set.
  - A failure is `CoreEvent::ActionFailed` with a new `ActionKind::RoomTag`. `Debug` follows `SetRoomNotificationMode`.
  - `crates/app` never names the SDK (rule 7).
- **K2, new filters:**
  - `RoomListTab::{Unreads, Mentions, Favourites, LowPriority, Invites}`, mapped in `filter_spec` and `build_filter` to `ReadReceiptsCategory::Messages`, `ReadReceiptsCategory::Mentions`, `new_filter_favourite`, `new_filter_low_priority` and `new_filter_invite`.
  - Each still adds `NonLeft`, the space filter and the search, as today.
  - `Default` stays `All`.
- **K3, sort settings:**
  - `UiSettings.room_sort: RoomSortChoice::{Activity, Name}` and `UiSettings.unread_first: bool`.
  - Read field by field in `settings_from_map` (unknown values give the default) and written by the hand-built JSON.
  - `UiSettings` stays `Copy`.
- **K4, new section keys:**
  - `SectionKey::{Favourites, LowPriority}`, written `"favourites"` and `"lowpriority"`.
  - The file stays **version 1**. An older twicecat drops the unknown keys, as its validation already does, and loses only where those two sections were placed in `order` or `collapsed`. `docs/sidebar.md` says so. This is not `Newer`, because nothing else is lost (Q13).
- **A1, accessibility:**
  - Every new action is in an options card: favourite, low priority, sort, unreads first, and showing or hiding sub-spaces.
  - Every new pick list goes through `kit::menu` inside `ui::guard::no_wheel`.
  - `SHORTCUTS` gets no row, and no key changes.
  - Chips are buttons and clicks only, like the tabs today.
  - The keyboard-only limit stays as documented. Ctrl+K still finds every room.

### Questions for the human (the plan follows each default)
- **Q1, Favourites:**
  - Default **A**: add Favourites and Low priority from the Matrix tags (shared with Element and other clients, kept on the homeserver) and keep the local Starred section unchanged. This reverses half of sidebar plan Q10: the core gains tag fields.
  - **B**: Starred is replaced by tag Favourites. The local stars are still shown in Favourites until they are removed.
  - **C**: no Matrix tags; Low priority is a local section like Starred.
- **Q2, Discord rows** inside a space's sectioned list: a "#" or speaker glyph, no preview, bold when unread. Home and the flat list keep today's rows. This reverses sidebar plan Q11 for spaces. Default: **yes**.
- **Q3, a collapsed category** still shows the selected room and this device's voice room. Unread rooms stay hidden and are counted on the header. Default: **yes**.
- **Q4, the space tree:** sub-spaces can be shown under their parent on the rail and selected (rules T1 to T5), which partly reverses sidebar plan Q5; the room-list sub-space sections stay. Whether a space is expanded is kept in memory only. Default: **yes, in memory only**. The alternative is to keep it in the file, which needs a new `expanded` list in the version 1 file (an older twicecat would drop it).
- **Q5, sort:** Activity or A to Z plus "Unreads first", global in `settings.json`, never over the user's own order. Default: **yes**. The alternative is per space, in the sidebar file.
- **Q6, Low priority last** in the flat All list. Default: **yes**.
- **Q7, voice rooms after text rooms** in sections the user did not order. Default: **yes**.
- **Q8, sweeten in custom sections** (animated reordering of rooms inside one of your sections): it would need each such section drawn whole, outside the windowing, and it would make dragging a room *out* of that section impossible, a regression of sbc.10. Default: **no**; `drag.rs` keeps that reorder.
- **Q9, folder colours** (Discord): default **no** (sidebar plan Q9 stands).
- **Q10, keyboard focus** for rail items and rows through sweeten's focusable buttons, which could lift the keyboard-only limit without chords. Default: **out of scope**, a separate plan if wanted.
- **Q11, the chip set and order:** All, Unreads, Mentions, DMs, Rooms, Favourites, Low priority, Invites, Spaces. Default: **this set**. Spaces is kept so that no current tab is lost.
- **Q12, sections in Home** (Element's "All chats"): default **no**; Home stays a flat list with chips and sort.
- **Q13, no file version bump** for the two new section keys (K4). Default: **no bump**.

### Rules for every task (AGENTS.md)
- **Rule 1:** nothing starts before the user approves this plan. Each task starts after the previous one's checks pass.
- **Rule 2:** every SDK, iced and sweeten item used is cited above. Anything else is checked in the registry source and cited in the ledger before use.
- **Rule 3:** the only manifest change is `sweeten = "=0.14.0"` in `crates/app/Cargo.toml` (approved by the user), in Task 5. No pinned version changes. The ledger records the `Cargo.lock` diff (one package), `cargo tree -p twicecat-app -i sweeten -e normal` and `cargo deny check`.
- **Rule 4, the gate before every commit:**
  - `cargo fmt --all`;
  - `cargo clippy --workspace --all-targets -- -D warnings`, and again with `--all-features`;
  - `cargo test --workspace` and `cargo check --workspace --lib --bins --locked`;
  - `scripts/check-app-deps.sh`, `scripts/check-core-api.sh`, `bash scripts/check-font-assets.sh` and `bash scripts/check-no-webrtc-download.sh`;
  - `scripts/check-commits.sh 342cd3b` (see the gate note above);
  - `cargo deny check`.
- **Rule 5:**
  - one commit per task, on top of the branch, `feat(core)` or `feat(app)` (Task 13 is `docs(app)`), with `Step: sbr.N`;
  - no rebase, no force-push, no `--fixup`;
  - every commit updates `PROGRESS.md`: a new "Sidebar rework steps" section, the "Sidebar rework human checkpoint", and the open issues. It also updates the ledger;
  - each app task that changes what the user can do updates `docs/sidebar.md` and `docs/accessibility.md` **in the same commit**, so the documents never describe a behaviour that is gone;
  - this plan and its ledger go in as `docs(plan)` once approved.
- **Rule 6:** the core integration tests (Tasks 1 and 2) run only against the `docker-compose.yml` homeserver with the `.env.test` accounts (`scripts/require-local-homeserver.sh`). Every app test uses made-up IDs. Nothing logs IDs, names or tags; `no_secret_logging.rs` passes.
- **Rule 7:** the app reaches tags, filters and sort only through `twicecat_core::api`. `check-app-deps.sh` and `check-core-api.sh` pass after every task.
- **Rule 8:** stop and ask if any of these happens:
  - a tag change does not re-emit the room (Task 1);
  - the tree-reset cancel does not reset sweeten in the app's real tree (Task 5);
  - sweeten behaves differently in the app than in the probe;
  - a homeserver test is flaky twice in a row.
- **Rule 9:** everything about how things look and feel in a real window goes on the human checkpoint below. The gallery PNGs are an aid, not proof.

### Task 1: Room tags in the core (Favourite, Low priority)
Language: rust
Files:
- `crates/core/src/api/types.rs`:
  - `RoomSummary.is_favourite` and `RoomSummary.is_low_priority` (both `false` in `RoomSummary::new`);
  - `RoomTagView::{Favourite, LowPriority}`;
  - `ActionKind::RoomTag`.
- `crates/core/src/api/mod.rs`: `Command::SetRoomTag { room_id, tag: Option<RoomTagView> }` and its `Debug`.
- `crates/core/src/room_list.rs`: `summarize` fills both fields from `item.is_favourite()` and `item.is_low_priority()`.
- `crates/core/src/engine/mod.rs`: handles the command through the room-job path (K1).
- The CLI: counts only, if it prints summaries.
- `crates/core/tests/room_tags.rs` (new, local homeserver).
- `PROGRESS.md` and `docs/plans/sidebar-rework.progress.md` (new).

Depends on: none
Acceptance criteria:
- Integration test (`scripts/run-integration-tests.sh`, local stack):
  - after `SetRoomTag { tag: Some(Favourite) }` the room's next summary has `is_favourite == true` and `is_low_priority == false`;
  - after `Some(LowPriority)` the flags are the other way round (the SDK removes `m.favourite`);
  - after `None` both are `false`;
  - a second SDK client of the same account sees `m.favourite` in the room's tags.
- `SetRoomTag` for an unknown room gives `ActionFailed { kind: RoomTag, .. }` and no panic.
- `format!("{:?}", ..)` of the command follows `SetRoomNotificationMode`'s redaction (one test); `no_secret_logging.rs` passes.
- `scripts/check-core-api.sh` passes: no SDK type in `api/`.
- The app compiles and its tests pass with the fields unused.
- Mutation, recorded and reverted: hard-coding `is_favourite: false` in `summarize` fails the integration test.
- The gate passes. Commit: `feat(core): report and set a room's favourite and low priority tags`, with `Step: sbr.1`.

Evidence: `cargo test -p twicecat-core --test room_tags -- --ignored` passes against the local homeserver.

### Task 2: New room list filters in the core
Language: rust
Files:
- `crates/core/src/api/types.rs`: `RoomListTab::{Unreads, Mentions, Favourites, LowPriority, Invites}`.
- `crates/core/src/room_list.rs`: `FilterPart` variants, `filter_spec` and `build_filter` (K2), and the table test.
- `crates/app/src/screens/room_list_pane.rs`: `tab_label` arms only. `TABS` is unchanged, so nothing new is drawn yet.
- `crates/core/tests/room_tags.rs`: filter cases.
- `PROGRESS.md` and the ledger.

Depends on: Task 1
Acceptance criteria:
- `filter_spec` table test: every new tab gives `NonLeft`, its own part, then the space part and the name match, in the existing order.
- Integration test (local stack):
  - with one favourite, one low-priority, one invited, one mentioned and one read room, each tab lists exactly its rooms;
  - `Unreads` also lists a room that is marked unread and has no messages.
- The existing `filter_spec_*` tests pass unchanged.
- The gate passes. Commit: `feat(core): filter the room list by unreads, mentions, favourites, low priority and invites`, with `Step: sbr.2`.

Evidence: `cargo test -p twicecat-core --lib room_list::` and `cargo test -p twicecat-core --test room_tags -- --ignored` pass.

### Task 3: Room list sort settings in the core
Language: rust
Files:
- `crates/core/src/api/rooms.rs`: `RoomSortChoice::{Activity, Name}` (`Default` is `Activity`), and `UiSettings.room_sort` and `UiSettings.unread_first`.
- `crates/core/src/paths.rs`: `settings_from_map` and the writer (K3).
- The app's `UiSettings` literals in tests.
- `PROGRESS.md` and the ledger.

Depends on: none
Acceptance criteria:
- Round trip of both fields through `settings.json`.
- A file without them gives `Activity` and `false`. A value such as `"room_sort": "zigzag"` or `"unread_first": 3` gives the default for that field and keeps the other settings.
- `an_oversized_settings_file_gives_defaults` and the serializer guard test pass unchanged.
- `UiSettings` is still `Copy` (a compile-time assertion).
- The gate passes. Commit: `feat(core): keep the room list sort in the settings`, with `Step: sbr.3`.

Evidence: `cargo test -p twicecat-core --lib paths::` passes. Nothing on screen changes.

### Task 4: Favourites and Low priority section keys in the sidebar file
Language: rust
Files:
- `crates/core/src/api/sidebar.rs`: `SectionKey::{Favourites, LowPriority}`.
- `crates/core/src/paths.rs`: parse and write `"favourites"` and `"lowpriority"` (K4).
- `crates/app/src/sections.rs` and `crates/app/src/sidebar.rs`: exhaustive match arms only. The new keys are never produced yet, so nothing is drawn.
- `PROGRESS.md` and the ledger.

Depends on: none
Acceptance criteria:
- Round trip of a layout whose `order` and `collapsed` hold both new keys. The file still says `"version":1`.
- A version 1 file with an unknown key such as `"pinned"` drops only that key and keeps the rest (the existing rule, now tested explicitly for forward compatibility).
- Every Task 1 test of the sidebar customization plan in `paths::` passes unchanged.
- The gate passes. Commit: `feat(core): section keys for favourites and low priority`, with `Step: sbr.4`.

Evidence: `cargo test -p twicecat-core --lib paths:: api::sidebar::` passes. Nothing on screen changes.

### Task 5: Reordering the rail with sweeten (top level and open folders), with Esc and focus-loss cancel
Language: rust
Files:
- `crates/app/Cargo.toml` (`sweeten = "=0.14.0"`, with a comment that the rail's reordering uses it) and `Cargo.lock`.
- `crates/app/src/ui/press_spy.rs` (new; S2) and `crates/app/src/ui/mod.rs`.
- `crates/app/src/rail_reorder.rs` (new; S4: `RailColumn`, `ReorderEvent`, `target()`) and `crates/app/src/lib.rs`.
- `crates/app/src/screens/spaces_sidebar.rs`:
  - S1: Home outside the reorderable column, then the top-level and per-folder sweeten columns;
  - slots lose `on_press` and `on_move`, and keep `interaction(Grab)`, `on_enter`, `on_exit`, `on_release` and `on_right_press`;
  - the drop line and ring are gone;
  - S3: the wrapper flips with `rail_generation`.
- `crates/app/src/spaces.rs`:
  - `SpacesMessage::{RailPressed, RailReorder}` replace `Press` and `Over`;
  - the click goes through S2;
  - the window listener is for the list only.
- `crates/app/src/state.rs`:
  - `rail_press`, `rail_reorder` and `rail_generation` replace `rail_drag`;
  - Esc, `WindowFocus(false)` and logout clear them;
  - the `rail_dragging` tests are rewritten.
- `crates/app/src/drag.rs`: module doc only (list only).
- `crates/app/src/sidebar_menu.rs`: opening a card clears `rail_reorder`.
- `crates/app/tests/ui_gallery.rs`: `rail-dragging` driven by real events through `Laid`, `rail-dragging-into` removed, and the cancel tests.
- `docs/sidebar.md` and `docs/accessibility.md`:
  - "You can also drag" now says the rail only reorders by dragging;
  - folders are made, filled and emptied with the menu;
  - leaving the rail ends a drag.
- `docs/plans/sidebar-customization.md`: a dated note on decision 6 pointing to this plan, and the `sweeten` exception to its "no new crates" rule.
- `crates/app/tests/docs_sidebar.rs`, if a phrase it checks changes.
- `PROGRESS.md` and the ledger.

Depends on: none (after approval). It comes first among the app tasks because it carries the most risk (rule 8).
Acceptance criteria:
- **Dependencies:**
  - `cargo tree -p twicecat-app -i sweeten -e normal` shows `sweeten v0.14.0` under `twicecat-app` only;
  - the sorted package list of `Cargo.lock` differs by exactly `sweeten 0.14.0`;
  - `cargo deny check` passes with `deny.toml` unchanged.
- **`rail_reorder::target` table test:**
  - equal indices give `None`;
  - `(0, 3)` gives `After(keys[3])` and `(3, 0)` gives `Before(keys[0])`;
  - an index out of range gives `None`;
  - for every pair on a 4-item rail with a folder, and inside a 3-space folder, the result through `rail_drop::plan` equals the matching sequence of menu Move up and Move down.
- **Headless gallery tests** (`Laid`, real events) on Home, A, B and a closed folder F:
  - a press on A, a 4.0 px move and a release selects A and starts no drag;
  - a press on A, a 4.5 px move, a move over F's slot and a release saves once, and the order becomes B, F, A;
  - Home cannot be picked;
  - a press on F's head drags the whole folder;
  - inside an open folder, a drag reorders only inside it.
- **Clicks:**
  - a press over the composer, then a move onto A and a release over A, is no click. Mutation, recorded and reverted: dropping the `rail_press == Some(key)` check fails this test;
  - the `Release` that arrives with `Dropped` selects nothing.
- **Cursor:** over A it is `Grab` when idle and `Grabbing` while dragging. In `Newer` or `OtherAccount`, nothing is picked and the cursor is `Pointer`.
- **Cancel test** with a kept `Tree`, diffed against each new view as the runtime does:
  - after `Picked`, Esc through `App::update` makes the next move and release give no `Dropped`, no click and no save;
  - the same holds for `WindowFocus(false)`.
  - Mutation, recorded and reverted: not flipping `rail_generation` fails this test.
- **Other ends of a drag:**
  - moving out of the rail's scrollable gives `Canceled`, and a later release saves nothing;
  - keys that changed between `Picked` and `Dropped` save nothing.
- **Esc and the room list:**
  - `esc_with_no_drag_is_not_taken_by_the_rail` (rewritten for the new messages) passes;
  - Esc during a list drag still cancels it;
  - the room-list drag tests in `sections.rs` and `list_drop.rs` pass unchanged.
- **Menu equivalence and docs:**
  - the menu-equivalence tests of `rail_drop` still cover Move to folder, New folder and Remove from folder;
  - `grep -rn "onto its middle to make a folder" docs/` finds nothing.
- **Commit:** the gate passes. `feat(app): reorder the rail with an animated drag; folders through the menu`, with `Step: sbr.5`. `git log 342cd3b..HEAD~1` is unchanged by this task (same hashes).

Evidence:
- `cargo test -p twicecat-app --test ui_gallery rail` passes.
- `TWICECAT_GALLERY_DIR=target/ui-gallery TWICECAT_GALLERY_ONLY=rail-dragging cargo test -p twicecat-app --test ui_gallery -- --ignored write_the_gallery` writes `target/ui-gallery/rail-dragging-*.png`.
- In `cargo run -p twicecat-app` (local `.env.test` account), dragging a space below another moves it, and Esc during a drag puts it back. How this feels goes on the checkpoint.

### Task 6: Discord rail separator and the Element space tree
Language: rust
Files:
- `crates/app/src/spaces.rs`:
  - `SpacesModel::sub_spaces(of)` (the level-1 spaces listed in a level-0 space's `rooms`, in core order);
  - `App.rail_expanded: HashSet<RoomId>` (memory only), with `SpacesMessage::ToggleTree(RoomId)`;
  - selecting a sub-space.
- `crates/app/src/screens/spaces_sidebar.rs`:
  - R1, the separator;
  - T1, the chevron;
  - T2, the band of 32 px sub-space slots, which capture the press (S1) and use `on_press` as plain clicks;
  - T3, the "contains selected" look.
- `crates/app/src/sidebar_menu.rs` and `screens/sidebar_menu.rs`: "Show sub-spaces" and "Hide sub-spaces" on a space's card.
- `crates/app/src/state.rs`: logout clears `rail_expanded`.
- `crates/app/tests/ui_gallery.rs`: the `rail-tree` scenario (Home, the separator, a space expanded with two sub-spaces, one of them selected and one with a badge, and a closed folder).
- `docs/sidebar.md` and `docs/accessibility.md` (the tree, its menu entries, and that it is not kept).
- `PROGRESS.md` and the ledger.

Depends on: Task 5
Acceptance criteria:
- `sub_spaces`:
  - lists only level-1 spaces held in the parent's `rooms`, in core order, without duplicates;
  - a level-1 space has none;
  - a space with none draws no chevron.
- A click on the chevron toggles the tree and saves nothing (no store call; `MemorySidebar` records none). The card entries do the same. Logout empties `rail_expanded`.
- Selecting a sub-space:
  - sets `query.space` to it;
  - the room list is sectioned with that sub-space's rooms (a `sections::build` test with a level-1 `SpaceView`);
  - the parent shows the selected look while closed.
- Headless tests:
  - a press on a sub-space slot, a 10 px move and a release start no sweeten drag, and the rail order is unchanged and unsaved;
  - a top-level drag of the expanded parent moves the parent and its band as one item.
- Gallery: `rail-tree` passes the layout and overlap checks at 1280 and 720 wide, in both densities and every test theme. The separator is at least 3:1 against the rail in every test theme (`theme::test_themes()`).
- `keys.rs` is untouched and `SHORTCUTS` is unchanged.
- Mutation, recorded and reverted: making the band's slots not capture the press fails the "no drag from a sub-space" test.
- The gate passes. Commit: `feat(app): a separator under Home and sub-spaces under their space on the rail`, with `Step: sbr.6`.

Evidence: `TWICECAT_GALLERY_ONLY=rail-tree` writes `rail-tree-*.png`. In a running app with a test space that has a sub-space, the chevron shows the sub-space, and clicking it lists its rooms.

### Task 7: Element filter chips
Language: rust
Files:
- `crates/app/src/screens/room_list_pane.rs`: `chips()` replaces `segmented_tabs` (L1: wrapping row, clicking the active chip goes back to All); `TABS` holds the nine chips in the Q11 order; empty-list hints for the new chips.
- `crates/app/src/state.rs`: choosing a chip sets `query.tab` as tabs do today; `sections_shown` is unchanged (All only).
- `crates/app/tests/ui_gallery.rs`: the `room-list-chips` scenario (wide and narrow pane, Unreads active) and the existing scenarios redrawn.
- `docs/sidebar.md` and `docs/accessibility.md` (the chips are clicks, like the tabs).
- `PROGRESS.md` and the ledger.

Depends on: Task 2
Acceptance criteria:
- Clicking each chip sends exactly one query with that tab. Clicking the active chip sends `All`. The All, DMs, Rooms and Spaces chips give the same queries as the old tabs (the existing tab tests, adapted only for the click target, pass).
- An empty Unreads list shows its hint ("No unread rooms"), and every chip has a hint (a table test).
- Gallery: at 720 wide the chips wrap with no overlap and no clipping. At 1280 the layout checks of every room-list scenario pass. The chips' text is at least 4.5:1 in every test theme.
- In the collapsed rail mode no chip is drawn.
- `keys.rs` is untouched.
- The gate passes. Commit: `feat(app): filter chips for the room list`, with `Step: sbr.7`.

Evidence: `TWICECAT_GALLERY_ONLY=room-list-chips` writes the PNGs. In a running app, the Favourites chip lists a room favourited in Element on the same test account.

### Task 8: Sort, unreads first and low priority last
Language: rust
Files:
- `crates/app/src/room_list.rs`: a pure `order(rooms, sort, unread_first, low_last) -> Vec<usize>`, stable (L2, L3).
- `crates/app/src/sections.rs`: applies `order` inside every section that is not user-ordered.
- `crates/app/src/screens/room_list_pane.rs`: a "Sort" button and its card (L2, through `kit::card`).
- `crates/app/src/state.rs`:
  - the settings are saved through `SettingsStore`;
  - load-all in flat mode with a sort that is not the default;
  - Alt+Up/Down and Ctrl+Shift+Up/Down walk the shown order.
- `crates/app/tests/ui_gallery.rs`: the `room-list-sort` scenario (the card open; the list in A to Z with unreads first).
- `docs/sidebar.md` and `docs/accessibility.md`.
- `PROGRESS.md` and the ledger.

Depends on: Task 1, Task 3, Task 7
Acceptance criteria:
- Tests of `order`:
  - `Activity` with no flag is the identity;
  - `Name` ignores case and sorts by the cleaned name, then the ID (two rooms named "a" and "A" are ordered by ID);
  - `unread_first` puts highlight before notification, notification before unread or marked, and those before the rest, keeping the order within each group;
  - `low_last` puts `is_low_priority` rooms at the end.
- In a section:
  - Starred and custom sections keep the user's order under every setting;
  - Channels in `Name` order are sorted (a `sections::build` test).
- In flat mode, `Name` with `total > len` sends `LoadMoreRooms` once per page until the list is complete.
- Alt+Down from the last shown room of a group goes to the next room *as shown*.
- Choosing a sort saves the settings once, and a restart restores it (`MemorySettings` or a temp `DataPaths` test).
- `menus_ignore_the_wheel.rs` covers the sort card.
- Mutation, recorded and reverted: a sort that is not stable fails the "order within groups" test.
- The gate passes. Commit: `feat(app): sort the room list by activity or name, with unreads first`, with `Step: sbr.8`.

Evidence: `TWICECAT_GALLERY_ONLY=room-list-sort` writes the PNGs. In a running app, choosing A to Z orders the list by name and survives a restart.

### Task 9: Favourites and Low priority sections, and their menu entries
Language: rust
Files:
- `crates/app/src/sections.rs`: the placement and default order of L4, with hidden empty sections.
- `crates/app/src/sidebar.rs`: `MoveSection` handles the two new keys.
- `crates/app/src/sidebar_menu.rs` and `screens/sidebar_menu.rs`: on every room's card, "Add to favourites" or "Remove from favourites", and "Mark as low priority" or "Remove low priority". They send `Command::SetRoomTag`.
- `crates/app/src/state.rs`: the command, and the notice on `ActionFailed { RoomTag }`.
- `crates/app/tests/ui_gallery.rs`: `space-sections-tags` (Starred, Favourites, Channels, Low priority, with a collapsed one).
- `docs/sidebar.md`: Favourites and Low priority are Matrix room tags kept on the homeserver, so other Matrix apps such as Element show them too; the local arrangement is still on this device only.
- `crates/app/tests/docs_sidebar.rs`: a check of that sentence.
- `PROGRESS.md` and the ledger.

Depends on: Task 1, Task 4, Task 8
Acceptance criteria:
- Placement tests (made-up IDs):
  - every room appears once;
  - Starred beats Favourites, Favourites beats custom, custom beats Low priority, and Low priority beats sub-space, DM and Channels;
  - the default order is that of L4;
  - both sections are hidden while empty;
  - a stored `order` holding `favourites` is kept.
- Menu:
  - "Add to favourites" on a room's card sends exactly one `SetRoomTag { Some(Favourite) }`, and "Remove from favourites" sends `None`;
  - the same holds for low priority;
  - the entries show the right wording for the room's current flags (a table test);
  - no sidebar file is saved by a tag action.
- Collapse and Move up/down on the two new headers save as for the other built-in sections.
- `docs_sidebar` passes, including the existing "no sync" and "no removal at logout" checks, and the new check that `sidebar.md` says Favourites and Low priority are kept on the homeserver.
- Mutation, recorded and reverted: letting Low priority beat a custom section fails the placement test.
- The gate passes. Commit: `feat(app): favourites and low priority sections from the room tags`, with `Step: sbr.9`.

Evidence: `TWICECAT_GALLERY_ONLY=space-sections-tags` writes the PNGs. In a running app, "Add to favourites" moves the room into Favourites, and Element on the same test account shows it as a favourite.

### Task 10: Dragging a room onto Favourites or Low priority
Language: rust
Files:
- `crates/app/src/list_drop.rs`: a new verdict `Verdict::Tag(RoomId, Option<RoomTagView>)` for drops onto the two headers and onto the room's own default section when the room is tagged (L6).
- `crates/app/src/state.rs`: the verdict sends `SetRoomTag`.
- `crates/app/tests/ui_gallery.rs`: `space-sections-dragging-tag`.
- `docs/sidebar.md` ("You can also drag").
- `PROGRESS.md` and the ledger.

Depends on: Task 9
Acceptance criteria:
- Table test: each tag drop gives the same command as the matching menu entry of Task 9.
- A room already in Favourites dropped on Favourites does nothing and sends nothing.
- A Starred room dropped on Low priority is refused and drawn as not allowed: Starred wins, so the menu path is "Unstar" first.
- Every existing `list_drop` and `space-sections-dragging*` test passes unchanged.
- Mutation, recorded and reverted: mapping a Low priority drop to `Favourite` fails the table test.
- The gate passes. Commit: `feat(app): drag a room onto favourites or low priority`, with `Step: sbr.10`.

Evidence: the gallery writes `space-sections-dragging-tag-*.png`. In a running app, dragging a room onto Low priority tags it.

### Task 11: Discord categories: headers and channel rows in a space
Language: rust
Files:
- `crates/app/src/screens/room_list_pane.rs`: `section_header` (C1), and `channel_row` (C2) used by `sectioned_list` for non-DM rooms; DMs keep `sectioned_room_row`.
- `crates/app/src/room_list.rs`: pure `channel_glyph(room) -> Glyph::{Hash, Speaker, Avatar}` and `channel_emphasis(room) -> Emphasis::{Unread, Read, Muted}`.
- `crates/app/tests/ui_gallery.rs`: `space-categories` (text and voice channels, an unread one, a muted one, a selected one, and a voice room with members) and the existing sectioned scenarios redrawn.
- `docs/sidebar.md` (a short description of the look).
- `PROGRESS.md` and the ledger.

Depends on: Task 9
Acceptance criteria:
- `channel_glyph`: a call room gives `Speaker`, a DM `Avatar`, anything else `Hash`.
- `channel_emphasis`: a room with unread messages, notifications or a mark gives `Unread`, a room in `Mute` mode gives `Muted`, anything else `Read`.
- Line heights are unchanged in both densities (the `line_offsets` tests pass unchanged), so windowing is unaffected.
- The badge of a channel row equals `badge_for` of the same room (a table test).
- Gallery: `space-categories` passes the layout and overlap checks at 1280 and 720 wide, in both densities and every test theme. In every test theme, unread names are at least 4.5:1 against the row, and read names and headers at least 3:1. The flat and Home scenarios are pixel-identical to before this task.
- Mutation, recorded and reverted: `Unread` for a muted room with notifications fails its test. A muted room with unread messages is `Muted`, as Discord dims it.
- The gate passes. Commit: `feat(app): draw a space's sections as categories of text and voice channels`, with `Step: sbr.11`.

Evidence: `TWICECAT_GALLERY_ONLY=space-categories` writes the PNGs. How they look against Discord goes on the checkpoint.

### Task 12: Discord categories: voice after text, and what a collapsed category keeps
Language: rust
Files:
- `crates/app/src/sections.rs`: C3, inside sections that are not user-ordered, after `order`. C4: a collapsed section keeps the selected room and this device's call room with its member lines.
- `crates/app/src/state.rs`: room stepping over the drawn lines (unchanged code path; tests).
- `crates/app/tests/ui_gallery.rs`: `space-categories-collapsed`.
- `docs/sidebar.md`.
- `PROGRESS.md` and the ledger.

Depends on: Task 11
Acceptance criteria:
- `sections::build` tests:
  - in Channels with two text and two voice rooms, the text rooms come first under every sort;
  - in a custom section the user's order is kept even when a voice room comes first;
  - a collapsed section with the selected room draws its header and that room only;
  - with this device in a call in that section, it also draws the voice room and its member lines;
  - the header badge counts every room of the section;
  - collapsing it with neither room in it draws the header only, as today.
- Alt+Down skips the hidden rooms and reaches the kept one.
- The `space-sections-collapsed` test is updated only where it had the selected room in a collapsed section; the ledger names the change.
- Mutation, recorded and reverted: keeping every unread room in a collapsed section fails the Q3 test.
- The gate passes. Commit: `feat(app): voice channels after text channels; a collapsed category keeps the open room and your call`, with `Step: sbr.12`.

Evidence: `TWICECAT_GALLERY_ONLY=space-categories-collapsed` writes the PNGs. In a running app, collapsing the category of the open room keeps that room in view.

### Task 13: Documents, deferred list and checkpoint
Language: rust
Files:
- `docs/sidebar.md` and `docs/accessibility.md`: a final pass over the whole sidebar:
  - every menu entry is named;
  - the keyboard-only limit;
  - what is on the homeserver (tags) and what is on this device (the layout, the sort);
  - that the space tree is not kept.
- `crates/app/tests/docs_sidebar.rs`: the new menu entries are in `sidebar_md_names_the_menu_entries_the_app_has`.
- `PROGRESS.md`:
  - "Sidebar rework: what was deferred" (the out-of-scope list and Q8 to Q12);
  - the checkpoint completed;
  - the open issues (two drag mechanisms; the tree-reset cancel depends on sweeten's widget tag; the merge commit fails `check-commits.sh main`).
- The ledger.

Depends on: Task 12
Acceptance criteria:
- `cargo test -p twicecat-app --test docs_sidebar` passes. Every entry of every options card (a list generated from the card builders) appears in `docs/sidebar.md`.
- `the_keyboard_map_of_accessibility_gets_no_sidebar_row` passes unchanged.
- The checkpoint below appears in PROGRESS.md, unticked.
- The gate passes. Commit: `docs(app): the reworked sidebar, what it keeps where, and what is left`, with `Step: sbr.13`.

Evidence: `cargo test -p twicecat-app --test docs_sidebar` passes; `grep -n "homeserver" docs/sidebar.md` shows the tag sentence.

## Human checkpoint (added to PROGRESS.md "Sidebar rework human checkpoint" by the task named)
- (Task 5) The rail's drag in a real window (Linux Wayland and X11, Windows, macOS):
  - the lifted item, the sliding and the ghost, in every shipped theme and both densities;
  - Grab and Grabbing;
  - a drag starts only after more than 4 px;
  - Esc and alt-tab end a drag at once and put the item back;
  - leaving the rail ends it;
  - releasing outside the window leaves no stuck drag;
  - a click still selects, and a right-click still opens the card;
  - folders are made, filled and emptied only through the menu (accepted by the user; is it acceptable in use?).
- (Task 5) The room list's drag is unchanged (`drag.rs`): a line, the header highlight, and the refusal.
- (Task 6) The separator, the chevron and the sub-space band; selecting a sub-space; the parent's "contains selected" look; the tree is forgotten after a restart (Q4).
- (Task 7) The chips wrap well at narrow widths, and the active chip is clear in every theme.
- (Task 8) The sort card. A to Z and unreads first feel right in a large account (500+ rooms), and loading every page is not slow.
- (Tasks 9 and 10) A favourite or low priority set here shows in Element on the same account and the other way round, within a few seconds. A failed tag change shows the notice.
- (Task 11) The categories look like Discord in both themes: the uppercase headers, "#" and the speaker glyph, unread names in bold with the pill, read names muted, muted rooms dimmed.
- (Task 12) A collapsed category keeps the open room and the call room; voice channels come after text channels.
- (All) Every new action is possible without dragging, through the options cards, including a trackpad's secondary click on macOS. No shortcut was added or changed. The keyboard-only limit is unchanged; is it still acceptable (Q10)?

What cannot be verified headlessly:
- real rendering (the gallery uses tiny-skia, not wgpu);
- sweeten's animation timing;
- overlays (tooltips, the cards);
- real pointer drags and hover;
- tag round trips with Element.

PLAN READY — awaiting validation
