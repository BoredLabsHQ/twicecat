//! Spaces in the app (plan Q15, Task 13): the sidebar's model and the rule
//! for which spaces it lists.
//!
//! A space filters the room list; it is not a security boundary. A space's
//! admins can list any room as a child (Element allows the same), so being
//! under a space says nothing about who may join the room or who is in it,
//! and the UI never presents it that way.
//!
//! Space names belong to people: nothing here logs them, and no `Debug` of a
//! type here (the model, `RailKey`, `RailItem`, `RailFolder`) shows a room ID or
//! a name.

use std::collections::{HashMap, HashSet};
use std::fmt;

use iced::{Point, Subscription, Task, mouse, touch};
use twicecat_core::api::{
    Command, RailEntry, RoomId, RoomListQuery, RoomSummary, SectionKey, SidebarLayout,
    SpaceUnreadView, SpaceView, UnreadCountsView, UserId,
};

use crate::drag::{Outcome, Target, Zone};
use crate::keys::Shortcut;
use crate::list_drop::{self, ListKey, Verdict};
use crate::rail_drop;
use crate::rail_reorder::{self, RailColumn, RailReorder, ReorderEvent};
use crate::sections::{Header, Line};
use crate::sidebar::{Op as SidebarOp, SidebarState};
use crate::state::{App, Message};
use crate::view_store::ViewAnswer;

/// Most spaces kept; the sidebar is not a place for thousands of buttons.
pub const MAX_SPACES: usize = 200;

/// Most per-space unread entries kept: the core reports level 0 and level 1
/// spaces, so a little more than [`MAX_SPACES`] is room for all of them.
const MAX_UNREAD_ENTRIES: usize = 4 * MAX_SPACES;

/// What an item of the rail stands for. `Debug` hides the room ID.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum RailKey {
    /// The **Home** slot.
    Home,
    /// A space.
    Space(RoomId),
    /// A folder, by its ID in the layout.
    Folder(u16),
}

impl fmt::Debug for RailKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Home => f.write_str("Home"),
            Self::Space(_) => f.write_str("Space(<redacted>)"),
            Self::Folder(id) => f.debug_tuple("Folder").field(id).finish(),
        }
    }
}

/// The spaces the user has joined, as the core last reported them, and the
/// unread counts the core rolled up for them.
#[derive(Default)]
pub struct SpacesModel {
    spaces: Vec<SpaceView>,
    /// For each top-level space that has sub-spaces: where they are in
    /// `spaces`, in the core's order. Worked out once per list, not per frame.
    tree: HashMap<RoomId, Vec<usize>>,
    unread: HashMap<RoomId, UnreadCountsView>,
    outside: UnreadCountsView,
}

impl fmt::Debug for SpacesModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpacesModel")
            .field("spaces", &self.spaces.len())
            .field("unread", &self.unread.len())
            .finish()
    }
}

/// A folder of the rail, with the spaces of it that the core reports.
/// `Debug` shows its number, whether it is open and how many spaces it
/// draws, never its name or a room ID.
#[derive(Clone, PartialEq, Eq)]
pub struct RailFolder<'a> {
    /// Its ID in the layout.
    pub id: u16,
    /// Its name, if the user gave it one.
    pub name: Option<&'a str>,
    /// Whether it is shown open.
    pub open: bool,
    /// Its spaces in the layout's order; stored ones the core does not
    /// report are left out here (and stay in the layout).
    pub spaces: Vec<&'a SpaceView>,
}

impl fmt::Debug for RailFolder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RailFolder")
            .field("id", &self.id)
            .field("named", &self.name.is_some())
            .field("open", &self.open)
            .field("spaces", &self.spaces.len())
            .finish()
    }
}

/// One item of the rail below **Home**. `Debug` hides the room ID and the
/// folder's name.
#[derive(Clone, PartialEq, Eq)]
pub enum RailItem<'a> {
    /// A top-level space on the rail itself.
    Space(&'a SpaceView),
    /// A folder.
    Folder(RailFolder<'a>),
}

impl fmt::Debug for RailItem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Space(_) => f.write_str("Space(<redacted>)"),
            Self::Folder(folder) => f.debug_tuple("Folder").field(folder).finish(),
        }
    }
}

/// The sum of `counts`, saturating; unread when any of them is.
pub fn sum_counts(counts: impl IntoIterator<Item = UnreadCountsView>) -> UnreadCountsView {
    counts
        .into_iter()
        .fold(UnreadCountsView::default(), |sum, c| UnreadCountsView {
            notifications: sum.notifications.saturating_add(c.notifications),
            highlights: sum.highlights.saturating_add(c.highlights),
            unread: sum.unread || c.unread,
        })
}

impl SpacesModel {
    /// Replaces the list with the core's (capped at [`MAX_SPACES`]).
    pub fn set(&mut self, mut spaces: Vec<SpaceView>) {
        spaces.truncate(MAX_SPACES);
        self.tree = sub_space_index(&spaces);
        self.spaces = spaces;
    }

    /// Replaces the unread counts with the core's (capped).
    pub fn set_unread(&mut self, spaces: Vec<SpaceUnreadView>, outside: UnreadCountsView) {
        self.unread = spaces
            .into_iter()
            .take(MAX_UNREAD_ENTRIES)
            .map(|view| (view.space, view.counts))
            .collect();
        self.outside = outside;
    }

    /// Forgets all spaces and counts (logout).
    pub fn clear(&mut self) {
        self.spaces.clear();
        self.tree.clear();
        self.unread.clear();
        self.outside = UnreadCountsView::default();
    }

    /// The spaces, in the core's order.
    pub fn spaces(&self) -> &[SpaceView] {
        &self.spaces
    }

    /// Whether the user is in no space, so the sidebar is not drawn.
    pub fn is_empty(&self) -> bool {
        self.spaces.is_empty()
    }

    /// Whether `id` is one of the spaces.
    pub fn contains(&self, id: &RoomId) -> bool {
        self.spaces.iter().any(|space| space.id == *id)
    }

    /// The unread counts of a space; none when the core sent none.
    pub fn unread_for(&self, id: &RoomId) -> UnreadCountsView {
        self.unread.get(id).copied().unwrap_or_default()
    }

    /// The unread counts of the rooms in no space (the **Home** badge).
    pub fn outside(&self) -> UnreadCountsView {
        self.outside
    }

    /// The counts of a folder: the sum over its spaces.
    pub fn folder_counts(&self, folder: &RailFolder<'_>) -> UnreadCountsView {
        sum_counts(folder.spaces.iter().map(|space| self.unread_for(&space.id)))
    }

    /// The sub-spaces of the top-level space `of`, for the rail's space tree:
    /// the level 1 spaces among its `rooms` (its direct children), each once,
    /// in the core's order. A level 1 space, an unknown space and a space
    /// with none give an empty list. The same rule as the room list's
    /// sub-space sections (`sections::build`).
    pub fn sub_spaces(&self, of: &RoomId) -> Vec<&SpaceView> {
        self.tree
            .get(of)
            .map(|at| at.iter().filter_map(|&i| self.spaces.get(i)).collect())
            .unwrap_or_default()
    }

    /// Whether `selected` is one of the sub-spaces of `of`: a closed parent
    /// then stands for it, as a closed folder does for the space it holds.
    pub fn holds_selected(&self, of: &RoomId, selected: Option<&RoomId>) -> bool {
        selected.is_some_and(|sel| self.sub_spaces(of).iter().any(|sub| sub.id == *sel))
    }

    /// The top-level spaces, in the core's order. Sub-spaces are on the rail
    /// only under their space, when its tree is open (see `sub_spaces`).
    pub fn entries(&self) -> Vec<&SpaceView> {
        self.spaces
            .iter()
            .filter(|space| space.level == 0)
            .collect()
    }

    /// The rail below **Home**: the layout's order and folders merged with
    /// the core's top-level spaces. The layout's order comes first; spaces
    /// the layout does not name follow in the core's order; a stored space
    /// the core does not report (or that is not top-level) is not drawn, and
    /// the layout is not changed. A space is drawn once: the first place the
    /// layout names it wins. A folder with none of its spaces known is not
    /// drawn.
    pub fn rail_items<'a>(&'a self, layout: Option<&'a SidebarLayout>) -> Vec<RailItem<'a>> {
        let top: HashMap<&RoomId, &SpaceView> = self
            .spaces
            .iter()
            .filter(|space| space.level == 0)
            .map(|space| (&space.id, space))
            .collect();
        let mut placed: HashSet<&RoomId> = HashSet::new();
        let mut out = Vec::new();
        for entry in layout.map_or(&[][..], |layout| layout.rail.as_slice()) {
            match entry {
                RailEntry::Space(id) => {
                    if let Some(space) = top.get(id)
                        && placed.insert(id)
                    {
                        out.push(RailItem::Space(space));
                    }
                }
                RailEntry::Folder(folder) => {
                    let mut spaces = Vec::new();
                    for id in &folder.spaces {
                        if let Some(space) = top.get(id)
                            && placed.insert(id)
                        {
                            spaces.push(*space);
                        }
                    }
                    if !spaces.is_empty() {
                        out.push(RailItem::Folder(RailFolder {
                            id: folder.id,
                            name: folder.name.as_ref().map(|name| name.as_str()),
                            open: folder.open,
                            spaces,
                        }));
                    }
                }
            }
        }
        for space in self.spaces.iter().filter(|space| space.level == 0) {
            if placed.insert(&space.id) {
                out.push(RailItem::Space(space));
            }
        }
        out
    }
}

/// Where the sub-spaces of each top-level space are in `spaces`.
fn sub_space_index(spaces: &[SpaceView]) -> HashMap<RoomId, Vec<usize>> {
    let level_1: Vec<usize> = spaces
        .iter()
        .enumerate()
        .filter(|(_, space)| space.level == 1)
        .map(|(at, _)| at)
        .collect();
    let mut index = HashMap::new();
    if level_1.is_empty() {
        return index;
    }
    for parent in spaces.iter().filter(|space| space.level == 0) {
        if index.contains_key(&parent.id) {
            continue;
        }
        let held: HashSet<&RoomId> = parent.rooms.iter().collect();
        let mut seen: HashSet<&RoomId> = HashSet::new();
        let kids: Vec<usize> = level_1
            .iter()
            .copied()
            .filter(|&at| {
                let id = &spaces[at].id;
                *id != parent.id && held.contains(id) && seen.insert(id)
            })
            .collect();
        if !kids.is_empty() {
            index.insert(parent.id.clone(), kids);
        }
    }
    index
}

/// The window event for the room list's drag: the cursor, and the left button
/// coming up. (The rail's drag is a sweeten column's own; it needs none.)
pub(crate) fn pointer_message(event: iced::Event) -> Option<Message> {
    match event {
        iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
            Some(Message::Spaces(SpacesMessage::PointerMoved(position)))
        }
        iced::Event::Touch(touch::Event::FingerMoved { position, .. }) => {
            Some(Message::Spaces(SpacesMessage::PointerMoved(position)))
        }
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | iced::Event::Touch(touch::Event::FingerPressed { .. }) => {
            Some(Message::Spaces(SpacesMessage::PointerPressed))
        }
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | iced::Event::Touch(touch::Event::FingerLifted { .. }) => {
            Some(Message::Spaces(SpacesMessage::PointerReleased))
        }
        // The system took the touch away: not a release, so no drop.
        iced::Event::Touch(touch::Event::FingerLost { .. }) => {
            Some(Message::Spaces(SpacesMessage::PointerCancelled))
        }
        _ => None,
    }
}

fn listener(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    pointer_message(event)
}

/// The message of the sidebar.
#[derive(Debug, Clone, PartialEq)]
pub enum SpacesMessage {
    /// A space button, or **Home** (`None`).
    Select(Option<RoomId>),
    /// The pointer entered an item of the rail.
    Enter(RailKey),
    /// The pointer left an item of the rail.
    Exit(RailKey),
    /// The left button went down on an item of the rail (reported by a press
    /// spy, which leaves the press to the sweeten column that picks the item).
    RailPressed(RailKey),
    /// What a sweeten column of the rail reports: an item picked, dropped or
    /// let go.
    RailReorder(RailColumn, ReorderEvent),
    /// The chevron of a space on the rail: shows or hides its sub-spaces. Kept
    /// in memory only, so it saves nothing.
    ToggleTree(RoomId),
    /// A click on a section header of the room list: collapses or expands
    /// the section of the selected space, and saves it.
    ToggleSection(SectionKey),
    /// The left button came up over an item of the rail: a click when it
    /// went down on the same item and the pointer has not left it since.
    Release(RailKey),
    /// The pointer entered a room row or a section header of the sectioned
    /// list.
    ListEnter(ListKey),
    /// The pointer left a room row or a section header.
    ListExit(ListKey),
    /// The left button went down on a room row or a section header.
    ListPress(ListKey),
    /// The left button came up over a room row or a section header: a click
    /// when it went down on the same one and the pointer has not left it
    /// since (the row opens, the header collapses or expands).
    ListRelease(ListKey),
    /// While a drag runs in the list: the pointer moved over a row or a
    /// header, in this part of it.
    ListOver(ListKey, Zone),
    /// The pointer is at this place inside a row or header it pressed (before
    /// a drag starts): where the ghost is held.
    ListGrab(ListKey, Point),
    /// While the button is down: the pointer moved (window coordinates).
    PointerMoved(Point),
    /// The left button, or a finger, went down anywhere in the window while a
    /// press or drag was tracked: its release was missed, so it is cancelled.
    PointerPressed,
    /// The left button, or the finger, came up anywhere in the window.
    PointerReleased,
    /// The system cancelled the touch (`FingerLost`): what was tracked is
    /// forgotten and nothing is dropped.
    PointerCancelled,
}

/// Where the restore of the remembered space stands (Task 11d). `Debug`
/// hides the ID.
#[derive(Clone, PartialEq, Eq, Default)]
pub(crate) enum ViewRestore {
    /// Nobody is logged in.
    #[default]
    Idle,
    /// The remembered space is being read.
    Loading,
    /// Read: select it as soon as the list of spaces holds it, unless the
    /// user picks something first.
    Waiting(RoomId),
    /// Nothing more to do: restored, nothing was kept, or the user chose.
    Done,
}

impl fmt::Debug for ViewRestore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => "Idle",
            Self::Loading => "Loading",
            Self::Waiting(_) => "Waiting(<redacted>)",
            Self::Done => "Done",
        })
    }
}

impl App {
    /// Starts reading what `account` remembered.
    pub(crate) fn load_view(&mut self, account: &UserId) -> Task<Message> {
        self.view_restore = ViewRestore::Loading;
        self.view_store.load(account).map(Message::ViewLoaded)
    }

    /// The remembered space arrived: wait for it to be in the list.
    pub(crate) fn on_view_loaded(&mut self, answer: ViewAnswer) -> Task<Message> {
        if self.user_id.as_ref() != Some(&answer.account)
            || self.view_restore != ViewRestore::Loading
        {
            // Another account's answer, or the user chose in the meantime.
            return Task::none();
        }
        self.view_restore = match answer.space {
            Some(space) => ViewRestore::Waiting(space),
            None => ViewRestore::Done,
        };
        self.restore_view()
    }

    /// Selects the remembered space once the list of spaces holds it. It is
    /// not saved (it is what was saved). A space that is gone is never
    /// selected, and nothing says so.
    pub(crate) fn restore_view(&mut self) -> Task<Message> {
        let ViewRestore::Waiting(space) = &self.view_restore else {
            return Task::none();
        };
        if !self.spaces.contains(space) {
            return Task::none();
        }
        let space = space.clone();
        self.view_restore = ViewRestore::Done;
        self.select_space(Some(space))
    }

    /// The user chose a space, or Home (`None`): it is selected, and
    /// remembered once when that changes what is remembered. The user's choice
    /// ends the restore, so it is never overridden.
    pub(crate) fn pick_space(&mut self, space: Option<RoomId>) -> Task<Message> {
        let pending = matches!(
            self.view_restore,
            ViewRestore::Loading | ViewRestore::Waiting(_)
        );
        let before = self.query.space.clone();
        let select = self.select_space(space.clone());
        if self.view_restore != ViewRestore::Idle {
            self.view_restore = ViewRestore::Done;
        }
        let chosen = self.query.space == space;
        let (Some(user), true) = (self.user_id.clone(), chosen) else {
            return select;
        };
        if self.query.space == before && !pending {
            return select;
        }
        let save = self.view_store.save(&user, space).discard();
        Task::batch([select, save])
    }

    /// Drops what was remembered for the logged-in account (an explicit
    /// logout).
    pub(crate) fn forget_view(&mut self) -> Task<Message> {
        match self.user_id.clone() {
            Some(user) => self.view_store.remove(&user).discard(),
            None => Task::none(),
        }
    }

    /// A space button, or **Home** (`None`): the room list is filtered to the
    /// space, keeping the tab and the search. An unknown space is refused.
    pub(crate) fn select_space(&mut self, space: Option<RoomId>) -> Task<Message> {
        if space.as_ref().is_some_and(|id| !self.spaces.contains(id)) {
            return Task::none();
        }
        let query = RoomListQuery {
            tab: self.query.tab,
            search: self.query.search.clone(),
            space,
        };
        self.set_query(query)
    }

    /// The pointer on the rail: hover, and a click as a press and a release
    /// on one item with no leaving in between.
    pub(crate) fn on_rail(&mut self, message: SpacesMessage) -> Task<Message> {
        match message {
            SpacesMessage::Select(space) => self.pick_space(space),
            SpacesMessage::ToggleTree(id) => {
                self.set_tree(&id, None);
                Task::none()
            }
            SpacesMessage::ToggleSection(key) => {
                // Saved per space; refused (and nothing changes) while no
                // layout is loaded or the stored file is protected.
                let Some(space) = self.query.space.clone() else {
                    return Task::none();
                };
                match self.apply_sidebar(SidebarOp::ToggleSection { space, key }) {
                    // The rooms in view changed: their avatars and the pages
                    // are due as after any change of the list.
                    Ok(save) => Task::batch([save, self.after_list_change()]),
                    Err(_) => Task::none(),
                }
            }
            SpacesMessage::Enter(key) => {
                self.rail_hover = Some(key);
                Task::none()
            }
            SpacesMessage::Exit(key) => {
                if self.rail_hover.as_ref() == Some(&key) {
                    self.rail_hover = None;
                }
                // Leaving the item ends the click it could have been.
                if self.rail_press.as_ref() == Some(&key) {
                    self.rail_press = None;
                }
                Task::none()
            }
            SpacesMessage::RailPressed(key) => {
                // One pointer: a press here ends any press tracked in the list.
                let _ = self.list_drag.cancel();
                self.rail_press = Some(key);
                Task::none()
            }
            SpacesMessage::Release(key) => {
                // A click is a release over the item the button went down on,
                // with the pointer never gone from it, and no drag running
                // (the release that comes just before a drop is not one).
                let clicked = self.rail_press.as_ref() == Some(&key);
                self.rail_press = None;
                if clicked && self.rail_reorder.is_none() {
                    self.rail_click(key)
                } else {
                    Task::none()
                }
            }
            SpacesMessage::RailReorder(column, event) => self.on_rail_reorder(column, event),
            SpacesMessage::ListEnter(key) => {
                self.list_hover = Some(key);
                Task::none()
            }
            SpacesMessage::ListExit(key) => {
                if self.list_hover.as_ref() == Some(&key) {
                    self.list_hover = None;
                }
                self.list_drag.leave(&key);
                Task::none()
            }
            SpacesMessage::ListPress(key) => {
                let draggable = self.sections_shown()
                    && matches!(
                        self.sidebar.state(),
                        SidebarState::Loaded | SidebarState::Full
                    );
                // One pointer: a press here ends any press tracked on the rail.
                self.cancel_rail_reorder();
                self.list_drag.press(key, draggable);
                Task::none()
            }
            SpacesMessage::ListRelease(key) => match self.list_drag.release_on(&key) {
                Outcome::Click(ListKey::Room(id)) => self.update(Message::SelectRoom(id)),
                Outcome::Click(ListKey::Header(key)) => {
                    self.on_rail(SpacesMessage::ToggleSection(key))
                }
                _ => Task::none(),
            },
            SpacesMessage::ListOver(key, zone) => {
                self.list_over(key, zone);
                Task::none()
            }
            SpacesMessage::ListGrab(key, local) => {
                self.list_drag.grab(&key, local);
                Task::none()
            }
            SpacesMessage::PointerMoved(at) => {
                let _ = self.list_drag.moved(at);
                Task::none()
            }
            SpacesMessage::PointerPressed | SpacesMessage::PointerCancelled => {
                // A button is down again (the release of the one tracked was
                // missed), or the touch was cancelled. Nothing is dropped.
                let _ = self.list_drag.cancel();
                Task::none()
            }
            SpacesMessage::PointerReleased => match self.list_drag.end() {
                Outcome::Drop { item, target } => self.list_drop(&item, &target),
                _ => Task::none(),
            },
        }
    }

    /// Shows (`Some(true)`) or hides (`Some(false)`) the sub-spaces of `id` on
    /// the rail, or flips them (`None`). Only a space that has sub-spaces can
    /// be opened; one that is open can always be closed. Memory only: no store
    /// is touched.
    pub(crate) fn set_tree(&mut self, id: &RoomId, show: Option<bool>) {
        let open = self.rail_expanded.contains(id);
        let show = show.unwrap_or(!open);
        if show == open || (show && self.spaces.sub_spaces(id).is_empty()) {
            return;
        }
        if show {
            self.rail_expanded.insert(id.clone());
        } else {
            self.rail_expanded.remove(id);
        }
        // The rail's items changed height: a press or drag in progress means
        // nothing now, and a hover over a slot that is gone would never end.
        self.cancel_rail_reorder();
        self.clear_stale_rail_keys();
    }

    /// What dropping `item` on `target` would do in the sectioned list.
    fn list_verdict(&self, item: &ListKey, target: &Target<ListKey>) -> Verdict {
        let (Some(space), Some(layout), Some(lines), Some(defaults)) = (
            self.query.space.as_ref(),
            self.sidebar.layout(),
            self.section_lines(),
            self.default_section_lines(),
        ) else {
            return Verdict::Nothing;
        };
        list_drop::plan(space, layout, &lines, &defaults, item, target)
    }

    /// Whether `room` is drawn in a section that is not one of the user's own.
    fn in_built_in_section(&self, room: &RoomId) -> bool {
        self.section_lines()
            .and_then(|lines| crate::sections::spot_of(&lines, room))
            .is_some_and(|spot| !matches!(spot.section, SectionKey::Custom(_)))
    }

    /// The pointer is over `key`, in `zone`, during a drag in the list: a
    /// target if dropping there would change something, refused (drawn as not
    /// allowed) if it may not be done, and nothing otherwise.
    fn list_over(&mut self, key: ListKey, zone: Zone) {
        let Some(item) = self.list_drag.dragged().cloned() else {
            return;
        };
        // Onto a header a room has no before or after: it goes into it.
        let zone = match (&item, &key) {
            (ListKey::Room(_), ListKey::Header(_)) => Zone::Into,
            // A row of a section that is not one of the user's own stands
            // for its header too: the whole row is the target.
            (ListKey::Room(_), ListKey::Room(near)) if self.in_built_in_section(near) => Zone::Into,
            _ => zone,
        };
        // A gap between two rows (or two headers) is one place, named from
        // the line after it: the bottom half of one and the top half of the
        // next are the same target, so the line is drawn once and never moves
        // when the pointer crosses the boundary.
        let (key, zone) = match self.section_lines() {
            Some(lines) => list_drop::single_gap(&lines, &item, key, zone),
            None => (key, zone),
        };
        let target = Target { item: key, zone };
        if self.list_drag.seen(&target) {
            return;
        }
        match self.list_verdict(&item, &target) {
            Verdict::Ops(_) | Verdict::Tag(..) => self.list_drag.hover(Some(target)),
            Verdict::Refused => self.list_drag.refuse(target),
            Verdict::Nothing => self.list_drag.hover(None),
        }
    }

    /// The drag in the list ended on a target: the same operations the cards
    /// offer, done as one change and saved once. A refused change leaves the
    /// layout as it was.
    fn list_drop(&mut self, item: &ListKey, target: &Target<ListKey>) -> Task<Message> {
        self.list_hover = None;
        let ops = match self.list_verdict(item, target) {
            Verdict::Ops(ops) => ops,
            // Kept on the homeserver: the command of the card's entry, and
            // the layout file is not touched.
            Verdict::Tag(room_id, tag) => {
                return self.send(Command::SetRoomTag { room_id, tag });
            }
            Verdict::Nothing | Verdict::Refused => return Task::none(),
        };
        match self.sidebar.apply_all_known(ops, &[], &*self.sidebar_store) {
            // The rooms in view changed: their avatars and the pages are due
            // as after any change of the list.
            Ok(save) => Task::batch([save.discard(), self.after_list_change()]),
            Err(_) => Task::none(),
        }
    }

    /// The ghost tile of the item being dragged, while a drag runs: `None`
    /// before the 4 px threshold, after a drop or cancel, and when the item is
    /// no longer in the list. Derived from the drag state, so every path that
    /// ends the drag ends the ghost.
    pub fn ghost(&self) -> Option<Ghost<'_>> {
        let key = self.list_drag.dragged()?;
        let pointer = self.list_drag.pointer()?;
        let lines = self.section_lines()?;
        let item = lines.into_iter().find_map(|line| match (line, key) {
            (Line::Room(room), ListKey::Room(id)) if room.id == *id => Some(GhostItem::Room(room)),
            (Line::Header(header), ListKey::Header(section)) if header.key == *section => {
                Some(GhostItem::Header(header))
            }
            _ => None,
        })?;
        Some(Ghost {
            item,
            pointer,
            grab: self.list_drag.grab_offset(),
        })
    }

    /// Forgets the hover, press and drag of a row or header the list no
    /// longer draws (a room that left, a section collapsed away, a search
    /// that made the list flat): its pointer-exit would never come.
    pub(crate) fn settle_list_drag(&mut self) {
        if !self.list_drag.is_active() && self.list_hover.is_none() {
            return;
        }
        let drawn: HashSet<ListKey> = self
            .section_lines()
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(|line| match line {
                        Line::Header(header) => Some(ListKey::Header(header.key.clone())),
                        Line::Room(room) => Some(ListKey::Room(room.id.clone())),
                        Line::EmptyHint(_) | Line::Participant(_) | Line::More(_) => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        if self
            .list_hover
            .as_ref()
            .is_some_and(|key| !drawn.contains(key))
        {
            self.list_hover = None;
        }
        self.list_drag.retain(|key| drawn.contains(key));
    }

    /// The top-level spaces the core reports, for the drop planner.
    fn known_spaces(&self) -> Vec<RoomId> {
        self.spaces.entries().iter().map(|s| s.id.clone()).collect()
    }

    /// A sweeten column of the rail reported `event` (plan S3 and S4).
    ///
    /// A drop does what the options card does: the same `MoveSpace` and
    /// `MoveFolder` operations, done as one change and saved once.
    fn on_rail_reorder(&mut self, column: RailColumn, event: ReorderEvent) -> Task<Message> {
        match event {
            ReorderEvent::Picked { index } => {
                // The drag starts, so what could have been a click is not one.
                self.rail_press = None;
                let customizable = matches!(
                    self.sidebar.state(),
                    SidebarState::Loaded | SidebarState::Full
                );
                let items = self.spaces.rail_items(self.sidebar.layout());
                let keys = rail_reorder::column_keys(&items, column)
                    .filter(|keys| customizable && index < keys.len());
                if let Some(keys) = keys {
                    // One pointer: a drag here ends any press tracked in the list.
                    let _ = self.list_drag.cancel();
                    self.rail_reorder = Some(RailReorder { column, keys });
                }
                Task::none()
            }
            ReorderEvent::Canceled { .. } => {
                // Sweeten has already let go of the item; nothing is saved.
                self.rail_press = None;
                self.rail_reorder = None;
                Task::none()
            }
            ReorderEvent::Dropped {
                index,
                target_index,
            } => {
                self.rail_press = None;
                // No drag of ours (Esc, focus lost, or one never picked): the
                // drop is not ours to do.
                let Some(reorder) = self.rail_reorder.take() else {
                    return Task::none();
                };
                let items = self.spaces.rail_items(self.sidebar.layout());
                let now = rail_reorder::column_keys(&items, column);
                if reorder.column != column || now.as_deref() != Some(&reorder.keys[..]) {
                    // The rail changed under the drag: the indices mean nothing.
                    return Task::none();
                }
                let Some(place) = rail_reorder::target(&reorder.keys, index, target_index) else {
                    return Task::none();
                };
                self.rail_drop(&reorder.keys[index], &place.target())
            }
        }
    }

    /// Ends the rail's press and drag, wherever the app does it (Esc, focus
    /// lost, a card opened, logout). A drag that ran is ended in sweeten too,
    /// by flipping `rail_generation`: see its doc.
    pub(crate) fn cancel_rail_reorder(&mut self) {
        self.rail_press = None;
        if self.rail_reorder.take().is_some() {
            self.rail_generation = self.rail_generation.wrapping_add(1);
        }
    }

    /// The drag ended on a target: the same operations the options card
    /// offers, done as one change and saved once. A refused change leaves the
    /// layout as it was.
    fn rail_drop(&mut self, item: &RailKey, target: &Target<RailKey>) -> Task<Message> {
        let known = self.known_spaces();
        let Some(layout) = self.sidebar.layout() else {
            return Task::none();
        };
        let ops = rail_drop::plan(layout, &known, item, target);
        if ops.is_empty() {
            return Task::none();
        }
        let task = match self
            .sidebar
            .apply_all_known(ops, &known, &*self.sidebar_store)
        {
            Ok(task) => task.discard(),
            Err(_) => Task::none(),
        };
        self.clear_stale_rail_keys();
        task
    }

    /// Esc during a drag cancels it. The key is used here, so nothing else
    /// closes. `None` when no drag runs.
    pub(crate) fn rail_escape(&mut self, shortcut: Shortcut) -> Option<Task<Message>> {
        if shortcut != Shortcut::Escape
            || !(self.rail_reorder.is_some() || self.list_drag.is_dragging())
        {
            return None;
        }
        self.cancel_rail_reorder();
        let _ = self.list_drag.cancel();
        // The composer's copy of the key must not close another layer.
        self.escape_spent = false;
        Some(Task::none())
    }

    /// The window's pointer, listened to only while the left button is down
    /// on a row or header of the room list (the end of a drag is often
    /// outside every `mouse_area`, which only sees a release over itself).
    /// The rail needs none: its sweeten column sees the whole drag.
    pub fn list_drag_subscription(&self) -> Subscription<Message> {
        if self.list_drag.is_active() {
            iced::event::listen_with(listener)
        } else {
            Subscription::none()
        }
    }

    /// A click on an item of the rail.
    fn rail_click(&mut self, key: RailKey) -> Task<Message> {
        match key {
            RailKey::Home => self.pick_space(None),
            RailKey::Space(id) => self.pick_space(Some(id)),
            // Refused only with no layout loaded, when no folder shows.
            RailKey::Folder(id) => self
                .apply_sidebar(SidebarOp::ToggleFolder(id))
                .unwrap_or_else(|_| Task::none()),
        }
    }

    /// Forgets the hover and press of an item the rail no longer draws (and
    /// closes the options card or dialog about it): a
    /// space that is gone or inside a closed folder, and a folder that is
    /// gone, dissolved or has none of its spaces reported. Its pointer-exit
    /// would never come.
    pub(crate) fn clear_stale_rail_keys(&mut self) {
        // The layout or the spaces changed: what was worked out is stale.
        self.list_drag.forget_seen();
        let mut drawn: HashSet<RailKey> = HashSet::from([RailKey::Home]);
        let items = self.spaces.rail_items(self.sidebar.layout());
        // The sub-spaces under a drawn space whose tree is open are drawn too.
        let with_tree = |space: &SpaceView, drawn: &mut HashSet<RailKey>| {
            drawn.insert(RailKey::Space(space.id.clone()));
            if self.rail_expanded.contains(&space.id) {
                drawn.extend(
                    self.spaces
                        .sub_spaces(&space.id)
                        .iter()
                        .map(|sub| RailKey::Space(sub.id.clone())),
                );
            }
        };
        for item in &items {
            match item {
                RailItem::Space(space) => with_tree(space, &mut drawn),
                RailItem::Folder(folder) => {
                    drawn.insert(RailKey::Folder(folder.id));
                    if folder.open {
                        for space in &folder.spaces {
                            with_tree(space, &mut drawn);
                        }
                    }
                }
            }
        }
        if self
            .rail_hover
            .as_ref()
            .is_some_and(|key| !drawn.contains(key))
        {
            self.rail_hover = None;
        }
        if self
            .rail_press
            .as_ref()
            .is_some_and(|key| !drawn.contains(key))
        {
            self.rail_press = None;
        }
        // A drag over items that are no longer there: its indices mean
        // nothing, and sweeten would index past its children.
        let moved = self.rail_reorder.as_ref().is_some_and(|reorder| {
            rail_reorder::column_keys(&items, reorder.column).as_ref() != Some(&reorder.keys)
        });
        if moved {
            self.cancel_rail_reorder();
        }
        self.close_sidebar_menu_if_gone();
    }

    /// The core's unread counts per space and outside spaces.
    pub(crate) fn on_space_unread(
        &mut self,
        spaces: Vec<SpaceUnreadView>,
        outside: UnreadCountsView,
    ) {
        self.spaces.set_unread(spaces, outside);
    }

    /// The core's list of spaces: kept, their avatars requested once, and a
    /// selected space that is gone resets the filter to Home.
    pub(crate) fn on_spaces(&mut self, spaces: Vec<SpaceView>) -> Task<Message> {
        self.spaces.set(spaces);
        self.clear_stale_rail_keys();
        let ids = self.avatars.take_unrequested(
            self.spaces
                .spaces()
                .iter()
                .filter_map(|space| space.avatar.clone()),
        );
        let fetch = if ids.is_empty() {
            Task::none()
        } else {
            self.send(Command::FetchAvatars(ids))
        };
        let reset = match &self.query.space {
            Some(id) if !self.spaces.contains(id) => self.select_space(None),
            _ => Task::none(),
        };
        let restore = self.restore_view();
        Task::batch([fetch, reset, restore])
    }
}

/// What the ghost tile shows: the row or header being dragged.
#[derive(Clone)]
pub enum GhostItem<'a> {
    /// A room's channel row.
    Room(&'a RoomSummary),
    /// A section's header.
    Header(Header<'a>),
}

/// The ghost tile that follows the pointer during a drag in the sectioned list.
/// `Debug` shows no name.
#[derive(Clone)]
pub struct Ghost<'a> {
    /// What is dragged.
    pub item: GhostItem<'a>,
    /// The pointer, in window coordinates.
    pub pointer: Point,
    /// Where the pointer was inside the item at the press, if it was seen.
    pub grab: Option<Point>,
}

impl std::fmt::Debug for Ghost<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.item {
            GhostItem::Room(_) => "Room",
            GhostItem::Header(_) => "Header",
        };
        f.debug_struct("Ghost")
            .field("item", &kind)
            .field("pointer", &self.pointer)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use twicecat_core::api::{FolderLayout, SidebarLabel, UserId};

    use super::*;

    fn space(id: &str, level: u32) -> SpaceView {
        SpaceView {
            id: RoomId::from(id),
            name: id.to_owned(),
            avatar: None,
            level,
            children: 0,
            rooms: Vec::new(),
        }
    }

    fn counts(notifications: u64, highlights: u64, unread: bool) -> UnreadCountsView {
        UnreadCountsView {
            notifications,
            highlights,
            unread,
        }
    }

    fn model(spaces: Vec<SpaceView>) -> SpacesModel {
        let mut model = SpacesModel::default();
        model.set(spaces);
        model
    }

    fn layout(rail: Vec<RailEntry>) -> SidebarLayout {
        let mut layout = SidebarLayout::new(UserId::from("@me:x"));
        layout.rail = rail;
        layout
    }

    fn sp(id: &str) -> RailEntry {
        RailEntry::Space(RoomId::from(id))
    }

    fn folder(id: u16, open: bool, spaces: &[&str]) -> RailEntry {
        RailEntry::Folder(FolderLayout {
            id,
            name: None,
            open,
            spaces: spaces.iter().map(|s| RoomId::from(*s)).collect(),
        })
    }

    /// A readable form of the rail: `a`, or `[1: a b]` for a folder.
    fn shape(items: &[RailItem<'_>]) -> Vec<String> {
        items
            .iter()
            .map(|item| match item {
                RailItem::Space(s) => s.id.as_str().to_owned(),
                RailItem::Folder(f) => {
                    let ids: Vec<&str> = f.spaces.iter().map(|s| s.id.as_str()).collect();
                    format!("[{}: {}]", f.id, ids.join(" "))
                }
            })
            .collect()
    }

    fn sample() -> SpacesModel {
        model(vec![
            space("!a:x", 0),
            space("!a1:x", 1),
            space("!b:x", 0),
            space("!b1:x", 1),
            space("!c:x", 0),
        ])
    }

    #[test]
    fn entries_are_the_top_level_spaces_only() {
        let m = sample();
        let ids: Vec<&str> = m.entries().iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["!a:x", "!b:x", "!c:x"]);
        assert!(SpacesModel::default().entries().is_empty());
    }

    fn with_rooms(id: &str, level: u32, rooms: &[&str]) -> SpaceView {
        SpaceView {
            rooms: rooms.iter().map(|r| RoomId::from(*r)).collect(),
            ..space(id, level)
        }
    }

    fn ids(spaces: &[&SpaceView]) -> Vec<String> {
        spaces.iter().map(|s| s.id.as_str().to_owned()).collect()
    }

    #[test]
    fn sub_spaces_are_the_level_1_spaces_in_the_parents_rooms_in_the_cores_order() {
        let m = model(vec![
            with_rooms("!p:x", 0, &["!r1:x", "!s2:x", "!s1:x", "!r2:x"]),
            // The core's order, not the parent's list, decides.
            with_rooms("!s1:x", 1, &["!r3:x", "!s1:x"]),
            with_rooms("!s2:x", 1, &["!s2:x"]),
            // Level 1, but held by another parent only.
            with_rooms("!other:x", 1, &["!other:x"]),
            with_rooms("!q:x", 0, &["!other:x"]),
        ]);
        assert_eq!(
            ids(&m.sub_spaces(&RoomId::from("!p:x"))),
            ["!s1:x", "!s2:x"]
        );
        assert_eq!(ids(&m.sub_spaces(&RoomId::from("!q:x"))), ["!other:x"]);
    }

    #[test]
    fn a_room_a_level_2_space_and_a_duplicate_are_not_sub_spaces() {
        let m = model(vec![
            with_rooms("!p:x", 0, &["!room:x", "!deep:x", "!s:x", "!s:x", "!p:x"]),
            with_rooms("!s:x", 1, &["!deep:x", "!s:x"]),
            // Listed twice by the core: once.
            with_rooms("!s:x", 1, &["!s:x"]),
            with_rooms("!deep:x", 2, &["!deep:x"]),
        ]);
        assert_eq!(ids(&m.sub_spaces(&RoomId::from("!p:x"))), ["!s:x"]);
    }

    #[test]
    fn a_level_1_space_a_missing_space_and_a_space_with_none_have_no_sub_spaces() {
        let m = model(vec![
            with_rooms("!p:x", 0, &["!s:x"]),
            with_rooms("!s:x", 1, &["!t:x"]),
            with_rooms("!t:x", 1, &["!t:x"]),
            with_rooms("!lonely:x", 0, &["!room:x"]),
        ]);
        assert!(m.sub_spaces(&RoomId::from("!s:x")).is_empty(), "level 1");
        assert!(m.sub_spaces(&RoomId::from("!nope:x")).is_empty());
        assert!(m.sub_spaces(&RoomId::from("!lonely:x")).is_empty());
        assert!(
            SpacesModel::default()
                .sub_spaces(&RoomId::from("!p:x"))
                .is_empty()
        );
    }

    #[test]
    fn the_tree_follows_a_new_list_and_is_forgotten_at_logout() {
        let mut m = model(vec![
            with_rooms("!p:x", 0, &["!s:x"]),
            with_rooms("!s:x", 1, &["!s:x"]),
        ]);
        let p = RoomId::from("!p:x");
        assert_eq!(m.sub_spaces(&p).len(), 1);
        m.set(vec![with_rooms("!p:x", 0, &["!room:x"])]);
        assert!(m.sub_spaces(&p).is_empty(), "the sub-space left");
        m.set(vec![
            with_rooms("!p:x", 0, &["!s:x"]),
            with_rooms("!s:x", 1, &["!s:x"]),
        ]);
        assert_eq!(m.sub_spaces(&p).len(), 1);
        m.clear();
        assert!(m.sub_spaces(&p).is_empty());
    }

    #[test]
    fn a_parent_holds_the_selected_item_only_through_its_sub_spaces() {
        let m = model(vec![
            with_rooms("!p:x", 0, &["!s:x"]),
            with_rooms("!s:x", 1, &["!s:x"]),
        ]);
        let p = RoomId::from("!p:x");
        let s = RoomId::from("!s:x");
        assert!(m.holds_selected(&p, Some(&s)));
        assert!(
            !m.holds_selected(&p, Some(&p)),
            "itself is not its sub-space"
        );
        assert!(!m.holds_selected(&p, None), "Home");
        assert!(!m.holds_selected(&p, Some(&RoomId::from("!x:x"))));
    }

    #[test]
    fn without_a_layout_the_rail_is_the_cores_order() {
        let m = sample();
        assert_eq!(shape(&m.rail_items(None)), ["!a:x", "!b:x", "!c:x"]);
        let empty = layout(Vec::new());
        assert_eq!(shape(&m.rail_items(Some(&empty))), ["!a:x", "!b:x", "!c:x"]);
    }

    #[test]
    fn the_layout_order_is_kept() {
        let m = sample();
        let l = layout(vec![sp("!c:x"), sp("!a:x"), sp("!b:x")]);
        assert_eq!(shape(&m.rail_items(Some(&l))), ["!c:x", "!a:x", "!b:x"]);
    }

    #[test]
    fn a_new_space_is_appended_in_the_cores_order() {
        let m = sample();
        let l = layout(vec![sp("!c:x")]);
        assert_eq!(shape(&m.rail_items(Some(&l))), ["!c:x", "!a:x", "!b:x"]);
    }

    #[test]
    fn a_space_in_a_folder_is_not_repeated_at_top_level() {
        let m = sample();
        let l = layout(vec![sp("!c:x"), folder(7, false, &["!a:x", "!b:x"])]);
        assert_eq!(shape(&m.rail_items(Some(&l))), ["!c:x", "[7: !a:x !b:x]"]);
        // Named twice in the layout: drawn once, where it is named first.
        let twice = layout(vec![folder(1, true, &["!a:x"]), sp("!a:x")]);
        assert_eq!(
            shape(&m.rail_items(Some(&twice))),
            ["[1: !a:x]", "!b:x", "!c:x"]
        );
    }

    #[test]
    fn a_stored_space_the_core_does_not_report_is_skipped_but_stays_stored() {
        let m = sample();
        let l = layout(vec![
            sp("!gone:x"),
            folder(2, false, &["!gone2:x", "!b:x"]),
            folder(3, false, &["!gone3:x"]),
            // A sub-space is not on the rail.
            sp("!a1:x"),
        ]);
        let before = l.clone();
        assert_eq!(
            shape(&m.rail_items(Some(&l))),
            ["[2: !b:x]", "!a:x", "!c:x"]
        );
        assert_eq!(l, before, "the layout is not touched");
    }

    #[test]
    fn folder_name_and_open_state_come_from_the_layout() {
        let m = sample();
        let mut l = layout(vec![folder(4, true, &["!a:x"])]);
        let RailEntry::Folder(f) = &mut l.rail[0] else {
            unreachable!()
        };
        f.name = Some(SidebarLabel::try_from("Work").expect("a valid name"));
        let items = m.rail_items(Some(&l));
        let RailItem::Folder(f) = &items[0] else {
            panic!("a folder")
        };
        assert_eq!((f.id, f.name, f.open), (4, Some("Work"), true));
    }

    #[test]
    fn counts_sum_with_saturation_and_a_folder_is_the_sum_of_its_spaces() {
        let mut m = sample();
        m.set_unread(
            vec![
                SpaceUnreadView {
                    space: RoomId::from("!a:x"),
                    counts: counts(2, 1, true),
                },
                SpaceUnreadView {
                    space: RoomId::from("!b:x"),
                    counts: counts(5, 0, true),
                },
                SpaceUnreadView {
                    space: RoomId::from("!c:x"),
                    counts: counts(100, 9, true),
                },
            ],
            counts(0, 0, false),
        );
        let l = layout(vec![folder(1, false, &["!a:x", "!b:x"])]);
        let items = m.rail_items(Some(&l));
        let RailItem::Folder(f) = &items[0] else {
            panic!("a folder")
        };
        assert_eq!(m.folder_counts(f), counts(7, 1, true));
        assert_eq!(
            sum_counts([counts(u64::MAX, u64::MAX, false), counts(1, 1, false)]),
            counts(u64::MAX, u64::MAX, false)
        );
        assert_eq!(sum_counts([]), UnreadCountsView::default());
        assert_eq!(m.unread_for(&RoomId::from("!nope:x")), counts(0, 0, false));
    }

    #[test]
    fn debug_of_the_rail_items_shows_no_room_id_and_no_folder_name() {
        let m = model(vec![space("!secretroom:x", 0), space("!secretroom2:x", 0)]);
        let mut l = layout(vec![folder(1, true, &["!secretroom:x", "!secretroom2:x"])]);
        let RailEntry::Folder(f) = &mut l.rail[0] else {
            unreachable!()
        };
        f.name = Some(SidebarLabel::try_from("Payroll").expect("a valid name"));
        let items = m.rail_items(Some(&l));
        let mut texts = vec![format!("{items:?}")];
        if let RailItem::Folder(f) = &items[0] {
            texts.push(format!("{f:?}"));
        }
        texts.push(format!("{:?}", RailItem::Space(&m.spaces()[0])));
        for text in texts {
            for secret in ["!secretroom", "@a:x", "Payroll"] {
                assert!(!text.contains(secret), "{secret} is in {text}");
            }
        }
    }

    #[test]
    fn the_model_caps_and_answers_membership() {
        let mut model = SpacesModel::default();
        assert!(model.is_empty());
        model.set(
            (0..MAX_SPACES + 5)
                .map(|i| space(&format!("!s{i}:x"), 0))
                .collect(),
        );
        assert_eq!(model.spaces().len(), MAX_SPACES);
        assert!(model.contains(&RoomId::from("!s0:x")));
        assert!(!model.contains(&RoomId::from("!s9999:x")));
        model.set_unread(
            vec![SpaceUnreadView {
                space: RoomId::from("!s0:x"),
                counts: counts(1, 0, true),
            }],
            counts(3, 0, true),
        );
        let dbg = format!("{model:?}");
        assert!(!dbg.contains("!s0:x"), "{dbg}");
        assert!(!format!("{:?}", RailKey::Space(RoomId::from("!s0:x"))).contains("!s0:x"));
        model.clear();
        assert!(model.is_empty());
        assert_eq!(model.outside(), UnreadCountsView::default());
        assert_eq!(
            model.unread_for(&RoomId::from("!s0:x")),
            counts(0, 0, false)
        );
    }
}
