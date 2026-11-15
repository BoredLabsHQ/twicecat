//! The room list of a space as collapsible sections (sidebar plan, decision 4,
//! Task 8): where each room goes, the rolled-up badge of a collapsed section,
//! and the windowing of a list whose lines differ in height.
//!
//! Sections show only with a space selected, the All tab, a blank search and
//! the expanded list ([`App::sections_shown`]). Anything else keeps the flat
//! list of `room_list`.
//!
//! Room names and IDs belong to people: nothing here logs them, and no
//! `Debug` of a type here shows one.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

use twicecat_core::api::{
    CallMemberView, RoomId, RoomListChoice, RoomListTab, RoomNotificationModeView, RoomSummary,
    SectionKey, SpaceLayout, SpaceView,
};

use crate::calls::CallModel;
use crate::room_list::{Badge, Sorting, clamp_window, order, row_height, voice_badge};
use crate::state::App;
use crate::ui::tokens::Density;

#[cfg(test)]
thread_local! {
    /// How many times [`build`] ran on this thread (tests count rebuilds).
    static BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many times [`build`] ran on this thread.
#[cfg(test)]
pub(crate) fn build_runs() -> usize {
    BUILDS.with(std::cell::Cell::get)
}

/// One line of the sectioned list. `Debug` shows no room: a room's ID and name
/// belong to people and must not reach a log.
#[derive(Clone, PartialEq)]
pub enum Line<'a> {
    /// The header of a section.
    Header(Header<'a>),
    /// A room.
    Room(&'a RoomSummary),
    /// The hint under an empty custom section.
    EmptyHint(SectionKey),
    /// A person in a voice room's call, under the room.
    Participant(Person<'a>),
    /// "and N more" after [`MAX_PEOPLE_SHOWN`] people of a voice room.
    More(More<'a>),
}

impl std::fmt::Debug for Line<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Header(header) => f.debug_tuple("Header").field(header).finish(),
            Self::Room(_) => f.write_str("Room(..)"),
            Self::EmptyHint(key) => f.debug_tuple("EmptyHint").field(key).finish(),
            Self::Participant(person) => f.debug_tuple("Participant").field(person).finish(),
            Self::More(more) => f.debug_tuple("More").field(more).finish(),
        }
    }
}

/// How many people of a voice room's call are drawn under it; the rest are
/// "and N more".
pub const MAX_PEOPLE_SHOWN: usize = 8;

/// One person in a voice room's call. `Debug` shows only the marks: a user
/// ID, a name and a room ID belong to people.
#[derive(Clone, PartialEq)]
pub struct Person<'a> {
    /// The voice room.
    pub room: &'a RoomId,
    /// Who: the core's view.
    pub member: &'a CallMemberView,
    /// The name to draw: [`crate::calls::member_names`] of the room's people
    /// (the user ID is a remote server's raw string and the name may be
    /// missing or shared, so it is worked out once for the whole room).
    pub name: String,
    /// Speaking now: known only for the call this device is in.
    pub speaking: bool,
    /// Muted: known only for the call this device is in.
    pub muted: bool,
}

impl std::fmt::Debug for Person<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Person")
            .field("speaking", &self.speaking)
            .field("muted", &self.muted)
            .finish_non_exhaustive()
    }
}

/// The "and N more" line of a voice room. `Debug` shows only the number.
#[derive(Clone, PartialEq)]
pub struct More<'a> {
    /// The voice room (a click selects it).
    pub room: &'a RoomId,
    /// How many people are not drawn.
    pub hidden: usize,
}

impl std::fmt::Debug for More<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("More")
            .field("hidden", &self.hidden)
            .finish_non_exhaustive()
    }
}

/// Height of a person's line under a voice room, comfortable.
pub const PERSON_HEIGHT: f32 = 28.0;
/// Height of a person's line under a voice room, compact.
pub const COMPACT_PERSON_HEIGHT: f32 = 26.0;

/// The header of a section. `Debug` of the key hides a room ID; the name is
/// not shown by `Debug`.
#[derive(Clone, PartialEq)]
pub struct Header<'a> {
    /// Which section.
    pub key: SectionKey,
    /// The name as stored or reported: sub-space names come from other
    /// people and pass through `text_safety` where they are drawn.
    pub name: &'a str,
    /// Whether the section is collapsed.
    pub collapsed: bool,
    /// The rolled-up badge of its rooms: [`Badge::None`] unless collapsed.
    pub badge: Badge,
}

impl std::fmt::Debug for Header<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Header")
            .field("key", &self.key)
            .field("collapsed", &self.collapsed)
            .field("badge", &self.badge)
            .finish()
    }
}

/// Height of a section header, comfortable.
pub const HEADER_HEIGHT: f32 = 32.0;
/// Height of a section header, compact.
pub const COMPACT_HEADER_HEIGHT: f32 = 28.0;
/// Height of the hint under an empty custom section, comfortable.
pub const HINT_HEIGHT: f32 = 36.0;
/// Height of the hint under an empty custom section, compact.
pub const COMPACT_HINT_HEIGHT: f32 = 32.0;

/// The words under an empty custom section.
pub const EMPTY_HINT: &str = "Drop or move rooms here";

/// One section while the rooms are being placed.
struct Bucket<'a> {
    key: SectionKey,
    name: &'a str,
    /// Hidden while empty (every section but the user's own).
    hide_empty: bool,
    /// The rooms with their sort key: the order the user gave in Starred and
    /// custom sections, else the order of the list.
    rooms: Vec<(usize, &'a RoomSummary)>,
}

/// Places the rooms of `space` in sections and returns the lines to draw
/// (decision 4).
///
/// `spaces` are all the spaces the core reports: the level 1 ones whose ID is
/// among `space.rooms` are its sub-spaces. `rooms` are the rooms of the list
/// in its own (recent activity) order. Each room is placed once, in the first
/// of: Starred, Favourites (the room's `m.favourite` tag), a custom section,
/// Low priority (`m.lowpriority`), its sub-space's section, Direct messages
/// (when it is a DM), Channels. The default order of the sections is Starred,
/// Favourites, Channels, the sub-spaces, the custom sections, Direct messages
/// and Low priority (plan L4). Spaces are never rows, and the selected
/// space's own ID, which a level 1 space lists among its rooms, is left out.
/// Every section but a custom one is hidden while empty; an empty custom one
/// is followed by [`Line::EmptyHint`]. A collapsed section keeps its header,
/// with the badge of its rooms, and drops its rooms (but see [`build_open`]).
/// Inside a section the user does not order, text rooms come first and voice
/// rooms after them, each group in the chosen order (plan C3).
pub fn build<'a>(
    space: &'a SpaceView,
    spaces: &'a [SpaceView],
    rooms: &'a [RoomSummary],
    layout: Option<&'a SpaceLayout>,
) -> Vec<Line<'a>> {
    build_with(space, spaces, rooms, layout, None)
}

/// [`build`] that also lists, under each voice room (`is_call_room`) of an
/// expanded section, the people `calls` knows are in its call: one
/// [`Line::Participant`] each, at most [`MAX_PEOPLE_SHOWN`], then a
/// [`Line::More`]. Their marks come from `calls` for the call this device is
/// in only. A collapsed section hides them with its rooms. `None` draws no
/// people (the drop planner's defaults).
pub fn build_with<'a>(
    space: &'a SpaceView,
    spaces: &'a [SpaceView],
    rooms: &'a [RoomSummary],
    layout: Option<&'a SpaceLayout>,
    calls: Option<&'a CallModel>,
) -> Vec<Line<'a>> {
    build_sorted(space, spaces, rooms, layout, calls, Sorting::default())
}

/// [`build_with`] with the rooms of every section the user does not order
/// (all but Starred and the custom ones) put in `sorting`'s order (plan L2).
/// Starred and the custom sections keep the user's order under every setting.
pub fn build_sorted<'a>(
    space: &'a SpaceView,
    spaces: &'a [SpaceView],
    rooms: &'a [RoomSummary],
    layout: Option<&'a SpaceLayout>,
    calls: Option<&'a CallModel>,
    sorting: Sorting,
) -> Vec<Line<'a>> {
    build_open(space, spaces, rooms, layout, calls, sorting, None)
}

/// [`build_sorted`] for the list as it is drawn: `selected` is the open
/// room. A collapsed section still draws it, and the voice room of the call
/// this device is in with that room's people, after its header; any other room
/// of the section stays hidden and only counts in the header's badge (plan
/// C4).
pub fn build_open<'a>(
    space: &'a SpaceView,
    spaces: &'a [SpaceView],
    rooms: &'a [RoomSummary],
    layout: Option<&'a SpaceLayout>,
    calls: Option<&'a CallModel>,
    sorting: Sorting,
    selected: Option<&'a RoomId>,
) -> Vec<Line<'a>> {
    let placing = Placing {
        tags: true,
        selected,
    };
    build_placed(space, spaces, rooms, layout, calls, sorting, placing)
}

/// The lines with no layout and no tags: every room in the section that is
/// its default one (Channels, a sub-space or Direct messages), whatever
/// Matrix tags it carries. A drop uses it to tell which section is a room's
/// own default one, also for a room that is drawn in Favourites or Low
/// priority now.
pub fn build_defaults<'a>(
    space: &'a SpaceView,
    spaces: &'a [SpaceView],
    rooms: &'a [RoomSummary],
) -> Vec<Line<'a>> {
    let placing = Placing {
        tags: false,
        selected: None,
    };
    build_placed(
        space,
        spaces,
        rooms,
        None,
        None,
        Sorting::default(),
        placing,
    )
}

/// How [`build_placed`] places and draws the rooms.
#[derive(Clone, Copy)]
struct Placing<'a> {
    /// Whether a room's Favourite and Low priority tags place it (always,
    /// except for [`build_defaults`]).
    tags: bool,
    /// The open room, which a collapsed section keeps drawn.
    selected: Option<&'a RoomId>,
}

/// [`build_sorted`] with the choices of `placing`.
fn build_placed<'a>(
    space: &'a SpaceView,
    spaces: &'a [SpaceView],
    rooms: &'a [RoomSummary],
    layout: Option<&'a SpaceLayout>,
    calls: Option<&'a CallModel>,
    sorting: Sorting,
    placing: Placing<'a>,
) -> Vec<Line<'a>> {
    let Placing { tags, selected } = placing;
    #[cfg(test)]
    BUILDS.with(|n| n.set(n.get() + 1));
    let own = &space.id;
    // The sub-spaces: level 1 children of a level 0 space. A level 1 space
    // has none, whatever its list holds.
    let held: HashSet<&RoomId> = space.rooms.iter().collect();
    let mut seen_subs: HashSet<&RoomId> = HashSet::new();
    let subs: Vec<&SpaceView> = if space.level == 0 {
        spaces
            .iter()
            .filter(|s| {
                s.level == 1 && s.id != *own && held.contains(&s.id) && seen_subs.insert(&s.id)
            })
            .collect()
    } else {
        Vec::new()
    };
    // Which sub-space holds a room (the first one, in the core's order). A
    // sub-space's own ID and the selected space's are not rooms.
    let mut sub_of: HashMap<&RoomId, &RoomId> = HashMap::new();
    for sub in &subs {
        for id in &sub.rooms {
            if *id != sub.id && *id != *own {
                sub_of.entry(id).or_insert(&sub.id);
            }
        }
    }
    let mut not_rows: HashSet<&RoomId> = subs.iter().map(|s| &s.id).collect();
    not_rows.insert(own);

    let mut starred_at: HashMap<&RoomId, usize> = HashMap::new();
    let mut custom_at: HashMap<&RoomId, (u16, usize)> = HashMap::new();
    if let Some(layout) = layout {
        for (n, id) in layout.starred.iter().enumerate() {
            starred_at.entry(id).or_insert(n);
        }
        for custom in &layout.custom {
            for (n, id) in custom.rooms.iter().enumerate() {
                custom_at.entry(id).or_insert((custom.id, n));
            }
        }
    }

    // The sections in their default order.
    let mut buckets: Vec<Bucket<'a>> = Vec::new();
    let mut slot: HashMap<SectionKey, usize> = HashMap::new();
    let mut add = |key: SectionKey, name: &'a str, hide_empty: bool| {
        if !slot.contains_key(&key) {
            slot.insert(key.clone(), buckets.len());
            buckets.push(Bucket {
                key,
                name,
                hide_empty,
                rooms: Vec::new(),
            });
        }
    };
    add(SectionKey::Starred, "Starred", true);
    add(SectionKey::Favourites, "Favourites", true);
    add(SectionKey::Channels, "Channels", true);
    for sub in &subs {
        add(SectionKey::SubSpace(sub.id.clone()), &sub.name, true);
    }
    for custom in layout.map_or(&[][..], |l| l.custom.as_slice()) {
        add(SectionKey::Custom(custom.id), custom.name.as_str(), false);
    }
    add(SectionKey::Direct, "Direct messages", true);
    add(SectionKey::LowPriority, "Low priority", true);

    let mut placed: HashSet<&RoomId> = HashSet::new();
    for (n, room) in rooms.iter().enumerate() {
        if room.is_space || not_rows.contains(&room.id) || !placed.insert(&room.id) {
            continue;
        }
        let (key, at) = if let Some(&at) = starred_at.get(&room.id) {
            (SectionKey::Starred, at)
        } else if tags && room.is_favourite {
            (SectionKey::Favourites, n)
        } else if let Some(&(id, at)) = custom_at.get(&room.id) {
            (SectionKey::Custom(id), at)
        } else if tags && room.is_low_priority {
            (SectionKey::LowPriority, n)
        } else if let Some(&sub) = sub_of.get(&room.id) {
            (SectionKey::SubSpace(sub.clone()), n)
        } else if room.is_dm {
            (SectionKey::Direct, n)
        } else {
            (SectionKey::Channels, n)
        };
        if let Some(&i) = slot.get(&key) {
            buckets[i].rooms.push((at, room));
        }
    }
    for bucket in &mut buckets {
        if matches!(bucket.key, SectionKey::Starred | SectionKey::Custom(_)) {
            // The user's own order, whatever the sort setting.
            bucket.rooms.sort_by_key(|(at, _)| *at);
        } else if !sorting.is_default() {
            // Low priority last is a rule of the flat list only (plan L3).
            let refs: Vec<&RoomSummary> = bucket.rooms.iter().map(|(_, room)| *room).collect();
            let picked = order(&refs, sorting.by, sorting.unread_first, false);
            bucket.rooms = picked.into_iter().map(|i| bucket.rooms[i]).collect();
        }
        if !matches!(bucket.key, SectionKey::Starred | SectionKey::Custom(_)) {
            // Voice after text (plan C3): a stable partition, so each group
            // keeps the order the sort gave it.
            let (voice, text): (Vec<_>, Vec<_>) =
                bucket.rooms.iter().partition(|(_, room)| voice_badge(room));
            bucket.rooms = text.into_iter().chain(voice).copied().collect();
        }
    }

    let collapsed: &[SectionKey] = layout.map_or(&[], |l| l.collapsed.as_slice());
    let call_room = calls
        .and_then(|calls| calls.call.as_ref())
        .map(|call| &call.room_id);
    let mut lines = Vec::new();
    for key in section_order(&buckets, layout) {
        let bucket = &buckets[slot[&key]];
        let is_collapsed = collapsed.contains(&key);
        let badge = if is_collapsed {
            let all: Vec<&RoomSummary> = bucket.rooms.iter().map(|(_, r)| *r).collect();
            section_badge(&all)
        } else {
            Badge::None
        };
        lines.push(Line::Header(Header {
            key: key.clone(),
            name: bucket.name,
            collapsed: is_collapsed,
            badge,
        }));
        if is_collapsed {
            // It keeps the open room and the room of this device's call,
            // with its people (plan C4); the badge above counts them all.
            for (_, room) in &bucket.rooms {
                if selected == Some(&room.id) || call_room == Some(&room.id) {
                    push_room(&mut lines, room, calls);
                }
            }
            continue;
        }
        if bucket.rooms.is_empty() {
            lines.push(Line::EmptyHint(key));
        } else {
            for (_, room) in &bucket.rooms {
                push_room(&mut lines, room, calls);
            }
        }
    }
    lines
}

/// The line of `room`, and under a voice room the people of its call.
fn push_room<'a>(lines: &mut Vec<Line<'a>>, room: &'a RoomSummary, calls: Option<&'a CallModel>) {
    lines.push(Line::Room(room));
    if let Some(calls) = calls.filter(|_| room.is_call_room) {
        push_people(lines, room, calls);
    }
}

/// The people of `room`'s call under its line: at most [`MAX_PEOPLE_SHOWN`]
/// and then "and N more".
fn push_people<'a>(lines: &mut Vec<Line<'a>>, room: &'a RoomSummary, calls: &'a CallModel) {
    let Some(members) = calls.members.get(&room.id) else {
        return;
    };
    // Over everyone in the call, not only those drawn, so a name does not
    // change when someone behind "and N more" joins or leaves.
    let names = crate::calls::member_names(members);
    for (member, name) in members.iter().zip(names).take(MAX_PEOPLE_SHOWN) {
        let (speaking, muted) = calls.member_marks(&room.id, &member.user_id);
        lines.push(Line::Participant(Person {
            room: &room.id,
            member,
            name,
            speaking,
            muted,
        }));
    }
    if members.len() > MAX_PEOPLE_SHOWN {
        lines.push(Line::More(More {
            room: &room.id,
            hidden: members.len() - MAX_PEOPLE_SHOWN,
        }));
    }
}

/// The sections to draw, in order: the stored order for the keys it names
/// (unknown ones are skipped, and kept in the file), and each drawn section it
/// does not name right after the section that precedes it by default.
fn section_order(buckets: &[Bucket<'_>], layout: Option<&SpaceLayout>) -> Vec<SectionKey> {
    let defaults: Vec<&SectionKey> = buckets
        .iter()
        .filter(|b| !(b.hide_empty && b.rooms.is_empty()))
        .map(|b| &b.key)
        .collect();
    let mut order: Vec<SectionKey> = Vec::new();
    for key in layout.map_or(&[][..], |l| l.order.as_slice()) {
        if defaults.contains(&key) && !order.contains(key) {
            order.push(key.clone());
        }
    }
    for (i, key) in defaults.iter().enumerate() {
        if order.contains(key) {
            continue;
        }
        let after = defaults[..i]
            .iter()
            .rev()
            .find_map(|earlier| order.iter().position(|k| k == *earlier));
        order.insert(after.map_or(0, |p| p + 1), (*key).clone());
    }
    order
}

/// The badge of a collapsed section over its rooms: `badge_for`'s priority
/// (highlights, then notifications, then a dot) over the sums. Muted rooms
/// are left out, as the core leaves them out of a space's totals; the sums
/// saturate.
pub fn section_badge(rooms: &[&RoomSummary]) -> Badge {
    let (mut notifications, mut highlights, mut unread) = (0u64, 0u64, false);
    for room in rooms {
        if room.notification_mode == Some(RoomNotificationModeView::Mute) {
            continue;
        }
        notifications = notifications.saturating_add(room.unread_notifications);
        highlights = highlights.saturating_add(room.highlights);
        unread |= room.unread_messages > 0 || room.marked_unread;
    }
    if highlights > 0 {
        Badge::Highlight(highlights)
    } else if notifications > 0 {
        Badge::Count(notifications)
    } else if unread {
        Badge::Dot
    } else {
        Badge::None
    }
}

/// The sections the lines draw, in order: what a move up or down swaps among.
pub fn drawn_sections(lines: &[Line<'_>]) -> Vec<SectionKey> {
    lines
        .iter()
        .filter_map(|line| match line {
            Line::Header(header) => Some(header.key.clone()),
            Line::Room(_) | Line::EmptyHint(_) | Line::Participant(_) | Line::More(_) => None,
        })
        .collect()
}

/// Where a room is drawn.
#[derive(Clone, PartialEq, Eq)]
pub struct Spot {
    /// The section that holds it.
    pub section: SectionKey,
    /// The rooms drawn in that section, in order.
    pub rooms: Vec<RoomId>,
    /// Its place among them.
    pub index: usize,
}

impl std::fmt::Debug for Spot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Spot")
            .field("section", &self.section)
            .field("rooms", &self.rooms.len())
            .field("index", &self.index)
            .finish()
    }
}

/// Where `room` is drawn, or `None` when no line shows it (its section is
/// collapsed, or it is not in the list).
pub fn spot_of(lines: &[Line<'_>], room: &RoomId) -> Option<Spot> {
    let mut section: Option<&SectionKey> = None;
    let mut rooms: Vec<RoomId> = Vec::new();
    let mut found: Option<(SectionKey, usize)> = None;
    for line in lines {
        match line {
            Line::Header(header) => {
                if found.is_some() {
                    break;
                }
                section = Some(&header.key);
                rooms.clear();
            }
            Line::Room(r) => {
                if r.id == *room
                    && let Some(key) = section
                {
                    found = Some((key.clone(), rooms.len()));
                }
                rooms.push(r.id.clone());
            }
            Line::EmptyHint(_) | Line::Participant(_) | Line::More(_) => {}
        }
    }
    let (section, index) = found?;
    Some(Spot {
        section,
        rooms,
        index,
    })
}

/// The height of `line` in `density`.
pub fn line_height(line: &Line<'_>, density: Density) -> f32 {
    match (line, density) {
        (Line::Header(_), Density::Comfortable) => HEADER_HEIGHT,
        (Line::Header(_), Density::Compact) => COMPACT_HEADER_HEIGHT,
        (Line::Room(_), density) => row_height(density),
        (Line::EmptyHint(_), Density::Comfortable) => HINT_HEIGHT,
        (Line::EmptyHint(_), Density::Compact) => COMPACT_HINT_HEIGHT,
        (Line::Participant(_) | Line::More(_), Density::Comfortable) => PERSON_HEIGHT,
        (Line::Participant(_) | Line::More(_), Density::Compact) => COMPACT_PERSON_HEIGHT,
    }
}

/// The top of every line and, last, the total height: `lines.len() + 1`
/// prefix sums.
pub fn line_offsets(lines: &[Line<'_>], density: Density) -> Vec<f32> {
    let mut offsets = Vec::with_capacity(lines.len() + 1);
    let mut top = 0.0_f32;
    offsets.push(top);
    for line in lines {
        top += line_height(line, density);
        offsets.push(top);
    }
    offsets
}

/// The lines to build for a scrollable showing `viewport_h` pixels at
/// `offset_y`: those that cross the viewport, plus `overscan` on each side.
/// `offsets` are the prefix sums of [`line_offsets`]; two binary searches
/// find the ends. NaN and negative inputs count as zero and a stale offset
/// is clamped to the last page, as [`crate::room_list::visible_range`] does
/// (both use the same clamping), so the range never leaves the lines.
pub fn visible_lines(
    offsets: &[f32],
    offset_y: f32,
    viewport_h: f32,
    overscan: usize,
) -> Range<usize> {
    let Some(len) = offsets.len().checked_sub(1).filter(|len| *len > 0) else {
        return 0..0;
    };
    let (offset, viewport) = clamp_window(offset_y, viewport_h, offsets[len]);
    // The first line whose bottom is below the top of the viewport, and the
    // first whose top is at or below its bottom.
    let first = offsets[1..].partition_point(|bottom| *bottom <= offset);
    let last = offsets[..len].partition_point(|top| *top < offset + viewport);
    let start = first.saturating_sub(overscan).min(len);
    let end = last.saturating_add(overscan).min(len).max(start);
    start..end
}

/// The heights of the spacers above and below the built `range` of lines, so
/// the scrollable's content stays as tall as all the lines.
pub fn line_spacers(offsets: &[f32], range: &Range<usize>) -> (f32, f32) {
    let total = offsets.last().copied().unwrap_or(0.0);
    let top = offsets.get(range.start).copied().unwrap_or(0.0);
    let end = offsets.get(range.end).copied().unwrap_or(total);
    (top, (total - end).max(0.0))
}

/// The load-all rule: in sectioned mode every room of the space is needed to
/// place it, so another page is asked for whenever the core says there are
/// more rooms than are loaded (the caller sends it once per loaded length).
pub fn must_load_all(len: usize, total: Option<u32>) -> bool {
    total.is_some_and(|total| usize::try_from(total).unwrap_or(usize::MAX) > len)
}

impl App {
    /// Whether the room list shows sections now: a space is selected, the tab
    /// is All, the search is blank and the list is expanded (not the rail).
    /// Anything else keeps the flat list.
    pub fn sections_shown(&self) -> bool {
        self.query.tab == RoomListTab::All
            && self.query.search.trim().is_empty()
            && self.pane_layout().list == RoomListChoice::Expanded
            && self
                .query
                .space
                .as_ref()
                .is_some_and(|id| self.spaces.contains(id))
    }

    /// The lines of the sectioned list; `None` while the list is flat.
    pub fn section_lines(&self) -> Option<Vec<Line<'_>>> {
        if !self.sections_shown() {
            return None;
        }
        let id = self.query.space.as_ref()?;
        let space = self.spaces.spaces().iter().find(|s| s.id == *id)?;
        let stored = self
            .sidebar
            .layout()
            .and_then(|layout| layout.spaces.iter().find(|s| s.space == *id));
        Some(build_open(
            space,
            self.spaces.spaces(),
            &self.rooms,
            stored,
            Some(&self.calls),
            self.sorting(),
            self.selected.as_ref(),
        ))
    }

    /// The lines of the sectioned list with no layout and no tags: every room
    /// in its default section, and every section open. A drop uses it to tell which
    /// section is a room's own default one; `None` while the list is flat.
    pub(crate) fn default_section_lines(&self) -> Option<Vec<Line<'_>>> {
        if !self.sections_shown() {
            return None;
        }
        let id = self.query.space.as_ref()?;
        let space = self.spaces.spaces().iter().find(|s| s.id == *id)?;
        Some(build_defaults(space, self.spaces.spaces(), &self.rooms))
    }

    /// The rooms as the list draws them, in order: the list itself while it
    /// is flat, else the room lines of the sections, so a room in a
    /// collapsed section is not among them. The room-stepping keys walk this.
    pub(crate) fn rooms_as_drawn(&self) -> Cow<'_, [RoomSummary]> {
        match self.section_lines() {
            None => match self.flat_order() {
                None => Cow::Borrowed(&self.rooms),
                Some(order) => {
                    Cow::Owned(order.into_iter().map(|i| self.rooms[i].clone()).collect())
                }
            },
            Some(lines) => Cow::Owned(
                lines
                    .into_iter()
                    .filter_map(|line| match line {
                        Line::Room(room) => Some(room.clone()),
                        Line::Header(_)
                        | Line::EmptyHint(_)
                        | Line::Participant(_)
                        | Line::More(_) => None,
                    })
                    .collect(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tokio::sync::mpsc;
    use twicecat_core::api::{
        Command, CoreEvent, CustomSection, DeviceId, ListDiff, RoomId, RoomListChoice, RoomListTab,
        RoomNotificationModeView, RoomSummary, SectionKey, SidebarLabel, SidebarLayout,
        SpaceLayout, SpaceView, UserId,
    };

    use super::*;
    use crate::bridge::EventsSlot;
    use crate::room_list::{Badge, OVERSCAN_ROWS, visible_range};
    use crate::sidebar::Op;
    use crate::sidebar_store::{MemorySidebar, SidebarAnswer, SidebarLoad};
    use crate::spaces::SpacesMessage;
    use crate::state::{Message, Startup};
    use crate::ui::tokens::Density;

    fn rid(n: &str) -> RoomId {
        RoomId::from(format!("!{n}:x"))
    }

    fn room(n: &str) -> RoomSummary {
        RoomSummary::new(rid(n), n.to_owned())
    }

    fn dm(n: &str) -> RoomSummary {
        let mut r = room(n);
        r.is_dm = true;
        r
    }

    fn space_row(n: &str) -> RoomSummary {
        let mut r = room(n);
        r.is_space = true;
        r
    }

    fn view(n: &str, level: u32, rooms: &[&str]) -> SpaceView {
        SpaceView {
            id: rid(n),
            name: n.to_owned(),
            avatar: None,
            level,
            children: rooms.len() as u64,
            rooms: rooms.iter().map(|r| rid(r)).collect(),
        }
    }

    /// Space `sp` (level 0) with the sub-spaces `tech` (which holds a
    /// level-2 room `rustroom`) and `voice`, and the SDK's own-ID-last
    /// convention for the level-1 spaces.
    fn spaces() -> Vec<SpaceView> {
        vec![
            view(
                "sp",
                0,
                &["chan1", "chan2", "dm1", "tech", "voice", "starred1"],
            ),
            view(
                "tech",
                1,
                &["t1", "t2", "rustroom", "rust", "techdm", "tech"],
            ),
            view("voice", 1, &["v1", "voice"]),
        ]
    }

    /// Rooms in recent-activity order, the sub-space rows among them.
    fn rooms() -> Vec<RoomSummary> {
        let mut v1 = room("v1");
        v1.is_call_room = true;
        vec![
            room("t1"),
            room("chan1"),
            space_row("tech"),
            v1,
            room("rustroom"),
            room("chan2"),
            dm("dm1"),
            space_row("voice"),
            space_row("rust"),
            room("t2"),
            dm("techdm"),
            room("starred1"),
        ]
    }

    fn lines_of<'a>(lines: &[Line<'a>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| match line {
                Line::Header(h) => format!("# {}", h.name),
                Line::Room(r) => r.name.clone(),
                Line::EmptyHint(_) => "~hint".to_owned(),
                Line::Participant(p) => format!(
                    "  - {}{}{}",
                    p.name,
                    if p.speaking { " *speaking" } else { "" },
                    if p.muted { " !muted" } else { "" }
                ),
                Line::More(m) => format!("  - and {} more", m.hidden),
            })
            .collect()
    }

    fn built(layout: Option<&SpaceLayout>) -> Vec<String> {
        let sp = spaces();
        let rs = rooms();
        lines_of(&build(&sp[0], &sp, &rs, layout))
    }

    fn label(s: &str) -> SidebarLabel {
        SidebarLabel::try_from(s).expect("a valid name")
    }

    mod tags {
        use super::*;
        use twicecat_core::api::RoomSortChoice;

        fn tagged(rs: &mut [RoomSummary], name: &str, favourite: bool, low: bool) {
            let r = rs
                .iter_mut()
                .find(|r| r.name == name)
                .expect("a room of the fixture");
            r.is_favourite = favourite;
            r.is_low_priority = low;
        }

        fn heads(out: &[String]) -> Vec<&str> {
            out.iter()
                .filter(|l| l.starts_with('#'))
                .map(String::as_str)
                .collect()
        }

        fn out_with(rs: &[RoomSummary], layout: Option<&SpaceLayout>) -> Vec<String> {
            let sp = spaces();
            lines_of(&build(&sp[0], &sp, rs, layout))
        }

        /// The names under the header `head`.
        fn under<'a>(out: &'a [String], head: &str) -> Vec<&'a str> {
            let at = out
                .iter()
                .position(|l| l == head)
                .unwrap_or_else(|| panic!("no header {head}"));
            out[at + 1..]
                .iter()
                .take_while(|l| !l.starts_with('#'))
                .map(String::as_str)
                .collect()
        }

        #[test]
        fn both_sections_are_hidden_while_empty() {
            let out = built(None);
            assert!(
                !out.iter()
                    .any(|l| l == "# Favourites" || l == "# Low priority")
            );
            // Rooms that carry no tag leave them empty even when collapsed.
            let mut layout = SpaceLayout::new(rid("sp"));
            layout.collapsed = vec![SectionKey::Favourites, SectionKey::LowPriority];
            assert_eq!(built(Some(&layout)), out);
        }

        #[test]
        fn the_default_order_is_starred_favourites_channels_sub_spaces_custom_direct_low_priority()
        {
            let mut rs = rooms();
            tagged(&mut rs, "chan2", true, false);
            tagged(&mut rs, "starred1", false, true);
            let mut layout = SpaceLayout::new(rid("sp"));
            layout.starred = vec![rid("t1")];
            layout.custom = vec![CustomSection {
                id: 7,
                name: label("Mine"),
                rooms: vec![rid("t2")],
            }];
            let out = out_with(&rs, Some(&layout));
            assert_eq!(
                heads(&out),
                [
                    "# Starred",
                    "# Favourites",
                    "# Channels",
                    "# tech",
                    "# voice",
                    "# Mine",
                    "# Direct messages",
                    "# Low priority",
                ]
            );
        }

        #[test]
        fn a_room_goes_to_the_first_section_that_applies() {
            // Both flags on one room cannot happen on a server, but the
            // placement is defined: Favourites comes first.
            let mut rs = rooms();
            tagged(&mut rs, "t1", true, true); // also starred and custom
            tagged(&mut rs, "t2", true, false); // also custom
            tagged(&mut rs, "chan1", false, true); // also custom
            tagged(&mut rs, "rustroom", false, true); // in a sub-space
            tagged(&mut rs, "dm1", false, true); // a DM
            tagged(&mut rs, "chan2", false, true); // a channel
            tagged(&mut rs, "techdm", true, false); // a DM in a sub-space
            let mut layout = SpaceLayout::new(rid("sp"));
            layout.starred = vec![rid("t1")];
            layout.custom = vec![CustomSection {
                id: 7,
                name: label("Mine"),
                rooms: vec![rid("t1"), rid("t2"), rid("chan1")],
            }];
            let out = out_with(&rs, Some(&layout));
            assert_eq!(under(&out, "# Starred"), ["t1"], "Starred beats Favourites");
            assert_eq!(
                under(&out, "# Favourites"),
                ["t2", "techdm"],
                "Favourites beats custom and its sub-space"
            );
            assert_eq!(
                under(&out, "# Mine"),
                ["chan1"],
                "a custom section beats Low priority"
            );
            assert_eq!(
                under(&out, "# Low priority"),
                ["rustroom", "chan2", "dm1"],
                "Low priority beats a sub-space, Direct messages and Channels"
            );
        }

        #[test]
        fn every_room_appears_once() {
            let mut rs = rooms();
            tagged(&mut rs, "t1", true, true);
            tagged(&mut rs, "t2", true, false);
            tagged(&mut rs, "chan1", false, true);
            tagged(&mut rs, "dm1", false, true);
            tagged(&mut rs, "v1", true, false);
            let mut layout = SpaceLayout::new(rid("sp"));
            layout.starred = vec![rid("t1"), rid("chan2")];
            layout.custom = vec![CustomSection {
                id: 7,
                name: label("Mine"),
                rooms: vec![rid("t2"), rid("chan1")],
            }];
            let out = out_with(&rs, Some(&layout));
            let mut seen: Vec<&str> = out
                .iter()
                .map(String::as_str)
                .filter(|l| !l.starts_with('#') && *l != "~hint" && !l.starts_with("  -"))
                .collect();
            seen.sort_unstable();
            let mut expected: Vec<&str> = rs
                .iter()
                .filter(|r| !r.is_space)
                .map(|r| r.name.as_str())
                .collect();
            expected.sort_unstable();
            assert_eq!(seen, expected, "each room of the list once, none lost");
        }

        #[test]
        fn a_stored_order_holding_favourites_is_kept() {
            let mut rs = rooms();
            tagged(&mut rs, "chan2", true, false);
            let mut layout = SpaceLayout::new(rid("sp"));
            layout.order = vec![
                SectionKey::Channels,
                SectionKey::Direct,
                SectionKey::Favourites,
            ];
            let out = out_with(&rs, Some(&layout));
            let at = |h: &str| out.iter().position(|l| l == h).expect(h);
            assert!(at("# Channels") < at("# Direct messages"));
            assert!(
                at("# Direct messages") < at("# Favourites"),
                "Favourites is where the stored order puts it: {:?}",
                heads(&out)
            );
            // Hidden while empty, the key stays in the stored order and
            // comes back at its place when a room is tagged again.
            tagged(&mut rs, "chan2", false, false);
            let hidden = out_with(&rs, Some(&layout));
            assert!(!hidden.iter().any(|l| l == "# Favourites"));
            assert_eq!(layout.order.len(), 3);
        }

        #[test]
        fn a_collapsed_new_section_keeps_its_header_with_the_badge() {
            let mut rs = rooms();
            tagged(&mut rs, "chan2", false, true);
            rs.iter_mut()
                .find(|r| r.name == "chan2")
                .expect("chan2")
                .unread_notifications = 3;
            let mut layout = SpaceLayout::new(rid("sp"));
            layout.collapsed = vec![SectionKey::LowPriority];
            let sp = spaces();
            let lines = build(&sp[0], &sp, &rs, Some(&layout));
            let header = lines
                .iter()
                .find_map(|l| match l {
                    Line::Header(h) if h.key == SectionKey::LowPriority => Some(h),
                    _ => None,
                })
                .expect("the Low priority header");
            assert!(header.collapsed);
            assert_eq!(header.badge, Badge::Count(3));
            assert!(!lines_of(&lines).iter().any(|l| l == "chan2"));
        }

        #[test]
        fn the_new_sections_follow_the_chosen_sort() {
            let mut rs = rooms();
            tagged(&mut rs, "t2", true, false);
            tagged(&mut rs, "chan1", true, false);
            tagged(&mut rs, "t1", true, false);
            let sp = spaces();
            let by_name = Sorting {
                by: RoomSortChoice::Name,
                unread_first: false,
            };
            let out = lines_of(&build_sorted(&sp[0], &sp, &rs, None, None, by_name));
            assert_eq!(under(&out, "# Favourites"), ["chan1", "t1", "t2"]);
            let out = lines_of(&build(&sp[0], &sp, &rs, None));
            assert_eq!(
                under(&out, "# Favourites"),
                ["t1", "chan1", "t2"],
                "the list's own order by default"
            );
        }
    }

    mod sorted {
        use super::*;
        use twicecat_core::api::RoomSortChoice;

        fn fixture() -> (Vec<SpaceView>, Vec<RoomSummary>, SpaceLayout) {
            let sp = vec![view(
                "sp",
                0,
                &[
                    "zeta", "Alpha", "mid", "quiet", "st1", "st2", "cu1", "cu2", "dm_b", "dm_a",
                ],
            )];
            let mut rs: Vec<RoomSummary> =
                ["zeta", "Alpha", "mid", "quiet", "st1", "st2", "cu1", "cu2"]
                    .into_iter()
                    .map(room)
                    .collect();
            rs.push(dm("dm_b"));
            rs.push(dm("dm_a"));
            // Unread: `mid` has a mention, `quiet` stays quiet, `zeta` is
            // marked unread.
            rs[2].highlights = 1;
            rs[0].marked_unread = true;
            let mut layout = SpaceLayout::new(rid("sp"));
            layout.starred = vec![rid("st2"), rid("st1")];
            layout.custom = vec![CustomSection {
                id: 1,
                name: label("Mine"),
                rooms: vec![rid("cu2"), rid("cu1")],
            }];
            (sp, rs, layout)
        }

        fn lines(sorting: Sorting) -> Vec<String> {
            let (sp, rs, layout) = fixture();
            lines_of(&build_sorted(
                &sp[0],
                &sp,
                &rs,
                Some(&layout),
                None,
                sorting,
            ))
        }

        #[test]
        fn the_default_is_the_lists_own_order() {
            assert_eq!(
                lines(Sorting::default()),
                [
                    "# Starred",
                    "st2",
                    "st1",
                    "# Channels",
                    "zeta",
                    "Alpha",
                    "mid",
                    "quiet",
                    "# Mine",
                    "cu2",
                    "cu1",
                    "# Direct messages",
                    "dm_b",
                    "dm_a"
                ]
            );
        }

        #[test]
        fn channels_and_dms_in_name_order_are_sorted() {
            let out = lines(Sorting {
                by: RoomSortChoice::Name,
                unread_first: false,
            });
            assert_eq!(
                out,
                [
                    "# Starred",
                    "st2",
                    "st1",
                    "# Channels",
                    "Alpha",
                    "mid",
                    "quiet",
                    "zeta",
                    "# Mine",
                    "cu2",
                    "cu1",
                    "# Direct messages",
                    "dm_a",
                    "dm_b"
                ]
            );
        }

        #[test]
        fn starred_and_custom_sections_keep_the_users_order_under_every_setting() {
            for by in [RoomSortChoice::Activity, RoomSortChoice::Name] {
                for unread_first in [false, true] {
                    let out = lines(Sorting { by, unread_first });
                    let at = |name: &str| out.iter().position(|l| l == name).expect(name);
                    assert!(at("st2") < at("st1"), "{by:?} {unread_first}");
                    assert!(at("cu2") < at("cu1"), "{by:?} {unread_first}");
                }
            }
        }

        #[test]
        fn unread_first_puts_unread_rooms_first_inside_a_section_only() {
            let out = lines(Sorting {
                by: RoomSortChoice::Name,
                unread_first: true,
            });
            let channels = out.iter().position(|l| l == "# Channels").expect("header");
            // `mid` (a mention) before `zeta` (marked), then the quiet ones
            // in name order.
            assert_eq!(
                out[channels + 1..channels + 5],
                ["mid", "zeta", "Alpha", "quiet"]
            );
        }

        #[test]
        fn sorting_does_not_change_which_section_a_room_is_in() {
            let (sp, rs, layout) = fixture();
            let sections = |sorting| {
                let lines = build_sorted(&sp[0], &sp, &rs, Some(&layout), None, sorting);
                let mut seen: Vec<(SectionKey, String)> = Vec::new();
                let mut key = None;
                for line in &lines {
                    match line {
                        Line::Header(h) => key = Some(h.key.clone()),
                        Line::Room(r) => seen.push((key.clone().expect("header"), r.name.clone())),
                        _ => {}
                    }
                }
                seen.sort_by(|a, b| a.1.cmp(&b.1));
                seen
            };
            let name = Sorting {
                by: RoomSortChoice::Name,
                unread_first: true,
            };
            assert_eq!(sections(Sorting::default()), sections(name));
        }

        // ---- the app: the settings, the keys, the paging ----

        use crate::settings::RecordingSettings;
        use crate::sidebar_menu::{SidebarMenu, SidebarMenuMessage as M};

        fn menu(app: &mut App, message: M) {
            let _ = app.update(Message::SidebarMenu(message));
        }

        /// The card open, the choices made.
        fn choose(app: &mut App, by: RoomSortChoice, unread_first: bool) {
            menu(app, M::OpenSort);
            menu(app, M::SetSort(by));
            menu(app, M::UnreadFirst(unread_first));
        }

        #[test]
        fn the_sort_card_opens_and_esc_or_close_shuts_it() {
            use crate::keys::Shortcut;
            let (mut app, _rx) = space_app(None);
            menu(&mut app, M::OpenSort);
            assert_eq!(app.sidebar_menu, Some(SidebarMenu::Sort));
            press(&mut app, Shortcut::Escape);
            assert!(app.sidebar_menu.is_none());
            menu(&mut app, M::OpenSort);
            menu(&mut app, M::Close);
            assert!(app.sidebar_menu.is_none());
        }

        #[test]
        fn a_choice_with_no_card_open_changes_nothing() {
            let (mut app, _rx) = space_app(None);
            let store = RecordingSettings::default();
            app.set_settings_store(Box::new(store.clone()));
            menu(&mut app, M::SetSort(RoomSortChoice::Name));
            menu(&mut app, M::UnreadFirst(true));
            assert!(app.sorting().is_default());
            assert!(store.saved.lock().expect("mutex").is_empty());
        }

        #[test]
        fn choosing_a_sort_saves_the_settings_once_per_change() {
            let (mut app, _rx) = space_app(None);
            let store = RecordingSettings::default();
            app.set_settings_store(Box::new(store.clone()));
            menu(&mut app, M::OpenSort);
            menu(&mut app, M::SetSort(RoomSortChoice::Name));
            assert_eq!(store.saved.lock().expect("mutex").len(), 1);
            assert_eq!(
                store.saved.lock().expect("mutex")[0].room_sort,
                RoomSortChoice::Name
            );
            // The same choice again is not a change: nothing is written.
            menu(&mut app, M::SetSort(RoomSortChoice::Name));
            assert_eq!(store.saved.lock().expect("mutex").len(), 1);
            menu(&mut app, M::UnreadFirst(true));
            let saved = store.saved.lock().expect("mutex").clone();
            assert_eq!(saved.len(), 2);
            assert!(saved[1].unread_first && saved[1].room_sort == RoomSortChoice::Name);
            assert!(
                app.sidebar_menu.is_some(),
                "the card stays for the next choice"
            );
        }

        #[test]
        fn a_restart_restores_the_sort() {
            let root =
                std::env::temp_dir().join(format!("twicecat-sort-restart-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("a temp dir");
            let paths = twicecat_core::paths::DataPaths::at(&root);

            let (mut app, _rx) = space_app(None);
            let store = RecordingSettings::default();
            app.set_settings_store(Box::new(store.clone()));
            choose(&mut app, RoomSortChoice::Name, true);
            // What the disk store writes is the newest settings handed to it.
            let newest = *store.saved.lock().expect("mutex").last().expect("a save");
            paths.write_settings(&newest).expect("written");

            // The next start reads them back before the first frame.
            let (mut again, _rx) = space_app(None);
            again.set_ui_settings(paths.read_settings());
            assert_eq!(
                again.sorting(),
                Sorting {
                    by: RoomSortChoice::Name,
                    unread_first: true
                }
            );
            let _ = std::fs::remove_dir_all(&root);
        }

        #[test]
        fn alt_down_from_the_last_room_of_a_group_goes_to_the_next_room_as_shown() {
            use crate::keys::Shortcut;
            let (mut app, _rx) = space_app(None);
            // By activity the Tech section reads t1, rustroom, t2.
            let _ = app.update(Message::SelectRoom(rid("starred1")));
            press(&mut app, Shortcut::NextRoom);
            assert_eq!(app.selected, Some(rid("t1")), "the list's own order");
            choose(&mut app, RoomSortChoice::Name, false);
            menu(&mut app, M::Close);
            assert_eq!(
                drawn_names(&app)[..8],
                [
                    "# Channels",
                    "chan1",
                    "chan2",
                    "starred1",
                    "# tech",
                    "rustroom",
                    "t1",
                    "t2"
                ]
            );
            let _ = app.update(Message::SelectRoom(rid("starred1")));
            press(&mut app, Shortcut::NextRoom);
            assert_eq!(app.selected, Some(rid("rustroom")), "as shown, not t1");
            press(&mut app, Shortcut::PreviousRoom);
            assert_eq!(app.selected, Some(rid("starred1")));
        }

        #[test]
        fn alt_up_and_down_walk_the_sorted_flat_list_too() {
            use crate::keys::Shortcut;
            let (mut app, _rx) = space_app(None);
            let _ = app.update(Message::Spaces(SpacesMessage::Select(None)));
            let _ = app.update(Message::SelectRoom(rid("chan1")));
            press(&mut app, Shortcut::NextRoom);
            assert_eq!(app.selected, Some(rid("v1")), "by activity");
            choose(&mut app, RoomSortChoice::Name, false);
            menu(&mut app, M::Close);
            let _ = app.update(Message::SelectRoom(rid("chan1")));
            press(&mut app, Shortcut::NextRoom);
            assert_eq!(app.selected, Some(rid("chan2")), "by name");
            press(&mut app, Shortcut::NextRoom);
            assert_eq!(app.selected, Some(rid("dm1")));
            press(&mut app, Shortcut::PreviousRoom);
            press(&mut app, Shortcut::PreviousRoom);
            assert_eq!(app.selected, Some(rid("chan1")));
        }

        fn loads(rx: &mut mpsc::Receiver<Command>) -> usize {
            drain(rx)
                .into_iter()
                .filter(|c| matches!(c, Command::LoadMoreRooms))
                .count()
        }

        fn flat_500(total: u32) -> (App, mpsc::Receiver<Command>) {
            let (mut app, rx) = space_app(Some(total));
            let _ = app.update(Message::Spaces(SpacesMessage::Select(None)));
            let many: Vec<RoomSummary> = (0..500).map(|i| room(&format!("m{i}"))).collect();
            feed(
                &mut app,
                CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: many }]),
            );
            (app, rx)
        }

        #[test]
        fn a_sorted_flat_list_asks_for_one_page_per_loaded_length_until_complete() {
            let (mut app, mut rx) = flat_500(1100);
            let _ = drain(&mut rx);
            feed(
                &mut app,
                CoreEvent::RoomListLoaded {
                    total_rooms: Some(1100),
                },
            );
            assert_eq!(
                loads(&mut rx),
                0,
                "by activity only the end of the list asks"
            );
            // Choosing A to Z needs every room: the first page is asked for.
            menu(&mut app, M::OpenSort);
            menu(&mut app, M::SetSort(RoomSortChoice::Name));
            assert_eq!(loads(&mut rx), 1);
            // The same length again: not twice.
            feed(
                &mut app,
                CoreEvent::RoomListLoaded {
                    total_rooms: Some(1100),
                },
            );
            assert_eq!(loads(&mut rx), 0);
            let page = |from: usize, to: usize| {
                CoreEvent::RoomListChanged(vec![ListDiff::Append {
                    values: (from..to).map(|i| room(&format!("p{i}"))).collect(),
                }])
            };
            feed(&mut app, page(0, 300));
            assert_eq!(app.rooms.len(), 800);
            assert_eq!(loads(&mut rx), 1, "a page arrived, the next is asked for");
            feed(&mut app, page(300, 600));
            assert_eq!(app.rooms.len(), 1100);
            assert_eq!(loads(&mut rx), 0, "complete: no more");
        }

        #[test]
        fn unreads_first_in_the_flat_list_loads_everything_too() {
            let (mut app, mut rx) = flat_500(1100);
            let _ = drain(&mut rx);
            menu(&mut app, M::OpenSort);
            menu(&mut app, M::UnreadFirst(true));
            assert_eq!(loads(&mut rx), 1);
        }

        #[test]
        fn the_flat_list_is_shown_sorted_and_low_priority_last_with_the_all_chip() {
            let (mut app, _rx) = space_app(None);
            let _ = app.update(Message::Spaces(SpacesMessage::Select(None)));
            assert_eq!(app.flat_order(), None, "nothing to change");
            let mut low = room("chan1");
            low.is_low_priority = true;
            let at = app
                .rooms
                .iter()
                .position(|r| r.name == "chan1")
                .expect("chan1");
            feed(
                &mut app,
                CoreEvent::RoomListChanged(vec![ListDiff::Set {
                    index: at,
                    value: low,
                }]),
            );
            let shown = |app: &App| -> Vec<String> {
                app.rooms_as_drawn()
                    .iter()
                    .map(|r| r.name.clone())
                    .collect()
            };
            assert_eq!(shown(&app).last().map(String::as_str), Some("chan1"));
            // Another chip: low priority stays where the list put it.
            let _ = app.update(Message::ChipClicked(RoomListTab::Rooms));
            assert_eq!(app.flat_order(), None);
        }
    }

    // ---- the options card's questions about the lines (Task sbc.9) ----

    #[test]
    fn debug_of_a_line_shows_no_room_id_and_no_room_name() {
        let mut secret = RoomSummary::new(RoomId::from("!secretroom:x"), "Payroll team".to_owned());
        secret.is_dm = true;
        let lines = [
            Line::Room(&secret),
            Line::Header(Header {
                key: SectionKey::SubSpace(RoomId::from("!secretroom:x")),
                name: "Payroll team",
                collapsed: true,
                badge: Badge::Dot,
            }),
            Line::EmptyHint(SectionKey::SubSpace(RoomId::from("!secretroom:x"))),
        ];
        for line in &lines {
            let text = format!("{line:?}");
            for hidden in ["secretroom", "Payroll"] {
                assert!(!text.contains(hidden), "{hidden} is in {text}");
            }
        }
        assert_eq!(format!("{:?}", lines[0]), "Room(..)");
        assert!(format!("{:?}", lines[1]).starts_with("Header"));
        assert!(format!("{:?}", lines[2]).starts_with("EmptyHint"));
        let all = format!("{lines:?}");
        assert!(
            !all.contains("secretroom") && !all.contains("Payroll"),
            "{all}"
        );
    }

    #[test]
    fn the_drawn_sections_and_a_rooms_spot_come_from_the_lines() {
        let mut layout = SpaceLayout::new(rid("sp"));
        layout.custom = vec![
            CustomSection {
                id: 4,
                name: label("Mine"),
                rooms: vec![rid("chan2"), rid("t1")],
            },
            CustomSection {
                id: 5,
                name: label("Empty"),
                rooms: Vec::new(),
            },
        ];
        layout.collapsed = vec![SectionKey::Direct];
        let sp = spaces();
        let rs = rooms();
        let lines = build(&sp[0], &sp, &rs, Some(&layout));
        assert_eq!(
            drawn_sections(&lines),
            [
                SectionKey::Channels,
                SectionKey::SubSpace(rid("tech")),
                SectionKey::SubSpace(rid("voice")),
                SectionKey::Custom(4),
                SectionKey::Custom(5),
                SectionKey::Direct,
            ]
        );
        let spot = spot_of(&lines, &rid("t1")).expect("drawn");
        assert_eq!(spot.section, SectionKey::Custom(4));
        assert_eq!(spot.rooms, [rid("chan2"), rid("t1")]);
        assert_eq!(spot.index, 1);
        assert_eq!(spot_of(&lines, &rid("chan2")).map(|s| s.index), Some(0));
        // In a collapsed section, or not in the list: not drawn.
        assert_eq!(spot_of(&lines, &rid("dm1")), None);
        assert_eq!(spot_of(&lines, &rid("nobody")), None);
        // The first section's room, and the last section's.
        assert_eq!(
            spot_of(&lines, &rid("chan1")).map(|s| s.section),
            Some(SectionKey::Channels)
        );
        assert_eq!(
            spot_of(&lines, &rid("v1")).map(|s| s.section),
            Some(SectionKey::SubSpace(rid("voice")))
        );
    }

    // ---- placement ----

    #[test]
    fn every_room_of_the_space_appears_exactly_once() {
        let out = built(None);
        let mut seen = HashSet::new();
        for line in &out {
            if !line.starts_with('#') {
                assert!(seen.insert(line.clone()), "{line} twice in {out:?}");
            }
        }
        let want: HashSet<String> = [
            "t1", "t2", "chan1", "chan2", "v1", "rustroom", "dm1", "techdm", "starred1",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        assert_eq!(seen, want);
    }

    #[test]
    fn the_default_order_and_the_default_places() {
        assert_eq!(
            built(None),
            [
                "# Channels",
                "chan1",
                "chan2",
                "starred1",
                "# tech",
                "t1",
                "rustroom",
                "t2",
                "techdm",
                "# voice",
                "v1",
                "# Direct messages",
                "dm1",
            ]
        );
    }

    #[test]
    fn starred_beats_custom_beats_sub_space_beats_dm_beats_channels() {
        let mut layout = SpaceLayout::new(rid("sp"));
        layout.starred = vec![rid("t1"), rid("chan1")];
        layout.custom = vec![CustomSection {
            id: 7,
            name: label("Mine"),
            rooms: vec![rid("t1"), rid("t2"), rid("dm1")],
        }];
        let out = built(Some(&layout));
        assert_eq!(
            out,
            [
                "# Starred",
                "t1",
                "chan1",
                "# Channels",
                "chan2",
                "starred1",
                "# tech",
                "rustroom",
                "techdm",
                "# voice",
                "v1",
                "# Mine",
                "t2",
                "dm1",
            ],
            "t1 starred and in custom: Starred; t2 custom over its sub-space; \
             dm1 custom over DM; techdm (a DM in a sub-space) stays in the sub-space"
        );
    }

    #[test]
    fn the_stored_order_of_starred_and_custom_rooms_is_kept() {
        let mut layout = SpaceLayout::new(rid("sp"));
        layout.starred = vec![rid("chan2"), rid("chan1")];
        layout.custom = vec![CustomSection {
            id: 1,
            name: label("C"),
            rooms: vec![rid("t2"), rid("t1")],
        }];
        let out = built(Some(&layout));
        assert_eq!(&out[..3], ["# Starred", "chan2", "chan1"]);
        let at = out.iter().position(|l| l == "# C").expect("custom header");
        assert_eq!(&out[at..at + 3], ["# C", "t2", "t1"]);
    }

    #[test]
    fn sub_space_rows_are_not_rows_and_a_level_2_room_lands_in_its_level_1_section() {
        let out = built(None);
        for gone in ["tech", "voice", "rust"] {
            assert!(!out.iter().any(|l| l == gone), "{gone} drawn as a row");
        }
        let at = out
            .iter()
            .position(|l| l == "# tech")
            .expect("Tech section");
        assert!(out[at + 1..].iter().take(4).any(|l| l == "rustroom"));
    }

    #[test]
    fn the_spaces_own_id_is_left_out() {
        // A level-1 space is the selected one: the SDK lists it in its own
        // rooms, last (or not at all once the 2000-room cap cuts it off).
        let sp = vec![view("tech", 1, &["t1", "t2", "tech"])];
        for own_row in [space_row("tech"), room("tech")] {
            let rs = vec![room("t1"), own_row, room("t2")];
            let out = lines_of(&build(&sp[0], &sp, &rs, None));
            assert_eq!(out, ["# Channels", "t1", "t2"], "no row, no section for it");
        }
        // Cut off by the cap: the same result.
        let cut = vec![view("tech", 1, &["t1", "t2"])];
        let rs = vec![room("t1"), room("t2")];
        assert_eq!(
            lines_of(&build(&cut[0], &cut, &rs, None)),
            ["# Channels", "t1", "t2"]
        );
        // A level-0 space whose spaces list also holds a level-1 space that
        // is itself: never its own sub-space.
        let both = vec![view("a", 0, &["a", "r1"]), view("a", 1, &["a", "r1"])];
        let rs = vec![room("r1")];
        assert_eq!(
            lines_of(&build(&both[0], &both, &rs, None)),
            ["# Channels", "r1"]
        );
    }

    #[test]
    fn a_selected_sub_space_lists_its_own_rooms_in_channels_direct_messages_and_its_sections() {
        // The sub-space `tech` of `spaces()` is the selected one (a click on
        // it in the rail's space tree): it has no sub-space sections, its
        // rooms (a level 2 room too) are Channels, its DMs are Direct
        // messages, and the layout keyed by its own ID gives it sections.
        let sp = spaces();
        // What the core lists for `tech`: its rooms and its own row.
        let rs: Vec<RoomSummary> = rooms()
            .into_iter()
            .filter(|r| {
                ["t1", "rustroom", "t2", "techdm", "tech"]
                    .iter()
                    .any(|n| r.id == rid(n))
            })
            .collect();
        let tech = &sp[1];
        assert_eq!(tech.level, 1);
        assert_eq!(
            lines_of(&build(tech, &sp, &rs, None)),
            [
                "# Channels",
                "t1",
                "rustroom",
                "t2",
                "# Direct messages",
                "techdm"
            ],
            "its own ID and the sub-space rows are no rows, and no sub-space section"
        );
        let mut layout = SpaceLayout::new(rid("tech"));
        layout.custom = vec![CustomSection {
            id: 4,
            name: label("Mine"),
            rooms: vec![rid("t2")],
        }];
        layout.starred = vec![rid("rustroom")];
        assert_eq!(
            lines_of(&build(tech, &sp, &rs, Some(&layout))),
            [
                "# Starred",
                "rustroom",
                "# Channels",
                "t1",
                "# Mine",
                "t2",
                "# Direct messages",
                "techdm"
            ]
        );
    }

    #[test]
    fn a_level_1_space_in_two_sub_spaces_is_placed_once() {
        let sp = vec![
            view("sp", 0, &["a", "b"]),
            view("a", 1, &["x", "a"]),
            view("b", 1, &["x", "b"]),
        ];
        let rs = vec![room("x")];
        assert_eq!(
            lines_of(&build(&sp[0], &sp, &rs, None)),
            ["# a", "x"],
            "the first sub-space wins; the other is empty and hidden"
        );
    }

    #[test]
    fn empty_built_in_sections_are_hidden_and_an_empty_custom_one_shows_its_hint() {
        let sp = vec![view("sp", 0, &["a", "empty"]), view("empty", 1, &["empty"])];
        let rs = vec![room("a")];
        assert_eq!(
            lines_of(&build(&sp[0], &sp, &rs, None)),
            ["# Channels", "a"]
        );
        let mut layout = SpaceLayout::new(rid("sp"));
        layout.custom = vec![CustomSection {
            id: 3,
            name: label("Later"),
            rooms: vec![rid("gone")],
        }];
        assert_eq!(
            lines_of(&build(&sp[0], &sp, &rs, Some(&layout))),
            ["# Channels", "a", "# Later", "~hint"]
        );
        layout.collapsed = vec![SectionKey::Custom(3)];
        assert_eq!(
            lines_of(&build(&sp[0], &sp, &rs, Some(&layout))),
            ["# Channels", "a", "# Later"],
            "collapsed, the hint goes with the rooms"
        );
    }

    #[test]
    fn a_space_with_no_rooms_has_no_lines() {
        let sp = vec![view("sp", 0, &[])];
        assert!(build(&sp[0], &sp, &[], None).is_empty());
    }

    #[test]
    fn the_stored_order_is_kept_and_missing_keys_take_their_default_place() {
        let mut layout = SpaceLayout::new(rid("sp"));
        layout.order = vec![
            SectionKey::Direct,
            SectionKey::SubSpace(rid("voice")),
            SectionKey::Channels,
        ];
        let heads: Vec<String> = built(Some(&layout))
            .into_iter()
            .filter(|l| l.starts_with('#'))
            .collect();
        // `tech` is missing from the stored order: it goes right after the
        // section that precedes it by default (Channels).
        assert_eq!(
            heads,
            ["# Direct messages", "# voice", "# Channels", "# tech"]
        );
    }

    #[test]
    fn unknown_stored_keys_are_not_drawn_and_not_a_problem() {
        let mut layout = SpaceLayout::new(rid("sp"));
        layout.order = vec![
            SectionKey::Custom(99),
            SectionKey::SubSpace(rid("nowhere")),
            SectionKey::Starred,
            SectionKey::Direct,
        ];
        let heads: Vec<String> = built(Some(&layout))
            .into_iter()
            .filter(|l| l.starts_with('#'))
            .collect();
        assert_eq!(
            heads,
            ["# Channels", "# tech", "# voice", "# Direct messages"],
            "empty Starred is hidden even when stored first; the one stored key \
             is where it is by default"
        );
    }

    #[test]
    fn a_collapsed_section_keeps_its_header_and_drops_its_rooms() {
        let mut layout = SpaceLayout::new(rid("sp"));
        layout.collapsed = vec![SectionKey::SubSpace(rid("tech")), SectionKey::Channels];
        let out = built(Some(&layout));
        assert_eq!(
            out,
            [
                "# Channels",
                "# tech",
                "# voice",
                "v1",
                "# Direct messages",
                "dm1"
            ]
        );
    }

    // ---- badges ----

    fn counted(n: &str, notifications: u64, highlights: u64) -> RoomSummary {
        let mut r = room(n);
        r.unread_notifications = notifications;
        r.highlights = highlights;
        r
    }

    #[test]
    fn a_section_badge_follows_badge_for_over_the_summed_counts() {
        let a = counted("a", 3, 1);
        let b = counted("b", 2, 1);
        assert_eq!(section_badge(&[&a, &b]), Badge::Highlight(2));
        let c = counted("c", 4, 0);
        let d = counted("d", 1, 0);
        assert_eq!(section_badge(&[&c, &d]), Badge::Count(5));
        let mut marked = room("m");
        marked.marked_unread = true;
        assert_eq!(section_badge(&[&room("q"), &marked]), Badge::Dot);
        let mut quiet = room("u");
        quiet.unread_messages = 3;
        assert_eq!(section_badge(&[&quiet]), Badge::Dot);
        assert_eq!(section_badge(&[&room("z")]), Badge::None);
        assert_eq!(section_badge(&[]), Badge::None);
    }

    #[test]
    fn highlights_2_and_notifications_5_make_highlight_2() {
        let a = counted("a", 3, 2);
        let b = counted("b", 2, 0);
        assert_eq!(section_badge(&[&a, &b]), Badge::Highlight(2));
    }

    #[test]
    fn muted_rooms_are_left_out_of_the_sums_like_the_core_does() {
        let mut loud = counted("a", 0, 0);
        loud.unread_notifications = 2;
        let mut muted = counted("b", 9, 4);
        muted.notification_mode = Some(RoomNotificationModeView::Mute);
        assert_eq!(section_badge(&[&loud, &muted]), Badge::Count(2));
        assert_eq!(section_badge(&[&muted]), Badge::None);
    }

    #[test]
    fn sums_saturate() {
        let a = counted("a", u64::MAX, 0);
        let b = counted("b", 5, 0);
        assert_eq!(section_badge(&[&a, &b]), Badge::Count(u64::MAX));
    }

    #[test]
    fn only_a_collapsed_header_carries_a_badge() {
        let sp = vec![view("sp", 0, &["a", "b"])];
        let rs = vec![counted("a", 5, 2), counted("b", 0, 0)];
        let mut layout = SpaceLayout::new(rid("sp"));
        let badge_of = |layout: &SpaceLayout| -> Badge {
            match &build(&sp[0], &sp, &rs, Some(layout))[0] {
                Line::Header(h) => h.badge,
                other => panic!("not a header: {other:?}"),
            }
        };
        assert_eq!(badge_of(&layout), Badge::None, "expanded: no header badge");
        layout.collapsed = vec![SectionKey::Channels];
        assert_eq!(badge_of(&layout), Badge::Highlight(2));
    }

    // ---- windowing ----

    fn mixed_lines() -> Vec<RoomSummary> {
        (0..20).map(|i| room(&format!("r{i}"))).collect()
    }

    /// Header, 3 rooms, header, hint, header, 3 rooms ... in comfortable
    /// heights: 32, 64, 64, 64, 32, 36, 32, 64, ...
    fn mixed<'a>(rooms: &'a [RoomSummary]) -> Vec<Line<'a>> {
        let header = |key| {
            Line::Header(Header {
                key,
                name: "h",
                collapsed: false,
                badge: Badge::None,
            })
        };
        let mut out = vec![header(SectionKey::Channels)];
        out.extend(rooms[0..3].iter().map(Line::Room));
        out.push(header(SectionKey::Custom(1)));
        out.push(Line::EmptyHint(SectionKey::Custom(1)));
        out.push(header(SectionKey::Direct));
        out.extend(rooms[3..8].iter().map(Line::Room));
        out
    }

    #[test]
    fn heights_follow_the_density() {
        let rooms = mixed_lines();
        let lines = mixed(&rooms);
        let heights: Vec<f32> = lines
            .iter()
            .map(|l| line_height(l, Density::Comfortable))
            .collect();
        assert_eq!(&heights[..7], [32.0, 64.0, 64.0, 64.0, 32.0, 36.0, 32.0]);
        let compact: Vec<f32> = lines
            .iter()
            .map(|l| line_height(l, Density::Compact))
            .collect();
        assert_eq!(&compact[..7], [28.0, 44.0, 44.0, 44.0, 28.0, 32.0, 28.0]);
    }

    #[test]
    fn offsets_are_prefix_sums_ending_with_the_total() {
        let rooms = mixed_lines();
        let lines = mixed(&rooms);
        let offsets = line_offsets(&lines, Density::Comfortable);
        assert_eq!(offsets.len(), lines.len() + 1);
        assert_eq!(&offsets[..5], [0.0, 32.0, 96.0, 160.0, 224.0]);
        let total: f32 = lines
            .iter()
            .map(|l| line_height(l, Density::Comfortable))
            .sum();
        assert_eq!(*offsets.last().expect("a total"), total);
        assert_eq!(line_offsets(&[], Density::Comfortable), [0.0]);
    }

    /// Offsets of lines 32, 64, 64, 36, 32, 64: 0, 32, 96, 160, 196, 228, 292.
    const OFFSETS: [f32; 7] = [0.0, 32.0, 96.0, 160.0, 196.0, 228.0, 292.0];

    #[test]
    fn visible_lines_returns_exactly_the_lines_that_cross_the_viewport() {
        // 50..150 crosses line 1 (32..96) and line 2 (96..160), not 0 or 3.
        assert_eq!(visible_lines(&OFFSETS, 50.0, 100.0, 0), 1..3);
        // Exactly line 2 fills 96..160: neighbours touch but do not cross.
        assert_eq!(visible_lines(&OFFSETS, 96.0, 64.0, 0), 2..3);
        // One pixel more at the bottom brings line 3 in.
        assert_eq!(visible_lines(&OFFSETS, 96.0, 65.0, 0), 2..4);
        // One pixel more at the top brings line 1 in.
        assert_eq!(visible_lines(&OFFSETS, 95.0, 64.0, 0), 1..3);
        // The first page.
        assert_eq!(visible_lines(&OFFSETS, 0.0, 33.0, 0), 0..2);
    }

    #[test]
    fn overscan_adds_lines_on_both_sides_and_stops_at_the_ends() {
        assert_eq!(visible_lines(&OFFSETS, 96.0, 64.0, 1), 1..4);
        assert_eq!(visible_lines(&OFFSETS, 96.0, 64.0, 10), 0..6);
        assert_eq!(visible_lines(&OFFSETS, 0.0, 10.0, 2), 0..3);
    }

    #[test]
    fn nan_negative_and_stale_offsets_are_clamped() {
        let first = visible_lines(&OFFSETS, 0.0, 100.0, 0);
        assert_eq!(visible_lines(&OFFSETS, f32::NAN, 100.0, 0), first);
        assert_eq!(visible_lines(&OFFSETS, -500.0, 100.0, 0), first);
        assert_eq!(visible_lines(&OFFSETS, f32::NEG_INFINITY, 100.0, 0), first);
        // The list shrank under the offset: the last page.
        let last = visible_lines(&OFFSETS, 1.0e9, 100.0, 0);
        assert_eq!(last, 3..6, "line 3 still crosses the last page");
        assert_eq!(visible_lines(&OFFSETS, f32::INFINITY, 100.0, 0), last);
        // A bad viewport counts as zero high.
        assert_eq!(visible_lines(&OFFSETS, 0.0, f32::NAN, 0), 0..0);
        assert_eq!(visible_lines(&OFFSETS, 0.0, f32::INFINITY, 0), 0..0);
        assert_eq!(visible_lines(&OFFSETS, 0.0, -3.0, 0), 0..0);
        assert_eq!(visible_lines(&[0.0], 5.0, 100.0, 3), 0..0);
        assert_eq!(visible_lines(&[], 5.0, 100.0, 3), 0..0);
    }

    #[test]
    fn spacers_and_built_lines_add_up_to_the_total() {
        let total = *OFFSETS.last().expect("a total");
        for (offset, viewport, overscan) in [
            (0.0, 100.0, 0),
            (50.0, 100.0, 1),
            (96.0, 64.0, 0),
            (1.0e9, 100.0, 0),
            (f32::NAN, 100.0, 2),
            (100.0, 1000.0, 0),
        ] {
            let range = visible_lines(&OFFSETS, offset, viewport, overscan);
            let (top, bottom) = line_spacers(&OFFSETS, &range);
            let built = OFFSETS[range.end] - OFFSETS[range.start];
            assert_eq!(
                top + built + bottom,
                total,
                "{offset} {viewport} {overscan}"
            );
        }
        // The same over real lines.
        let rooms = mixed_lines();
        let lines = mixed(&rooms);
        let offsets = line_offsets(&lines, Density::Compact);
        let range = visible_lines(&offsets, 100.0, 120.0, 1);
        let (top, bottom) = line_spacers(&offsets, &range);
        let built: f32 = lines[range.clone()]
            .iter()
            .map(|l| line_height(l, Density::Compact))
            .sum();
        let total: f32 = lines.iter().map(|l| line_height(l, Density::Compact)).sum();
        assert!((top + built + bottom - total).abs() < 0.01);
    }

    #[test]
    fn equal_heights_give_what_visible_range_gives() {
        let offsets: Vec<f32> = (0..=1000).map(|i| i as f32 * 64.0).collect();
        for (offset, viewport) in [
            (0.0, 800.0),
            (63.0, 800.0),
            (64.0, 800.0),
            (12_345.0, 640.0),
            (1.0e9, 800.0),
            (f32::NAN, 800.0),
            (-4.0, 800.0),
        ] {
            assert_eq!(
                visible_lines(&offsets, offset, viewport, OVERSCAN_ROWS),
                visible_range(offset, viewport, 64.0, 1000, OVERSCAN_ROWS),
                "{offset} {viewport}"
            );
        }
    }

    // ---- the app ----

    fn app() -> (App, mpsc::Receiver<Command>) {
        let (tx, rx) = mpsc::channel(4096);
        let (_events_tx, events_rx) = mpsc::channel(1);
        let (mut app, _boot) = App::new(Startup {
            sink: Box::new(tx),
            events: EventsSlot::new(events_rx),
        });
        app.set_sidebar_store(Box::new(MemorySidebar::default()));
        (app, rx)
    }

    fn feed(app: &mut App, event: CoreEvent) {
        let _ = app.update(Message::Core(Box::new(event)));
    }

    /// An app logged in as `@a:x` with its layout loaded, the spaces and the
    /// rooms of the fixtures, and the space `sp` selected.
    fn space_app(total: Option<u32>) -> (App, mpsc::Receiver<Command>) {
        let (mut app, rx) = app();
        feed(
            &mut app,
            CoreEvent::LoggedIn {
                user_id: UserId::from("@a:x"),
                device_id: DeviceId::from("D"),
                restored: false,
            },
        );
        let _ = app.update(Message::SidebarLoaded(SidebarAnswer::new(
            UserId::from("@a:x"),
            SidebarLoad::Layout(SidebarLayout::new(UserId::from("@a:x"))),
        )));
        feed(&mut app, CoreEvent::Spaces(spaces()));
        feed(
            &mut app,
            CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: rooms() }]),
        );
        feed(&mut app, CoreEvent::RoomListLoaded { total_rooms: total });
        let _ = app.update(Message::Spaces(SpacesMessage::Select(Some(rid("sp")))));
        (app, rx)
    }

    fn drain(rx: &mut mpsc::Receiver<Command>) -> Vec<Command> {
        let mut out = Vec::new();
        while let Ok(c) = rx.try_recv() {
            out.push(c);
        }
        out
    }

    fn drawn_names(app: &App) -> Vec<String> {
        lines_of(&app.section_lines().expect("sections show"))
    }

    // ---- when sections show ----

    #[test]
    fn sections_show_with_a_space_the_all_tab_a_blank_search_and_the_expanded_list() {
        let (mut app, _rx) = space_app(None);
        assert!(app.sections_shown());
        assert!(app.section_lines().is_some());
        // A blank search of spaces only is still blank.
        let _ = app.update(Message::SearchChanged("   ".to_owned()));
        assert!(app.sections_shown());
    }

    #[test]
    fn home_a_search_another_tab_or_the_rail_keep_the_flat_list() {
        let flat = |app: &App| !app.sections_shown() && app.section_lines().is_none();
        let (mut app, _rx) = space_app(None);
        let _ = app.update(Message::SearchChanged("t1".to_owned()));
        assert!(flat(&app), "a search");
        let _ = app.update(Message::SearchChanged(String::new()));
        assert!(app.sections_shown());
        for tab in [RoomListTab::People, RoomListTab::Rooms, RoomListTab::Spaces] {
            let _ = app.update(Message::ChipClicked(tab));
            assert!(flat(&app), "{tab:?}");
        }
        let _ = app.update(Message::ChipClicked(RoomListTab::All));
        assert!(app.sections_shown());
        app.ui_settings.room_list = RoomListChoice::Rail;
        assert!(flat(&app), "the rail");
        app.ui_settings.room_list = RoomListChoice::Expanded;
        let _ = app.update(Message::Spaces(SpacesMessage::Select(None)));
        assert!(flat(&app), "Home");
    }

    #[test]
    fn the_flat_list_is_the_rooms_in_their_order_as_before() {
        let (mut app, _rx) = space_app(None);
        let _ = app.update(Message::Spaces(SpacesMessage::Select(None)));
        let names: Vec<String> = app
            .rooms_as_drawn()
            .iter()
            .map(|r| r.name.clone())
            .collect();
        let want: Vec<String> = app.rooms.iter().map(|r| r.name.clone()).collect();
        assert_eq!(names, want);
    }

    // ---- keyboard ----

    fn press(app: &mut App, shortcut: crate::keys::Shortcut) {
        let _ = app.update(Message::Shortcut {
            shortcut,
            captured: false,
        });
    }

    fn collapse(app: &mut App, key: SectionKey) {
        assert!(
            app.apply_sidebar(Op::ToggleSection {
                space: rid("sp"),
                key,
            })
            .is_ok()
        );
    }

    #[test]
    fn alt_down_goes_from_the_last_room_of_a_section_to_the_first_of_the_next_drawn_one() {
        use crate::keys::Shortcut;
        let (mut app, _rx) = space_app(None);
        collapse(&mut app, SectionKey::SubSpace(rid("tech")));
        assert_eq!(
            drawn_names(&app),
            [
                "# Channels",
                "chan1",
                "chan2",
                "starred1",
                "# tech",
                "# voice",
                "v1",
                "# Direct messages",
                "dm1"
            ]
        );
        let _ = app.update(Message::SelectRoom(rid("starred1")));
        press(&mut app, Shortcut::NextRoom);
        assert_eq!(app.selected, Some(rid("v1")), "Tech is collapsed: skipped");
        press(&mut app, Shortcut::NextRoom);
        assert_eq!(app.selected, Some(rid("dm1")));
        press(&mut app, Shortcut::NextRoom);
        assert_eq!(app.selected, Some(rid("chan1")), "wraps to the first drawn");
        press(&mut app, Shortcut::PreviousRoom);
        assert_eq!(app.selected, Some(rid("dm1")), "and back");
        press(&mut app, Shortcut::PreviousRoom);
        press(&mut app, Shortcut::PreviousRoom);
        assert_eq!(app.selected, Some(rid("starred1")));
    }

    #[test]
    fn ctrl_shift_arrows_walk_the_drawn_rooms_too() {
        use crate::keys::Shortcut;
        let (mut app, _rx) = space_app(None);
        collapse(&mut app, SectionKey::SubSpace(rid("tech")));
        let mut unread = room("v1");
        unread.unread_messages = 2;
        let mut hidden = room("t1");
        hidden.unread_messages = 2;
        feed(
            &mut app,
            CoreEvent::RoomListChanged(vec![
                ListDiff::Set {
                    index: 3,
                    value: unread,
                },
                ListDiff::Set {
                    index: 0,
                    value: hidden,
                },
            ]),
        );
        let _ = app.update(Message::SelectRoom(rid("chan1")));
        press(&mut app, Shortcut::NextUnread);
        assert_eq!(app.selected, Some(rid("v1")));
        press(&mut app, Shortcut::NextUnread);
        assert_eq!(
            app.selected,
            Some(rid("v1")),
            "t1 is in a collapsed section"
        );
    }

    #[test]
    fn the_keys_walk_the_flat_list_when_sections_do_not_show() {
        use crate::keys::Shortcut;
        let (mut app, _rx) = space_app(None);
        let _ = app.update(Message::SearchChanged("x".to_owned()));
        let _ = app.update(Message::SelectRoom(rid("t1")));
        press(&mut app, Shortcut::NextRoom);
        assert_eq!(app.selected, Some(rid("chan1")), "the list's own order");
    }

    // ---- load all ----

    #[test]
    fn the_load_all_rule() {
        assert!(!must_load_all(10, None));
        assert!(!must_load_all(10, Some(10)));
        assert!(!must_load_all(10, Some(3)));
        assert!(must_load_all(10, Some(11)));
        assert!(must_load_all(0, Some(1)));
    }

    #[test]
    fn sectioned_mode_asks_for_a_page_per_loaded_page_until_everything_is_in() {
        let (mut app, mut rx) = space_app(Some(40));
        let loads = |rx: &mut mpsc::Receiver<Command>| {
            drain(rx)
                .into_iter()
                .filter(|c| matches!(c, Command::LoadMoreRooms))
                .count()
        };
        // Selecting the space asks the core for its rooms; its answer
        // arrives as a list and a total. 12 loaded of 40, and the viewport is
        // nowhere near the end: the next page is still asked for.
        let _ = drain(&mut rx);
        feed(
            &mut app,
            CoreEvent::RoomListLoaded {
                total_rooms: Some(40),
            },
        );
        assert_eq!(loads(&mut rx), 1);
        // The same length again: not asked twice.
        feed(
            &mut app,
            CoreEvent::RoomListLoaded {
                total_rooms: Some(40),
            },
        );
        assert_eq!(loads(&mut rx), 0);
        // A page arrives: asked once more.
        let page: Vec<RoomSummary> = (0..20).map(|i| room(&format!("p{i}"))).collect();
        feed(
            &mut app,
            CoreEvent::RoomListChanged(vec![ListDiff::Append { values: page }]),
        );
        assert_eq!(loads(&mut rx), 1);
        let page: Vec<RoomSummary> = (20..28).map(|i| room(&format!("p{i}"))).collect();
        feed(
            &mut app,
            CoreEvent::RoomListChanged(vec![ListDiff::Append { values: page }]),
        );
        assert_eq!(app.rooms.len(), 40);
        assert_eq!(loads(&mut rx), 0, "len == total: done");
    }

    #[test]
    fn sectioned_mode_pages_even_with_the_viewport_far_from_the_end() {
        let (mut app, mut rx) = space_app(Some(1500));
        let _ = drain(&mut rx);
        let many: Vec<RoomSummary> = (0..500).map(|i| room(&format!("m{i}"))).collect();
        feed(
            &mut app,
            CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: many }]),
        );

        feed(
            &mut app,
            CoreEvent::RoomListLoaded {
                total_rooms: Some(1500),
            },
        );
        assert_eq!(
            drain(&mut rx)
                .iter()
                .filter(|c| matches!(c, Command::LoadMoreRooms))
                .count(),
            1,
            "500 of 1500 loaded, the viewport is at the top, and one page is asked for \
             once per loaded length"
        );
    }

    #[test]
    fn the_flat_list_still_pages_only_near_its_end() {
        let (mut app, mut rx) = space_app(Some(5000));
        let _ = app.update(Message::Spaces(SpacesMessage::Select(None)));
        let _ = drain(&mut rx);
        let many: Vec<RoomSummary> = (0..500).map(|i| room(&format!("m{i}"))).collect();
        feed(
            &mut app,
            CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: many }]),
        );
        feed(
            &mut app,
            CoreEvent::RoomListLoaded {
                total_rooms: Some(5000),
            },
        );
        assert!(
            drain(&mut rx)
                .iter()
                .all(|c| !matches!(c, Command::LoadMoreRooms)),
            "the viewport is at the top of 500 rooms"
        );
    }

    #[test]
    fn expanding_a_section_asks_for_the_avatars_of_the_rooms_it_shows() {
        use twicecat_core::api::AvatarId;
        let (mut app, mut rx) = space_app(None);
        let tech = SectionKey::SubSpace(rid("tech"));
        collapse(&mut app, tech.clone());
        let mut with_avatar = room("t1");
        with_avatar.avatar = Some(AvatarId::from("mxc://x/t1"));
        feed(
            &mut app,
            CoreEvent::RoomListChanged(vec![ListDiff::Set {
                index: 0,
                value: with_avatar,
            }]),
        );
        let asked = |rx: &mut mpsc::Receiver<Command>| {
            drain(rx)
                .into_iter()
                .any(|c| matches!(c, Command::FetchAvatars(ids) if ids.contains(&AvatarId::from("mxc://x/t1"))))
        };
        assert!(!asked(&mut rx), "t1 is in a collapsed section: not in view");
        let _ = app.update(Message::Spaces(SpacesMessage::ToggleSection(tech)));
        assert!(asked(&mut rx), "expanded: now in view");
    }

    #[test]
    fn the_avatars_of_the_channel_rows_drawn_are_asked_for_and_none_for_hidden_rooms() {
        use twicecat_core::api::AvatarId;
        let (mut app, mut rx) = space_app(None);
        collapse(&mut app, SectionKey::SubSpace(rid("tech")));
        let _ = drain(&mut rx);
        let mut rows = app.rooms.clone();
        for r in &mut rows {
            // A voice room in view, a text room in a collapsed section.
            if r.id == rid("v1") {
                r.is_call_room = true;
                r.avatar = Some(AvatarId::from("mxc://x/v1"));
            } else if r.id == rid("t1") {
                r.avatar = Some(AvatarId::from("mxc://x/t1"));
            }
        }
        feed(
            &mut app,
            CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: rows }]),
        );
        let asked: Vec<AvatarId> = drain(&mut rx)
            .into_iter()
            .filter_map(|c| match c {
                Command::FetchAvatars(ids) => Some(ids),
                _ => None,
            })
            .flatten()
            .collect();
        assert!(asked.contains(&AvatarId::from("mxc://x/v1")), "{asked:?}");
        assert!(!asked.contains(&AvatarId::from("mxc://x/t1")), "{asked:?}");
    }

    // ---- toggling ----

    #[test]
    fn a_header_click_toggles_the_section_and_saves() {
        let (mut app, _rx) = space_app(None);
        let key = SectionKey::SubSpace(rid("voice"));
        let _ = app.update(Message::Spaces(SpacesMessage::ToggleSection(key.clone())));
        assert!(drawn_names(&app).iter().all(|l| l != "v1"));
        let layout = app.sidebar.layout().expect("a layout");
        assert_eq!(layout.spaces[0].collapsed, vec![key.clone()]);
        let _ = app.update(Message::Spaces(SpacesMessage::ToggleSection(key)));
        assert!(drawn_names(&app).iter().any(|l| l == "v1"));
    }

    // ---- the options cards: right-click, "⋯" and the menu flows (Task sbc.9) ----

    mod menu {
        use twicecat_core::api::sidebar::limits::MAX_CUSTOM_SECTIONS;

        use super::*;
        use crate::keys::Shortcut;
        use crate::sidebar_menu::{
            Entry, Purpose, SectionPick, SidebarMenu, SidebarMenuMessage as M, Target,
        };
        use crate::sidebar_store::Call;

        fn menu_app() -> (App, MemorySidebar, mpsc::Receiver<Command>) {
            let (mut app, rx) = space_app(None);
            let store = MemorySidebar::default();
            app.set_sidebar_store(Box::new(store.clone()));
            (app, store, rx)
        }

        fn send(app: &mut App, message: M) {
            let _ = app.update(Message::SidebarMenu(message));
        }

        fn saves(store: &MemorySidebar) -> usize {
            store
                .calls()
                .iter()
                .filter(|c| matches!(c, Call::Save(..)))
                .count()
        }

        fn record(app: &App) -> SpaceLayout {
            app.sidebar
                .layout()
                .and_then(|l| l.spaces.iter().find(|s| s.space == rid("sp")))
                .cloned()
                .expect("a record for the space")
        }

        fn entry(app: &mut App, entry: Entry) {
            send(app, M::Entry(entry));
        }

        fn name(app: &mut App, text: &str) {
            send(app, M::NameInput(text.to_owned()));
            send(app, M::NameSubmit);
        }

        fn make_section(app: &mut App, section: &str, rooms: &[&str]) {
            send(app, M::OpenRoom(rid(rooms[0])));
            send(app, M::Section(SectionPick::New));
            name(app, section);
            for room in &rooms[1..] {
                let id = record(app).custom.last().expect("a section").id;
                send(app, M::OpenRoom(rid(room)));
                send(app, M::Section(SectionPick::Existing(id)));
            }
            assert!(app.sidebar_menu.is_none());
        }

        #[test]
        fn a_right_click_or_the_dots_open_the_card_of_a_drawn_room_or_header() {
            let (mut app, _store, _rx) = menu_app();
            send(&mut app, M::OpenRoom(rid("chan1")));
            assert_eq!(
                app.sidebar_menu,
                Some(SidebarMenu::Card {
                    target: Target::Room {
                        space: rid("sp"),
                        room: rid("chan1")
                    },
                    error: None
                })
            );
            send(&mut app, M::Close);
            send(&mut app, M::OpenSection(SectionKey::SubSpace(rid("tech"))));
            assert_eq!(
                app.sidebar_menu,
                Some(SidebarMenu::Card {
                    target: Target::Section {
                        space: rid("sp"),
                        key: SectionKey::SubSpace(rid("tech"))
                    },
                    error: None
                })
            );
            send(&mut app, M::Close);
            // Not drawn: a section that is hidden, a room that is not here.
            send(&mut app, M::OpenSection(SectionKey::Starred));
            send(&mut app, M::OpenRoom(rid("nobody")));
            assert!(app.sidebar_menu.is_none());
            // A room in a collapsed section is not drawn.
            collapse(&mut app, SectionKey::Direct);
            send(&mut app, M::OpenRoom(rid("dm1")));
            assert!(app.sidebar_menu.is_none());
        }

        #[test]
        fn no_card_opens_in_the_flat_list_or_while_customizing_is_off_or_a_layer_is_up() {
            let (mut app, _store, _rx) = menu_app();
            let _ = app.update(Message::SearchChanged("chan".to_owned()));
            send(&mut app, M::OpenRoom(rid("chan1")));
            send(&mut app, M::OpenSection(SectionKey::Channels));
            assert!(app.sidebar_menu.is_none(), "a search keeps the flat list");
            let _ = app.update(Message::SearchChanged(String::new()));
            let (mut app2, _store2, _rx2) = menu_app();
            app2.switcher.open = true;
            send(&mut app2, M::OpenRoom(rid("chan1")));
            assert!(app2.sidebar_menu.is_none(), "the switcher is over the list");
            let (mut app3, _store3, _rx3) = menu_app();
            let _ = app3.update(Message::SidebarLoaded(SidebarAnswer::new(
                UserId::from("@a:x"),
                SidebarLoad::Newer,
            )));
            app3.sidebar.unload();
            send(&mut app3, M::OpenRoom(rid("chan1")));
            assert!(app3.sidebar_menu.is_none(), "no layout loaded");
        }

        #[test]
        fn star_on_a_rooms_card_toggles_its_star_and_saves_once() {
            let (mut app, store, _rx) = menu_app();
            send(&mut app, M::OpenRoom(rid("chan2")));
            entry(&mut app, Entry::Star);
            assert!(app.sidebar_menu.is_none(), "the card closes");
            assert_eq!(saves(&store), 1);
            assert_eq!(
                &drawn_names(&app)[..3],
                ["# Starred", "chan2", "# Channels"]
            );
            // The card of a starred room offers Unstar.
            send(&mut app, M::OpenRoom(rid("chan2")));
            entry(&mut app, Entry::Unstar);
            assert_eq!(saves(&store), 2);
            assert_eq!(drawn_names(&app)[0], "# Channels");
            assert!(drawn_names(&app).iter().all(|l| l != "# Starred"));
        }

        #[test]
        fn a_new_section_from_a_room_holds_the_room_and_survives_in_the_layout() {
            let (mut app, store, _rx) = menu_app();
            send(&mut app, M::OpenRoom(rid("chan2")));
            send(&mut app, M::Section(SectionPick::New));
            assert!(matches!(
                &app.sidebar_menu,
                Some(SidebarMenu::Name(d)) if matches!(d.purpose, Purpose::NewSection { room: Some(_), .. })
            ));
            name(&mut app, "  Ops ");
            assert!(app.sidebar_menu.is_none());
            assert_eq!(saves(&store), 1);
            let out = drawn_names(&app);
            let at = out.iter().position(|l| l == "# Ops").expect("Ops header");
            assert_eq!(out[at + 1], "chan2");
            let saved = record(&app);
            assert_eq!(saved.custom[0].name.as_str(), "Ops");
            assert_eq!(saved.custom[0].rooms, [rid("chan2")]);
            // Ops sits before Direct messages (its default place).
            assert!(
                at < out
                    .iter()
                    .position(|l| l == "# Direct messages")
                    .unwrap_or(usize::MAX)
            );
        }

        #[test]
        fn a_new_section_from_a_header_is_empty_and_shows_its_hint() {
            let (mut app, store, _rx) = menu_app();
            send(&mut app, M::OpenSection(SectionKey::Channels));
            entry(&mut app, Entry::NewSection);
            name(&mut app, "Later");
            assert_eq!(saves(&store), 1);
            let out = drawn_names(&app);
            let at = out.iter().position(|l| l == "# Later").expect("header");
            assert_eq!(out[at + 1], "~hint");
        }

        #[test]
        fn a_blank_or_bad_section_name_keeps_the_dialog_open_with_the_reason() {
            let (mut app, store, _rx) = menu_app();
            send(&mut app, M::OpenSection(SectionKey::Channels));
            entry(&mut app, Entry::NewSection);
            for bad in ["", "   ", &"x".repeat(33), "a\nb"] {
                name(&mut app, bad);
                match &app.sidebar_menu {
                    Some(SidebarMenu::Name(d)) => assert!(d.error.is_some(), "{bad:?}"),
                    other => panic!("the dialog closed: {other:?}"),
                }
            }
            assert_eq!(saves(&store), 0);
            send(&mut app, M::Escape);
            assert!(app.sidebar_menu.is_none());
        }

        #[test]
        fn rename_keeps_the_rooms_and_move_down_in_a_section_moves_only_inside_it() {
            let (mut app, store, _rx) = menu_app();
            make_section(&mut app, "Ops", &["chan1", "chan2"]);
            assert_eq!(saves(&store), 2);
            let before = record(&app);
            // Move down on chan1 (the first of the section).
            send(&mut app, M::OpenRoom(rid("chan1")));
            entry(&mut app, Entry::MoveDown);
            assert_eq!(saves(&store), 3);
            let after = record(&app);
            assert_eq!(after.custom[0].rooms, [rid("chan2"), rid("chan1")]);
            assert_eq!(after.order, before.order);
            assert_eq!(after.starred, before.starred);
            assert_eq!(after.collapsed, before.collapsed);
            // The last room cannot go down: no save.
            send(&mut app, M::OpenRoom(rid("chan1")));
            entry(&mut app, Entry::MoveDown);
            assert_eq!(saves(&store), 3);
            // Rename: the rooms stay.
            send(&mut app, M::OpenSection(SectionKey::Custom(0)));
            entry(&mut app, Entry::Rename);
            match &app.sidebar_menu {
                Some(SidebarMenu::Name(d)) => assert_eq!(d.text, "Ops", "the field has the name"),
                other => panic!("no dialog: {other:?}"),
            }
            name(&mut app, "Work");
            assert_eq!(record(&app).custom[0].name.as_str(), "Work");
            assert_eq!(record(&app).custom[0].rooms, [rid("chan2"), rid("chan1")]);
        }

        #[test]
        fn move_down_on_a_headers_card_moves_the_section_over_the_drawn_ones() {
            let (mut app, store, _rx) = menu_app();
            make_section(&mut app, "Ops", &["chan1"]);
            let heads = |app: &App| -> Vec<String> {
                drawn_names(app)
                    .into_iter()
                    .filter(|l| l.starts_with('#'))
                    .collect()
            };
            // Channels, tech, voice, Ops, Direct messages.
            assert_eq!(
                heads(&app),
                [
                    "# Channels",
                    "# tech",
                    "# voice",
                    "# Ops",
                    "# Direct messages"
                ]
            );
            let saved = saves(&store);
            send(&mut app, M::OpenSection(SectionKey::Custom(0)));
            entry(&mut app, Entry::MoveUp);
            assert_eq!(saves(&store), saved + 1);
            assert_eq!(
                heads(&app),
                [
                    "# Channels",
                    "# tech",
                    "# Ops",
                    "# voice",
                    "# Direct messages"
                ]
            );
            // Down from the last drawn section does nothing and saves nothing.
            send(&mut app, M::OpenSection(SectionKey::Direct));
            entry(&mut app, Entry::MoveDown);
            assert_eq!(saves(&store), saved + 1);
            // Starred is hidden while empty: Ops moved to the top steps over
            // it and a later star puts Starred where the order says.
            for _ in 0..3 {
                send(&mut app, M::OpenSection(SectionKey::Custom(0)));
                entry(&mut app, Entry::MoveUp);
            }
            assert_eq!(heads(&app)[0], "# Ops");
        }

        #[test]
        fn delete_returns_the_rooms_to_their_default_sections() {
            let (mut app, store, _rx) = menu_app();
            make_section(&mut app, "Ops", &["chan1", "t1"]);
            send(&mut app, M::OpenSection(SectionKey::Custom(0)));
            entry(&mut app, Entry::Collapse);
            send(&mut app, M::OpenSection(SectionKey::Custom(0)));
            entry(&mut app, Entry::MoveUp);
            let saved = saves(&store);
            send(&mut app, M::OpenSection(SectionKey::Custom(0)));
            entry(&mut app, Entry::Delete);
            assert_eq!(saves(&store), saved + 1);
            let out = drawn_names(&app);
            assert!(out.iter().all(|l| l != "# Ops"));
            let at = |name: &str| out.iter().position(|l| l == name).expect("drawn");
            assert!(at("# Channels") < at("chan1") && at("chan1") < at("# tech"));
            assert!(at("# tech") < at("t1") && at("t1") < at("# voice"));
            let rec = record(&app);
            assert!(rec.custom.is_empty());
            assert!(!rec.order.contains(&SectionKey::Custom(0)));
            assert!(!rec.collapsed.contains(&SectionKey::Custom(0)));
        }

        #[test]
        fn back_to_default_and_collapse_and_expand_work_from_the_cards() {
            let (mut app, _store, _rx) = menu_app();
            make_section(&mut app, "Ops", &["chan1"]);
            send(&mut app, M::OpenRoom(rid("chan1")));
            entry(&mut app, Entry::BackToDefault);
            assert!(record(&app).custom[0].rooms.is_empty());
            assert_eq!(
                drawn_names(&app).iter().filter(|l| *l == "~hint").count(),
                1
            );
            send(&mut app, M::OpenSection(SectionKey::Channels));
            entry(&mut app, Entry::Collapse);
            assert!(record(&app).collapsed.contains(&SectionKey::Channels));
            send(&mut app, M::OpenSection(SectionKey::Channels));
            entry(&mut app, Entry::Expand);
            assert!(!record(&app).collapsed.contains(&SectionKey::Channels));
        }

        /// Puts these tags on the rooms of the list (name, favourite, low
        /// priority) and feeds the list again.
        fn tag_rooms(app: &mut App, tags: &[(&str, bool, bool)]) {
            let mut rs = rooms();
            for (name, favourite, low) in tags {
                let r = rs
                    .iter_mut()
                    .find(|r| r.name == *name)
                    .expect("a room of the fixture");
                r.is_favourite = *favourite;
                r.is_low_priority = *low;
            }
            feed(
                app,
                CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: rs }]),
            );
        }

        fn tag_commands(rx: &mut mpsc::Receiver<Command>) -> Vec<Command> {
            drain(rx)
                .into_iter()
                .filter(|c| matches!(c, Command::SetRoomTag { .. }))
                .collect()
        }

        #[test]
        fn the_favourite_and_low_priority_entries_send_one_set_room_tag_and_save_no_file() {
            use twicecat_core::api::RoomTagView;
            let (mut app, store, mut rx) = menu_app();
            let _ = drain(&mut rx);
            let cases = [
                (Entry::AddFavourite, Some(RoomTagView::Favourite)),
                (Entry::RemoveFavourite, None),
                (Entry::MarkLowPriority, Some(RoomTagView::LowPriority)),
                (Entry::RemoveLowPriority, None),
            ];
            for (entry_pressed, tag) in cases {
                send(&mut app, M::OpenRoom(rid("chan1")));
                entry(&mut app, entry_pressed);
                assert!(app.sidebar_menu.is_none(), "{entry_pressed:?}: card closes");
                let sent = tag_commands(&mut rx);
                assert_eq!(sent.len(), 1, "{entry_pressed:?}: exactly one command");
                assert!(
                    matches!(
                        &sent[0],
                        Command::SetRoomTag { room_id, tag: t }
                            if *room_id == rid("chan1") && *t == tag
                    ),
                    "{entry_pressed:?}: {:?}",
                    sent[0]
                );
            }
            assert_eq!(saves(&store), 0, "a tag action saves no sidebar file");
        }

        #[test]
        fn the_tag_entries_show_the_wording_of_the_rooms_current_flags() {
            use crate::sidebar_menu::tag_entries;
            let table = [
                // (favourite, low priority) -> the two buttons
                (
                    (false, false),
                    ["Add to favourites", "Mark as low priority"],
                ),
                (
                    (true, false),
                    ["Remove from favourites", "Mark as low priority"],
                ),
                ((false, true), ["Add to favourites", "Remove low priority"]),
                // A server cannot hold both; the wording still follows each flag.
                (
                    (true, true),
                    ["Remove from favourites", "Remove low priority"],
                ),
            ];
            for ((favourite, low), words) in table {
                let labels = tag_entries(favourite, low).map(Entry::label);
                assert_eq!(labels, words, "favourite {favourite}, low priority {low}");
            }
        }

        #[test]
        fn a_tag_entry_on_a_card_that_is_gone_sends_nothing() {
            let (mut app, _store, mut rx) = menu_app();
            let _ = drain(&mut rx);
            entry(&mut app, Entry::AddFavourite);
            assert!(tag_commands(&mut rx).is_empty(), "no card open");
            // A section's card has no tag entries.
            send(&mut app, M::OpenSection(SectionKey::Channels));
            entry(&mut app, Entry::AddFavourite);
            assert!(tag_commands(&mut rx).is_empty());
        }

        // ---- the card of a row of the flat list (sidebar rework, Task 9b) ----

        /// The app on Home: the flat list.
        fn home_app() -> (App, MemorySidebar, mpsc::Receiver<Command>) {
            let (mut app, store, rx) = menu_app();
            let _ = app.update(Message::Spaces(SpacesMessage::Select(None)));
            assert!(!app.sections_shown());
            (app, store, rx)
        }

        fn flat_card_room(app: &App) -> Option<RoomId> {
            match &app.sidebar_menu {
                Some(SidebarMenu::Card {
                    target: Target::FlatRoom(room),
                    ..
                }) => Some(room.clone()),
                _ => None,
            }
        }

        #[test]
        fn a_right_click_on_a_flat_row_opens_its_card_under_home_a_chip_and_a_search() {
            let (mut app, _store, _rx) = home_app();
            send(&mut app, M::OpenFlatRoom(rid("chan1")));
            assert_eq!(flat_card_room(&app), Some(rid("chan1")), "Home");
            send(&mut app, M::Close);
            // A chip other than All, with a space selected.
            let _ = app.update(Message::Spaces(SpacesMessage::Select(Some(rid("sp")))));
            let _ = app.update(Message::ChipClicked(RoomListTab::Rooms));
            assert!(!app.sections_shown());
            send(&mut app, M::OpenFlatRoom(rid("chan2")));
            assert_eq!(flat_card_room(&app), Some(rid("chan2")), "a chip");
            send(&mut app, M::Close);
            // A search result.
            let _ = app.update(Message::ChipClicked(RoomListTab::All));
            let _ = app.update(Message::SearchChanged("t1".to_owned()));
            send(&mut app, M::OpenFlatRoom(rid("t1")));
            assert_eq!(flat_card_room(&app), Some(rid("t1")), "a search");
        }

        #[test]
        fn the_flat_card_does_not_open_where_the_list_is_sectioned_or_a_rail_or_for_a_stranger() {
            let (mut app, _store, _rx) = menu_app();
            assert!(app.sections_shown());
            send(&mut app, M::OpenFlatRoom(rid("chan1")));
            assert!(app.sidebar_menu.is_none(), "the sectioned list has its own");
            let _ = app.update(Message::Spaces(SpacesMessage::Select(None)));
            send(&mut app, M::OpenFlatRoom(rid("nobody")));
            assert!(app.sidebar_menu.is_none(), "a room that is not in the list");
            // The narrow rail has no card.
            app.ui_settings.room_list = RoomListChoice::Rail;
            send(&mut app, M::OpenFlatRoom(rid("chan1")));
            assert!(app.sidebar_menu.is_none(), "the rail");
        }

        #[test]
        fn the_flat_cards_entries_send_one_set_room_tag_close_it_and_save_no_file() {
            use twicecat_core::api::RoomTagView;
            let (mut app, store, mut rx) = home_app();
            let _ = drain(&mut rx);
            let cases = [
                (Entry::AddFavourite, Some(RoomTagView::Favourite)),
                (Entry::RemoveFavourite, None),
                (Entry::MarkLowPriority, Some(RoomTagView::LowPriority)),
                (Entry::RemoveLowPriority, None),
            ];
            for (entry_pressed, tag) in cases {
                send(&mut app, M::OpenFlatRoom(rid("chan1")));
                entry(&mut app, entry_pressed);
                assert!(app.sidebar_menu.is_none(), "{entry_pressed:?}: card closes");
                let sent = tag_commands(&mut rx);
                assert_eq!(sent.len(), 1, "{entry_pressed:?}: exactly one command");
                assert!(
                    matches!(
                        &sent[0],
                        Command::SetRoomTag { room_id, tag: t }
                            if *room_id == rid("chan1") && *t == tag
                    ),
                    "{entry_pressed:?}: {:?}",
                    sent[0]
                );
            }
            assert_eq!(saves(&store), 0, "a tag action saves no sidebar file");
        }

        #[test]
        fn the_flat_card_offers_only_the_tags() {
            use crate::sidebar_menu::{tag_change, tag_entries};
            // The card is the Task 9 pair (its wording table is tested above)
            // and nothing else: both are tag buttons.
            for (favourite, low) in [(false, false), (true, false), (false, true), (true, true)] {
                let offered = tag_entries(favourite, low);
                assert!(offered.iter().all(|e| tag_change(*e).is_some()));
            }
            // A button that needs a section or a space does nothing here.
            let (mut app, store, mut rx) = home_app();
            let _ = drain(&mut rx);
            for stray in [
                Entry::Star,
                Entry::Unstar,
                Entry::MoveUp,
                Entry::MoveDown,
                Entry::BackToDefault,
                Entry::NewSection,
            ] {
                send(&mut app, M::OpenFlatRoom(rid("chan1")));
                entry(&mut app, stray);
                assert!(drain(&mut rx).is_empty(), "{stray:?}");
                assert_eq!(flat_card_room(&app), Some(rid("chan1")), "{stray:?}");
            }
            assert_eq!(saves(&store), 0);
        }

        #[test]
        fn a_flat_card_for_a_room_that_left_the_list_sends_nothing() {
            let (mut app, store, mut rx) = home_app();
            let _ = drain(&mut rx);
            // Pressed on a card whose room is not in the list as drawn.
            app.sidebar_menu = Some(SidebarMenu::Card {
                target: Target::FlatRoom(rid("gone")),
                error: None,
            });
            entry(&mut app, Entry::AddFavourite);
            assert!(tag_commands(&mut rx).is_empty());
            assert!(app.sidebar_menu.is_none(), "the card goes away");
            // The room leaves while the card is open: the card closes with it.
            send(&mut app, M::OpenFlatRoom(rid("chan1")));
            let rest: Vec<RoomSummary> =
                rooms().into_iter().filter(|r| r.name != "chan1").collect();
            feed(
                &mut app,
                CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: rest }]),
            );
            assert!(app.sidebar_menu.is_none());
            entry(&mut app, Entry::AddFavourite);
            assert!(tag_commands(&mut rx).is_empty());
            assert_eq!(saves(&store), 0);
        }

        #[test]
        fn a_flat_card_closes_when_the_list_turns_sectioned() {
            let (mut app, _store, _rx) = home_app();
            send(&mut app, M::OpenFlatRoom(rid("chan1")));
            let _ = app.update(Message::Spaces(SpacesMessage::Select(Some(rid("sp")))));
            assert!(app.sidebar_menu.is_none(), "the list is sectioned now");
        }

        #[test]
        fn under_the_favourites_chip_remove_from_favourites_removes_the_tag_and_the_room_leaves() {
            let (mut app, store, mut rx) = home_app();
            tag_rooms(&mut app, &[("chan1", true, false), ("chan2", true, false)]);
            let _ = app.update(Message::ChipClicked(RoomListTab::Favourites));
            // The core answers the chip's query with the favourites only.
            let mut only: Vec<RoomSummary> = rooms()
                .into_iter()
                .filter(|r| r.name.starts_with("chan"))
                .collect();
            for r in &mut only {
                r.is_favourite = true;
            }
            feed(
                &mut app,
                CoreEvent::RoomListChanged(vec![ListDiff::Reset {
                    values: only.clone(),
                }]),
            );
            let _ = drain(&mut rx);
            send(&mut app, M::OpenFlatRoom(rid("chan1")));
            let labels = crate::sidebar_menu::tag_entries(true, false).map(Entry::label);
            assert_eq!(labels[0], "Remove from favourites");
            entry(&mut app, Entry::RemoveFavourite);
            let sent = tag_commands(&mut rx);
            assert_eq!(sent.len(), 1);
            assert!(matches!(
                &sent[0],
                Command::SetRoomTag { room_id, tag: None } if *room_id == rid("chan1")
            ));
            assert!(app.sidebar_menu.is_none());
            // The next room-list event drops the room from that list.
            only.retain(|r| r.name != "chan1");
            feed(
                &mut app,
                CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: only }]),
            );
            assert!(app.rooms.iter().all(|r| r.name != "chan1"));
            assert_eq!(saves(&store), 0);
        }

        #[test]
        fn the_flat_target_hides_the_room_id_in_debug() {
            let text = format!("{:?}", Target::FlatRoom(rid("!secretroom:x")));
            assert!(!text.contains("secretroom"), "{text}");
            let text = format!("{:?}", M::OpenFlatRoom(rid("!secretroom:x")));
            assert!(!text.contains("secretroom"), "{text}");
        }

        #[test]
        fn a_tagged_room_moves_into_its_section_when_the_core_reports_the_tag() {
            let (mut app, _store, _rx) = menu_app();
            tag_rooms(&mut app, &[("chan1", true, false), ("chan2", false, true)]);
            let out = drawn_names(&app);
            assert_eq!(out[0], "# Favourites");
            assert_eq!(out[1], "chan1");
            assert_eq!(out.last().map(String::as_str), Some("chan2"));
            assert!(out.iter().any(|l| l == "# Low priority"));
            // Its card still opens from the new section.
            send(&mut app, M::OpenRoom(rid("chan1")));
            assert!(matches!(
                &app.sidebar_menu,
                Some(SidebarMenu::Card {
                    target: Target::Room { .. },
                    ..
                })
            ));
        }

        #[test]
        fn collapse_and_move_on_the_new_headers_save_like_the_other_built_in_sections() {
            let (mut app, store, _rx) = menu_app();
            tag_rooms(&mut app, &[("chan1", true, false), ("chan2", false, true)]);
            let heads = |app: &App| -> Vec<String> {
                drawn_names(app)
                    .into_iter()
                    .filter(|l| l.starts_with('#'))
                    .collect()
            };
            assert_eq!(
                heads(&app),
                [
                    "# Favourites",
                    "# Channels",
                    "# tech",
                    "# voice",
                    "# Direct messages",
                    "# Low priority"
                ]
            );
            for key in [SectionKey::Favourites, SectionKey::LowPriority] {
                let before = saves(&store);
                send(&mut app, M::OpenSection(key.clone()));
                entry(&mut app, Entry::Collapse);
                assert_eq!(saves(&store), before + 1, "collapse saves once");
                assert!(record(&app).collapsed.contains(&key));
                send(&mut app, M::OpenSection(key.clone()));
                entry(&mut app, Entry::Expand);
                assert_eq!(saves(&store), before + 2, "expand saves once");
                assert!(!record(&app).collapsed.contains(&key));
            }
            // Move down on Favourites swaps it with Channels; Move up on Low
            // priority swaps it with Direct messages.
            let before = saves(&store);
            send(&mut app, M::OpenSection(SectionKey::Favourites));
            entry(&mut app, Entry::MoveDown);
            send(&mut app, M::OpenSection(SectionKey::LowPriority));
            entry(&mut app, Entry::MoveUp);
            assert_eq!(saves(&store), before + 2);
            assert_eq!(
                heads(&app),
                [
                    "# Channels",
                    "# Favourites",
                    "# tech",
                    "# voice",
                    "# Low priority",
                    "# Direct messages"
                ]
            );
            let order = record(&app).order;
            assert!(order.contains(&SectionKey::Favourites));
            assert!(order.contains(&SectionKey::LowPriority));
            // The first and the last cannot go further and save nothing.
            let before = saves(&store);
            send(&mut app, M::OpenSection(SectionKey::Channels));
            entry(&mut app, Entry::MoveUp);
            send(&mut app, M::OpenSection(SectionKey::Direct));
            entry(&mut app, Entry::MoveDown);
            assert_eq!(saves(&store), before);
        }

        #[test]
        fn a_21st_section_is_refused_with_the_reason_and_the_dialog_stays() {
            let (mut app, store, _rx) = menu_app();
            for n in 0..MAX_CUSTOM_SECTIONS {
                send(&mut app, M::OpenSection(SectionKey::Channels));
                entry(&mut app, Entry::NewSection);
                name(&mut app, &format!("S{n}"));
                assert!(app.sidebar_menu.is_none(), "section {n}");
            }
            let saved = saves(&store);
            send(&mut app, M::OpenSection(SectionKey::Channels));
            entry(&mut app, Entry::NewSection);
            name(&mut app, "One more");
            match &app.sidebar_menu {
                Some(SidebarMenu::Name(d)) => {
                    assert_eq!(d.error, Some("The sidebar layout is full"));
                }
                other => panic!("the dialog closed: {other:?}"),
            }
            assert_eq!(saves(&store), saved);
            assert_eq!(record(&app).custom.len(), MAX_CUSTOM_SECTIONS);
        }

        #[test]
        fn esc_closes_the_card_and_a_card_for_a_room_that_left_closes_itself() {
            let (mut app, _store, _rx) = menu_app();
            send(&mut app, M::OpenRoom(rid("chan1")));
            assert!(app.sidebar_menu.is_some());
            let _ = app.update(Message::Shortcut {
                shortcut: Shortcut::Escape,
                captured: false,
            });
            assert!(app.sidebar_menu.is_none());
            send(&mut app, M::OpenRoom(rid("chan1")));
            let rest: Vec<RoomSummary> = rooms()
                .into_iter()
                .filter(|r| r.id != rid("chan1"))
                .collect();
            feed(
                &mut app,
                CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: rest }]),
            );
            assert!(app.sidebar_menu.is_none(), "the room is not in the list");
            // A card of a section closes when its section is collapsed away
            // by a search that makes the list flat.
            send(&mut app, M::OpenSection(SectionKey::Channels));
            let _ = app.update(Message::SearchChanged("x".to_owned()));
            let _ = app.update(Message::Spaces(SpacesMessage::Select(None)));
            assert!(app.sidebar_menu.is_none());
        }
    }

    // ---- dragging rooms and sections (Task sbc.10) ----

    mod dragging {
        use iced::Point;

        use super::*;
        use crate::drag::Zone;
        use crate::keys::Shortcut;
        use crate::list_drop::ListKey;
        use crate::sidebar_menu::{Entry, SectionPick, SidebarMenuMessage as M};
        use crate::sidebar_store::Call;
        use crate::spaces::SpacesMessage as S;
        use twicecat_core::api::RoomTagView;

        fn drag_app() -> (App, MemorySidebar, mpsc::Receiver<Command>) {
            let (mut app, rx) = space_app(None);
            let store = MemorySidebar::default();
            app.set_sidebar_store(Box::new(store.clone()));
            (app, store, rx)
        }

        /// An app whose layout has "Ops" (0) holding chan1 and t2 in that order,
        /// "Later" (1) empty, and starred1 starred. chan2 and t1 are free.
        fn arranged() -> (App, MemorySidebar, mpsc::Receiver<Command>) {
            let (mut app, store, rx) = drag_app();
            let label = |t: &str| SidebarLabel::try_from(t).expect("a name");
            for op in [
                Op::NewSection {
                    space: rid("sp"),
                    name: label("Ops"),
                    room: Some(rid("chan1")),
                },
                Op::MoveRoomToSection {
                    space: rid("sp"),
                    room: rid("t2"),
                    section: 0,
                },
                Op::NewSection {
                    space: rid("sp"),
                    name: label("Later"),
                    room: None,
                },
                Op::Star {
                    space: rid("sp"),
                    room: rid("starred1"),
                },
            ] {
                assert!(app.apply_sidebar(op).is_ok());
            }
            (app, store, rx)
        }

        fn saves(store: &MemorySidebar) -> usize {
            store
                .calls()
                .iter()
                .filter(|c| matches!(c, Call::Save(..)))
                .count()
        }

        fn record(app: &App) -> SpaceLayout {
            app.sidebar
                .layout()
                .and_then(|l| l.spaces.iter().find(|s| s.space == rid("sp")))
                .cloned()
                .expect("a record for the space")
        }

        fn send(app: &mut App, message: S) {
            let _ = app.update(Message::Spaces(message));
        }

        fn menu(app: &mut App, message: M) {
            let _ = app.update(Message::SidebarMenu(message));
        }

        fn room(n: &str) -> ListKey {
            ListKey::Room(rid(n))
        }

        fn head(key: SectionKey) -> ListKey {
            ListKey::Header(key)
        }

        /// Presses `item` and moves 30 px: a drag is running.
        fn start(app: &mut App, item: &ListKey) {
            send(app, S::ListEnter(item.clone()));
            send(app, S::ListPress(item.clone()));
            send(app, S::PointerMoved(Point::new(10.0, 10.0)));
            send(app, S::PointerMoved(Point::new(40.0, 10.0)));
            assert!(app.list_drag.is_dragging());
        }

        /// Drags `item` to `target` and lets go.
        fn drag(app: &mut App, item: &ListKey, target: &ListKey, zone: Zone) {
            start(app, item);
            send(app, S::ListOver(target.clone(), zone));
            send(app, S::PointerReleased);
            assert!(!app.list_drag.is_active());
        }

        /// Every room the layout keeps, in its sections of the user's own and
        /// Starred, as (section, rooms) pairs.
        fn stored(app: &App) -> Vec<(String, Vec<RoomId>)> {
            let rec = record(app);
            let mut all = vec![("starred".to_owned(), rec.starred.clone())];
            all.extend(
                rec.custom
                    .iter()
                    .map(|c| (format!("custom{}", c.id), c.rooms.clone())),
            );
            all
        }

        fn every_room(app: &App) -> Vec<RoomId> {
            let mut rooms: Vec<RoomId> = stored(app).into_iter().flat_map(|(_, r)| r).collect();
            rooms.sort();
            rooms
        }

        fn without(rooms: &[RoomId], kept: &[&str]) -> Vec<RoomId> {
            rooms
                .iter()
                .filter(|r| !kept.iter().any(|k| **r == rid(k)))
                .cloned()
                .collect()
        }

        /// "Ops" (0) holds, in this order, chan1, t1, chan2, t2 and rustroom;
        /// t1 is the open room and t2 is the room of this device's call. It is
        /// collapsed, so only t1 and t2 are drawn.
        fn collapsed_custom() -> (App, MemorySidebar, mpsc::Receiver<Command>) {
            let (mut app, store, rx) = drag_app();
            let label = SidebarLabel::try_from("Ops").expect("a name");
            assert!(
                app.apply_sidebar(Op::NewSection {
                    space: rid("sp"),
                    name: label,
                    room: Some(rid("chan1")),
                })
                .is_ok()
            );
            for r in ["t1", "chan2", "t2", "rustroom"] {
                assert!(
                    app.apply_sidebar(Op::MoveRoomToSection {
                        space: rid("sp"),
                        room: rid(r),
                        section: 0,
                    })
                    .is_ok()
                );
            }
            let _ = app.update(Message::SelectRoom(rid("t1")));
            app.calls.call = Some(crate::calls::CurrentCall::new(
                rid("t2"),
                crate::calls::CallPhase::Joining,
            ));
            collapse(&mut app, SectionKey::Custom(0));
            assert_eq!(
                drawn_names(&app)
                    .into_iter()
                    .skip_while(|l| l != "# Ops")
                    .take(3)
                    .collect::<Vec<_>>(),
                ["# Ops", "t1", "t2"]
            );
            (app, store, rx)
        }

        #[test]
        fn dragging_a_kept_room_in_a_collapsed_custom_section_keeps_the_hidden_rooms_in_place() {
            let hidden = ["chan1", "chan2", "rustroom"];
            let before = {
                let (app, ..) = collapsed_custom();
                let ops: Vec<RoomId> = record(&app).custom[0].rooms.clone();
                assert_eq!(ops.len(), 5);
                ops
            };
            let hidden_ids = without(&before, &["t1", "t2"]);
            assert_eq!(hidden_ids, hidden.map(rid));
            // Beside the other kept room, both ways, and onto its own header.
            for (item, target, zone) in [
                (room("t1"), room("t2"), Zone::After),
                (room("t2"), room("t1"), Zone::Before),
                (room("t1"), head(SectionKey::Custom(0)), Zone::Into),
            ] {
                let (mut app, ..) = collapsed_custom();
                drag(&mut app, &item, &target, zone);
                let now = record(&app).custom[0].rooms.clone();
                assert_eq!(now.len(), 5, "no room is lost or doubled: {now:?}");
                assert_eq!(
                    without(&now, &["t1", "t2"]),
                    hidden_ids,
                    "hidden rooms stay"
                );
            }
            // Out of the collapsed section, into another: the hidden rooms stay.
            let (mut app, ..) = collapsed_custom();
            let later = SidebarLabel::try_from("Later").expect("a name");
            assert!(
                app.apply_sidebar(Op::NewSection {
                    space: rid("sp"),
                    name: later,
                    room: None,
                })
                .is_ok()
            );
            let all = every_room(&app);
            drag(
                &mut app,
                &room("t1"),
                &head(SectionKey::Custom(1)),
                Zone::Into,
            );
            assert_eq!(record(&app).custom[1].rooms, [rid("t1")]);
            let now = record(&app).custom[0].rooms.clone();
            assert_eq!(now, without(&before, &["t1"]), "the rest keeps its order");
            assert_eq!(every_room(&app), all, "no room lost or doubled");
        }

        #[test]
        fn dropping_on_a_collapsed_custom_section_loses_no_room() {
            let (mut app, ..) = collapsed_custom();
            let before = record(&app).custom[0].rooms.clone();
            // A free room onto the collapsed header, and onto its kept row.
            drag(
                &mut app,
                &room("starred1"),
                &head(SectionKey::Custom(0)),
                Zone::Into,
            );
            let after_header = record(&app).custom[0].rooms.clone();
            assert_eq!(&after_header[..5], before.as_slice());
            assert_eq!(after_header.len(), 6);
            drag(&mut app, &room("dm1"), &room("t1"), Zone::After);
            let now = record(&app).custom[0].rooms.clone();
            assert_eq!(now.len(), 7, "{now:?}");
            assert_eq!(
                without(&now, &["dm1", "t1", "t2", "starred1"]),
                without(&before, &["t1", "t2"]),
                "the hidden rooms keep their order"
            );
        }

        #[test]
        fn dragging_the_kept_room_of_a_collapsed_starred_keeps_the_hidden_rooms_in_place() {
            let build = || {
                let (mut app, store, rx) = drag_app();
                let label = SidebarLabel::try_from("Ops").expect("a name");
                assert!(
                    app.apply_sidebar(Op::NewSection {
                        space: rid("sp"),
                        name: label,
                        room: None,
                    })
                    .is_ok()
                );
                for r in ["chan1", "chan2", "t2"] {
                    assert!(
                        app.apply_sidebar(Op::Star {
                            space: rid("sp"),
                            room: rid(r),
                        })
                        .is_ok()
                    );
                }
                let _ = app.update(Message::SelectRoom(rid("chan2")));
                collapse(&mut app, SectionKey::Starred);
                let drawn = drawn_names(&app);
                assert_eq!(&drawn[..2], ["# Starred", "chan2"], "{drawn:?}");
                (app, store, rx)
            };
            let hidden = [rid("chan1"), rid("t2")];
            // Onto its own header, onto Channels (back to default), onto Ops.
            for (target, expect_starred) in [
                (head(SectionKey::Starred), vec!["chan1", "chan2", "t2"]),
                (head(SectionKey::Channels), vec!["chan1", "t2"]),
                (head(SectionKey::Custom(0)), vec!["chan1", "t2"]),
            ] {
                let (mut app, ..) = build();
                drag(&mut app, &room("chan2"), &target, Zone::Into);
                let rec = record(&app);
                let want: Vec<RoomId> = expect_starred.iter().map(|r| rid(r)).collect();
                assert_eq!(rec.starred, want, "{target:?}");
                assert_eq!(without(&rec.starred, &["chan2"]), hidden);
                let in_ops = rec.custom[0].rooms.contains(&rid("chan2"));
                assert_eq!(
                    in_ops,
                    matches!(target, ListKey::Header(SectionKey::Custom(0)))
                );
            }
        }

        /// How the drop of every kind of target compares with the card.
        #[test]
        fn every_kind_of_drop_gives_the_layout_of_the_matching_card_operation() {
            type Cards = fn(&mut App);
            type Row = (&'static str, ListKey, ListKey, Zone, Cards);
            let up_to_first = |app: &mut App| {
                // Direct messages up, one card Move up at a time.
                for _ in 0..8 {
                    menu(app, M::OpenSection(SectionKey::Direct));
                    menu(app, M::Entry(Entry::MoveUp));
                }
            };
            let table: Vec<Row> = vec![
                (
                    "a room onto Starred",
                    room("chan2"),
                    head(SectionKey::Starred),
                    Zone::Into,
                    |app| {
                        menu(app, M::OpenRoom(rid("chan2")));
                        menu(app, M::Entry(Entry::Star));
                    },
                ),
                (
                    "a room onto a section of the user's own",
                    room("t1"),
                    head(SectionKey::Custom(0)),
                    Zone::Into,
                    |app| {
                        menu(app, M::OpenRoom(rid("t1")));
                        menu(app, M::Section(SectionPick::Existing(0)));
                    },
                ),
                (
                    "a starred room onto a section of the user's own",
                    room("starred1"),
                    head(SectionKey::Custom(1)),
                    Zone::Into,
                    |app| {
                        menu(app, M::OpenRoom(rid("starred1")));
                        menu(app, M::Section(SectionPick::Existing(1)));
                    },
                ),
                (
                    "a room onto an empty section's hint (its header)",
                    room("chan2"),
                    head(SectionKey::Custom(1)),
                    Zone::Into,
                    |app| {
                        menu(app, M::OpenRoom(rid("chan2")));
                        menu(app, M::Section(SectionPick::Existing(1)));
                    },
                ),
                (
                    "a room of a section back onto Channels",
                    room("chan1"),
                    head(SectionKey::Channels),
                    Zone::Into,
                    |app| {
                        menu(app, M::OpenRoom(rid("chan1")));
                        menu(app, M::Entry(Entry::BackToDefault));
                    },
                ),
                (
                    "a starred room back onto its default section",
                    room("starred1"),
                    head(SectionKey::Channels),
                    Zone::Into,
                    |app| {
                        menu(app, M::OpenRoom(rid("starred1")));
                        menu(app, M::Entry(Entry::Unstar));
                    },
                ),
                (
                    "a room of a section back onto its sub-space",
                    room("t2"),
                    head(SectionKey::SubSpace(rid("tech"))),
                    Zone::Into,
                    |app| {
                        menu(app, M::OpenRoom(rid("t2")));
                        menu(app, M::Entry(Entry::BackToDefault));
                    },
                ),
                (
                    "a room before the first row of a section of the user's own",
                    room("chan2"),
                    room("chan1"),
                    Zone::Before,
                    |app| {
                        menu(app, M::OpenRoom(rid("chan2")));
                        menu(app, M::Section(SectionPick::Existing(0)));
                        for _ in 0..2 {
                            menu(app, M::OpenRoom(rid("chan2")));
                            menu(app, M::Entry(Entry::MoveUp));
                        }
                    },
                ),
                (
                    "a room between two rows of a section of the user's own",
                    room("chan2"),
                    room("chan1"),
                    Zone::After,
                    |app| {
                        menu(app, M::OpenRoom(rid("chan2")));
                        menu(app, M::Section(SectionPick::Existing(0)));
                        menu(app, M::OpenRoom(rid("chan2")));
                        menu(app, M::Entry(Entry::MoveUp));
                    },
                ),
                (
                    "a room of a section after the last row of the section",
                    room("chan1"),
                    room("t2"),
                    Zone::After,
                    |app| {
                        menu(app, M::OpenRoom(rid("chan1")));
                        menu(app, M::Entry(Entry::MoveDown));
                    },
                ),
                (
                    "a section header before the first header",
                    head(SectionKey::Direct),
                    head(SectionKey::Starred),
                    Zone::Before,
                    up_to_first,
                ),
                (
                    "a section header after the next one",
                    head(SectionKey::Custom(0)),
                    head(SectionKey::Custom(1)),
                    Zone::After,
                    |app| {
                        menu(app, M::OpenSection(SectionKey::Custom(0)));
                        menu(app, M::Entry(Entry::MoveDown));
                    },
                ),
            ];
            for (what, item, target, zone, cards) in table {
                let by_drag = {
                    let (mut app, store, _rx) = arranged();
                    let before = record(&app);
                    let saved = saves(&store);
                    drag(&mut app, &item, &target, zone);
                    assert_ne!(record(&app), before, "{what}: the drop did something");
                    assert_eq!(saves(&store), saved + 1, "{what}: one change, one save");
                    record(&app)
                };
                let by_card = {
                    let (mut app, store, _rx) = arranged();
                    let saved = saves(&store);
                    cards(&mut app);
                    assert!(saves(&store) > saved, "{what}: the card changed it");
                    record(&app)
                };
                assert_eq!(by_drag, by_card, "{what}");
            }
        }

        #[test]
        fn a_target_that_is_not_the_rooms_default_section_is_refused_and_drops_nothing() {
            let (mut app, store, _rx) = arranged();
            let before = record(&app);
            let saved = saves(&store);
            start(&mut app, &room("chan1"));
            send(&mut app, S::ListOver(head(SectionKey::Direct), Zone::Into));
            assert!(app.list_drag.target().is_none(), "no target");
            assert_eq!(
                app.list_drag.refused().map(|t| t.item.clone()),
                Some(head(SectionKey::Direct)),
                "drawn as not allowed"
            );
            send(&mut app, S::PointerReleased);
            assert_eq!(record(&app), before);
            assert_eq!(saves(&store), saved);
            assert!(!app.list_drag.is_active());
            // Leaving the place takes the refusal away.
            start(&mut app, &room("chan1"));
            send(&mut app, S::ListOver(head(SectionKey::Direct), Zone::Into));
            send(&mut app, S::ListExit(head(SectionKey::Direct)));
            assert!(app.list_drag.refused().is_none());
            // The sub-space header is refused for a Channels room too.
            send(
                &mut app,
                S::ListOver(head(SectionKey::SubSpace(rid("tech"))), Zone::Into),
            );
            assert!(app.list_drag.refused().is_some());
            send(&mut app, S::PointerReleased);
            assert_eq!(saves(&store), saved);
        }

        /// Puts these tags on the rooms of the list (name, favourite, low
        /// priority) and feeds the list again.
        fn tag_rooms(app: &mut App, tags: &[(&str, bool, bool)]) {
            let mut rs = rooms();
            for (name, favourite, low) in tags {
                let r = rs
                    .iter_mut()
                    .find(|r| r.name == *name)
                    .expect("a room of the fixture");
                r.is_favourite = *favourite;
                r.is_low_priority = *low;
            }
            feed(
                app,
                CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: rs }]),
            );
        }

        /// The tag commands in the channel, as (room, tag).
        fn tag_sent(rx: &mut mpsc::Receiver<Command>) -> Vec<(RoomId, Option<RoomTagView>)> {
            drain(rx)
                .into_iter()
                .filter_map(|c| match c {
                    Command::SetRoomTag { room_id, tag } => Some((room_id, tag)),
                    _ => None,
                })
                .collect()
        }

        /// Each tag drop sends exactly the command of the matching entry of
        /// the room's card, and saves no layout file.
        #[test]
        fn every_tag_drop_sends_the_command_of_the_matching_card_entry() {
            // "t1" is a favourite and "rustroom" is low priority, so both headers
            // are drawn.
            let tags = [("t1", true, false), ("rustroom", false, true)];
            let table = [
                ("chan2", SectionKey::Favourites, Entry::AddFavourite),
                ("chan2", SectionKey::LowPriority, Entry::MarkLowPriority),
                ("chan1", SectionKey::Favourites, Entry::AddFavourite),
                ("chan1", SectionKey::LowPriority, Entry::MarkLowPriority),
                ("t1", SectionKey::LowPriority, Entry::MarkLowPriority),
                ("rustroom", SectionKey::Favourites, Entry::AddFavourite),
                (
                    "t1",
                    SectionKey::SubSpace(rid("tech")),
                    Entry::RemoveFavourite,
                ),
                (
                    "rustroom",
                    SectionKey::SubSpace(rid("tech")),
                    Entry::RemoveLowPriority,
                ),
            ];
            let expected = [
                Some(RoomTagView::Favourite),
                Some(RoomTagView::LowPriority),
                Some(RoomTagView::Favourite),
                Some(RoomTagView::LowPriority),
                Some(RoomTagView::LowPriority),
                Some(RoomTagView::Favourite),
                None,
                None,
            ];
            for ((name, key, pressed), tag) in table.into_iter().zip(expected) {
                let (mut app, store, mut rx) = arranged();
                tag_rooms(&mut app, &tags);
                let _ = drain(&mut rx);
                let saved = saves(&store);
                drag(&mut app, &room(name), &head(key.clone()), Zone::Into);
                let by_drag = tag_sent(&mut rx);
                assert_eq!(saves(&store), saved, "{name} onto {key:?}: no file");
                menu(&mut app, M::OpenRoom(rid(name)));
                menu(&mut app, M::Entry(pressed));
                let by_card = tag_sent(&mut rx);
                assert_eq!(by_drag, by_card, "{name} onto {key:?}");
                assert_eq!(by_drag, [(rid(name), tag)], "{name} onto {key:?}");
            }
        }

        #[test]
        fn a_room_already_in_favourites_dropped_on_favourites_does_nothing() {
            let (mut app, store, mut rx) = arranged();
            tag_rooms(&mut app, &[("t1", true, false), ("rustroom", false, true)]);
            let _ = drain(&mut rx);
            let (before, saved) = (record(&app), saves(&store));
            for (name, key) in [
                ("t1", SectionKey::Favourites),
                ("rustroom", SectionKey::LowPriority),
            ] {
                start(&mut app, &room(name));
                send(&mut app, S::ListOver(head(key.clone()), Zone::Into));
                assert!(app.list_drag.target().is_none(), "{key:?}: no target");
                assert!(app.list_drag.refused().is_none(), "{key:?}: not refused");
                send(&mut app, S::PointerReleased);
            }
            assert!(drain(&mut rx).is_empty(), "nothing is sent");
            assert_eq!(record(&app), before);
            assert_eq!(saves(&store), saved);
        }

        #[test]
        fn a_starred_room_on_low_priority_is_refused_and_sends_nothing() {
            let (mut app, store, mut rx) = arranged();
            tag_rooms(&mut app, &[("dm1", false, true)]);
            let _ = drain(&mut rx);
            let saved = saves(&store);
            start(&mut app, &room("starred1"));
            send(
                &mut app,
                S::ListOver(head(SectionKey::LowPriority), Zone::Into),
            );
            assert!(app.list_drag.target().is_none(), "no target");
            assert_eq!(
                app.list_drag.refused().map(|t| t.item.clone()),
                Some(head(SectionKey::LowPriority)),
                "drawn as not allowed"
            );
            send(&mut app, S::PointerReleased);
            assert!(drain(&mut rx).is_empty());
            assert_eq!(saves(&store), saved);
        }

        #[test]
        fn a_tag_drop_keeps_a_target_ring_and_a_default_drop_of_an_untagged_room_stays_nothing() {
            let (mut app, _store, mut rx) = arranged();
            tag_rooms(&mut app, &[("t1", true, false)]);
            let _ = drain(&mut rx);
            start(&mut app, &room("chan2"));
            send(
                &mut app,
                S::ListOver(head(SectionKey::Favourites), Zone::Into),
            );
            assert_eq!(
                app.list_drag.target().map(|t| t.item.clone()),
                Some(head(SectionKey::Favourites)),
                "the ring shows"
            );
            send(&mut app, S::PointerReleased);
            // An untagged room on its own default section: nothing.
            start(&mut app, &room("chan2"));
            send(
                &mut app,
                S::ListOver(head(SectionKey::Channels), Zone::Into),
            );
            assert!(app.list_drag.target().is_none() && app.list_drag.refused().is_none());
            send(&mut app, S::PointerReleased);
            assert_eq!(tag_sent(&mut rx).len(), 1, "only the Favourites drop sent");
        }

        /// A room dropped on a row of Channels does what the drop on the
        /// Channels header does, and saves once.
        #[test]
        fn a_room_dropped_on_a_row_of_a_built_in_section_does_what_its_header_does() {
            // chan1 is in "Ops"; chan2 is a row of Channels.
            let (mut by_row, store, _rx) = arranged();
            let saved = saves(&store);
            start(&mut by_row, &room("chan1"));
            send(&mut by_row, S::ListOver(room("chan2"), Zone::After));
            assert_eq!(
                by_row.list_drag.target().map(|t| (t.item.clone(), t.zone)),
                Some((room("chan2"), Zone::Into)),
                "the whole row is the target: no before/after line"
            );
            send(&mut by_row, S::PointerReleased);
            assert_eq!(saves(&store), saved + 1, "saved once");
            let (mut by_head, _s, _r) = arranged();
            drag(
                &mut by_head,
                &room("chan1"),
                &head(SectionKey::Channels),
                Zone::Into,
            );
            assert_eq!(record(&by_row), record(&by_head));
            assert!(
                record(&by_row).custom[0]
                    .rooms
                    .iter()
                    .all(|r| *r != rid("chan1"))
            );
            // A DM on a row of Channels is refused and drawn so.
            let (mut app, store, _rx) = arranged();
            let saved = saves(&store);
            start(&mut app, &room("dm1"));
            send(&mut app, S::ListOver(room("chan2"), Zone::Before));
            assert!(app.list_drag.target().is_none());
            assert_eq!(
                app.list_drag.refused().map(|t| t.item.clone()),
                Some(room("chan2"))
            );
            send(&mut app, S::PointerReleased);
            assert_eq!(saves(&store), saved);
        }

        /// The pointer in the bottom half of a row and in the top half of the
        /// next is one target, and one row shows the line.
        #[test]
        fn a_gap_between_two_rows_is_one_target_and_one_line() {
            let (mut app, _store, _rx) = arranged();
            // "Ops" holds chan1 and t2, in that order.
            assert!(
                app.apply_sidebar(Op::MoveRoomToSection {
                    space: rid("sp"),
                    room: rid("chan2"),
                    section: 0
                })
                .is_ok()
            );
            let order: Vec<RoomId> = record(&app).custom[0].rooms.clone();
            assert_eq!(order.len(), 3);
            start(&mut app, &room("t1"));
            send(
                &mut app,
                S::ListOver(ListKey::Room(order[0].clone()), Zone::After),
            );
            let low = app.list_drag.target().cloned().expect("a target");
            send(
                &mut app,
                S::ListOver(ListKey::Room(order[1].clone()), Zone::Before),
            );
            let high = app.list_drag.target().cloned().expect("a target");
            assert_eq!(low, high, "the same target from both halves");
            assert_eq!(
                (low.item.clone(), low.zone),
                (ListKey::Room(order[1].clone()), Zone::Before)
            );
            // Only one row of the list carries the mark.
            let marked = |app: &App| {
                order
                    .iter()
                    .filter(|id| {
                        app.list_drag
                            .target()
                            .is_some_and(|t| t.item == ListKey::Room((*id).clone()))
                    })
                    .count()
            };
            assert_eq!(marked(&app), 1);
            // The first row keeps its top half, the last its bottom half.
            send(
                &mut app,
                S::ListOver(ListKey::Room(order[0].clone()), Zone::Before),
            );
            assert_eq!(
                app.list_drag.target().map(|t| (t.item.clone(), t.zone)),
                Some((ListKey::Room(order[0].clone()), Zone::Before))
            );
            send(
                &mut app,
                S::ListOver(ListKey::Room(order[2].clone()), Zone::After),
            );
            assert_eq!(
                app.list_drag.target().map(|t| (t.item.clone(), t.zone)),
                Some((ListKey::Room(order[2].clone()), Zone::After))
            );
            // Letting go in the gap puts the room between the two.
            send(
                &mut app,
                S::ListOver(ListKey::Room(order[0].clone()), Zone::After),
            );
            send(&mut app, S::PointerReleased);
            let now = record(&app).custom[0].rooms.clone();
            assert_eq!(now[0], order[0]);
            assert_eq!(now[1], rid("t1"));
            assert_eq!(now[2], order[1]);
        }

        #[test]
        fn a_press_and_release_with_no_movement_is_still_a_click() {
            let (mut app, store, mut rx) = arranged();
            drain(&mut rx);
            // A row: opens the room, even after a wobble under 4 px.
            send(&mut app, S::ListEnter(room("chan2")));
            send(&mut app, S::ListPress(room("chan2")));
            send(&mut app, S::PointerMoved(Point::new(10.0, 10.0)));
            send(&mut app, S::PointerMoved(Point::new(12.0, 11.0)));
            send(&mut app, S::ListRelease(room("chan2")));
            send(&mut app, S::PointerReleased);
            assert_eq!(app.selected, Some(rid("chan2")));
            assert!(
                drain(&mut rx)
                    .iter()
                    .any(|c| matches!(c, Command::SelectRoom(id) if *id == rid("chan2")))
            );
            // A header: collapses it, and again expands it.
            let saved = saves(&store);
            for expect in [true, false] {
                let key = head(SectionKey::Direct);
                send(&mut app, S::ListPress(key.clone()));
                send(&mut app, S::ListRelease(key));
                send(&mut app, S::PointerReleased);
                assert_eq!(record(&app).collapsed.contains(&SectionKey::Direct), expect);
            }
            assert_eq!(saves(&store), saved + 2);
        }

        #[test]
        fn a_drag_is_no_click() {
            let (mut app, store, mut rx) = arranged();
            drain(&mut rx);
            start(&mut app, &room("chan2"));
            send(&mut app, S::ListRelease(room("chan2")));
            send(&mut app, S::PointerReleased);
            assert_ne!(app.selected, Some(rid("chan2")), "nothing opened");
            assert!(drain(&mut rx).is_empty());
            let saved = saves(&store);
            let key = head(SectionKey::Direct);
            start(&mut app, &key);
            send(&mut app, S::ListRelease(key));
            send(&mut app, S::PointerReleased);
            assert!(!record(&app).collapsed.contains(&SectionKey::Direct));
            assert_eq!(saves(&store), saved);
        }

        #[test]
        fn in_the_flat_list_or_without_a_layout_nothing_is_draggable() {
            // A search makes the list flat.
            let (mut app, _store, _rx) = arranged();
            let _ = app.update(Message::SearchChanged("chan".to_owned()));
            assert!(!app.sections_shown());
            send(&mut app, S::ListPress(room("chan2")));
            send(&mut app, S::PointerMoved(Point::new(0.0, 0.0)));
            send(&mut app, S::PointerMoved(Point::new(50.0, 50.0)));
            assert!(!app.list_drag.is_dragging());
            // So does the People tab.
            let (mut app, _store, _rx) = arranged();
            let _ = app.update(Message::ChipClicked(RoomListTab::People));
            send(&mut app, S::ListPress(room("dm1")));
            send(&mut app, S::PointerMoved(Point::new(0.0, 0.0)));
            send(&mut app, S::PointerMoved(Point::new(50.0, 50.0)));
            assert!(!app.list_drag.is_dragging());
            // A layout that is protected cannot be arranged.
            let (mut app, _store, _rx) = drag_app();
            let _ = app.update(Message::SidebarLoaded(SidebarAnswer::new(
                UserId::from("@a:x"),
                SidebarLoad::Newer,
            )));
            app.sidebar.unload();
            send(&mut app, S::ListPress(room("chan2")));
            send(&mut app, S::PointerMoved(Point::new(0.0, 0.0)));
            send(&mut app, S::PointerMoved(Point::new(50.0, 50.0)));
            assert!(!app.list_drag.is_dragging());
        }

        #[test]
        fn esc_and_losing_the_focus_cancel_and_save_nothing() {
            for cancel in ["esc", "focus", "second press"] {
                let (mut app, store, _rx) = arranged();
                let saved = saves(&store);
                start(&mut app, &room("chan2"));
                send(
                    &mut app,
                    S::ListOver(head(SectionKey::Custom(0)), Zone::Into),
                );
                assert!(app.list_drag.target().is_some());
                match cancel {
                    "esc" => {
                        let _ = app.update(Message::Shortcut {
                            shortcut: Shortcut::Escape,
                            captured: false,
                        });
                    }
                    "focus" => {
                        let _ = app.update(Message::WindowFocus(false));
                    }
                    _ => send(&mut app, S::PointerPressed),
                }
                assert!(!app.list_drag.is_active(), "{cancel}");
                send(&mut app, S::PointerReleased);
                assert_eq!(saves(&store), saved, "{cancel}");
            }
        }

        #[test]
        fn a_release_with_no_target_or_off_the_target_saves_nothing() {
            let (mut app, store, _rx) = arranged();
            let saved = saves(&store);
            start(&mut app, &room("chan2"));
            send(&mut app, S::PointerReleased);
            start(&mut app, &room("chan2"));
            send(
                &mut app,
                S::ListOver(head(SectionKey::Custom(0)), Zone::Into),
            );
            send(&mut app, S::ListExit(head(SectionKey::Custom(0))));
            send(&mut app, S::PointerReleased);
            // Onto itself, and to where it already is.
            start(&mut app, &room("chan1"));
            send(&mut app, S::ListOver(room("chan1"), Zone::Before));
            assert!(app.list_drag.target().is_none());
            send(
                &mut app,
                S::ListOver(head(SectionKey::Custom(0)), Zone::Into),
            );
            assert!(app.list_drag.target().is_none(), "it is already there");
            send(&mut app, S::PointerReleased);
            assert_eq!(saves(&store), saved);
        }

        #[test]
        fn the_same_target_is_planned_once_until_the_pointer_leaves_or_things_change() {
            use crate::list_drop::plan_runs;
            let (mut app, _store, _rx) = arranged();
            start(&mut app, &room("chan2"));
            let runs = plan_runs();
            let over = |app: &mut App| {
                send(app, S::ListOver(head(SectionKey::Custom(0)), Zone::Into));
            };
            for _ in 0..5 {
                over(&mut app);
            }
            assert_eq!(plan_runs() - runs, 1, "five moves over it, one plan");
            send(&mut app, S::ListExit(head(SectionKey::Custom(0))));
            over(&mut app);
            assert_eq!(plan_runs() - runs, 2, "after leaving and coming back");
            // A change of the list forgets what was worked out.
            feed(
                &mut app,
                CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: rooms() }]),
            );
            over(&mut app);
            assert_eq!(plan_runs() - runs, 3, "after the rooms changed");
        }

        #[test]
        fn the_window_listener_runs_only_while_the_button_is_down_on_a_row_or_header() {
            use iced::advanced::subscription::into_recipes;
            let (mut app, _store, _rx) = arranged();
            let count = |app: &App| into_recipes(app.list_drag_subscription()).len();
            assert_eq!(count(&app), 0, "idle");
            send(&mut app, S::ListEnter(room("chan2")));
            assert_eq!(count(&app), 0, "hover alone");
            send(&mut app, S::ListPress(room("chan2")));
            assert_eq!(count(&app), 1, "pressed");
            send(&mut app, S::PointerReleased);
            assert_eq!(count(&app), 0, "released");
        }

        #[test]
        fn a_row_that_leaves_the_list_ends_its_drag_and_hover() {
            let (mut app, store, _rx) = arranged();
            let saved = saves(&store);
            send(&mut app, S::ListEnter(room("chan2")));
            start(&mut app, &room("chan2"));
            send(
                &mut app,
                S::ListOver(head(SectionKey::Custom(0)), Zone::Into),
            );
            let rest: Vec<RoomSummary> = rooms()
                .into_iter()
                .filter(|r| r.id != rid("chan2"))
                .collect();
            feed(
                &mut app,
                CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: rest }]),
            );
            assert!(!app.list_drag.is_active());
            assert!(app.list_hover.is_none());
            send(&mut app, S::PointerReleased);
            assert_eq!(saves(&store), saved);
        }

        #[test]
        fn a_card_that_opens_during_a_press_ends_it_and_a_list_press_ends_a_rail_press() {
            let (mut app, _store, _rx) = arranged();
            send(&mut app, S::ListPress(room("chan2")));
            menu(&mut app, M::OpenRoom(rid("chan2")));
            assert!(app.sidebar_menu.is_some());
            assert!(!app.list_drag.is_active());
            menu(&mut app, M::Close);
            send(
                &mut app,
                S::RailPressed(crate::spaces::RailKey::Space(rid("sp"))),
            );
            send(&mut app, S::ListPress(room("chan2")));
            assert!(app.rail_press.is_none());
            assert!(app.list_drag.is_active());
        }

        #[test]
        fn pointer_moves_over_an_unchanged_list_rebuild_nothing_but_a_real_change_settles() {
            let (mut app, _store, _rx) = arranged();
            send(&mut app, S::ListEnter(room("chan2")));
            start(&mut app, &room("chan2"));
            send(
                &mut app,
                S::ListOver(head(SectionKey::Custom(0)), Zone::Into),
            );
            let builds = build_runs();
            for n in 0..50 {
                send(&mut app, S::PointerMoved(Point::new(40.0 + n as f32, 10.0)));
                send(&mut app, S::ListEnter(room("chan1")));
                send(&mut app, S::ListExit(room("chan1")));
                send(&mut app, S::Exit(crate::spaces::RailKey::Home));
            }
            assert_eq!(
                build_runs(),
                builds,
                "no rebuild for pointer moves or hover"
            );
            assert!(app.list_drag.is_dragging());
            // A scroll asks for avatars (its own build) and is not settled.
            let _ = app.update(Message::RoomListScrolled {
                offset_y: 10.0,
                viewport_h: 500.0,
            });
            // A room that leaves still ends the drag.
            let rest: Vec<RoomSummary> = rooms()
                .into_iter()
                .filter(|r| r.id != rid("chan2"))
                .collect();
            feed(
                &mut app,
                CoreEvent::RoomListChanged(vec![ListDiff::Reset { values: rest }]),
            );
            assert!(build_runs() > builds, "a change of the list settles");
            assert!(!app.list_drag.is_active());
        }

        #[test]
        fn a_rail_press_ends_a_tracked_list_press() {
            let (mut app, _store, _rx) = arranged();
            send(&mut app, S::ListPress(room("chan2")));
            send(
                &mut app,
                S::RailPressed(crate::spaces::RailKey::Space(rid("sp"))),
            );
            assert!(!app.list_drag.is_active());
            assert!(app.rail_press.is_some());
        }

        #[test]
        fn a_click_works_again_after_a_cancelled_drag_and_after_the_window_press_cancel() {
            let (mut app, store, mut rx) = arranged();
            let click_room = |app: &mut App| {
                send(app, S::ListEnter(room("chan2")));
                send(app, S::ListPress(room("chan2")));
                send(app, S::ListRelease(room("chan2")));
                send(app, S::PointerReleased);
            };
            for cancel in ["esc", "focus", "second press"] {
                app.selected = None;
                drain(&mut rx);
                start(&mut app, &room("chan2"));
                match cancel {
                    "esc" => {
                        let _ = app.update(Message::Shortcut {
                            shortcut: Shortcut::Escape,
                            captured: false,
                        });
                    }
                    "focus" => {
                        let _ = app.update(Message::WindowFocus(false));
                    }
                    _ => send(&mut app, S::PointerPressed),
                }
                assert!(!app.list_drag.is_active(), "{cancel}");
                click_room(&mut app);
                assert_eq!(app.selected, Some(rid("chan2")), "{cancel}: the row opens");
            }
            // A press cancelled by the window press, then a header click.
            send(&mut app, S::ListPress(head(SectionKey::Direct)));
            send(&mut app, S::PointerPressed);
            let saved = saves(&store);
            send(&mut app, S::ListPress(head(SectionKey::Direct)));
            send(&mut app, S::ListRelease(head(SectionKey::Direct)));
            send(&mut app, S::PointerReleased);
            assert!(record(&app).collapsed.contains(&SectionKey::Direct));
            assert_eq!(saves(&store), saved + 1);
        }
    }

    // ---- people under voice rooms (Task sbc.12) ----

    fn cmember(n: &str, name: Option<&str>) -> CallMemberView {
        CallMemberView {
            user_id: UserId::from(format!("@{n}:x")),
            display_name: name.map(str::to_owned),
            avatar: None,
        }
    }

    fn calls_with(room_n: &str, members: Vec<CallMemberView>) -> CallModel {
        let mut calls = CallModel::default();
        calls.members.insert(rid(room_n), members);
        calls
    }

    fn built_with_calls(calls: &CallModel, layout: Option<&SpaceLayout>) -> Vec<String> {
        let sp = spaces();
        let rs = rooms();
        lines_of(&build_with(&sp[0], &sp, &rs, layout, Some(calls)))
    }

    #[test]
    fn people_are_drawn_under_their_room_in_order() {
        let calls = calls_with(
            "v1",
            vec![
                cmember("rhae", Some("Rhaenyra")),
                cmember("sept", Some("Septimus")),
                cmember("galep", None),
            ],
        );
        let lines = built_with_calls(&calls, None);
        let at = lines.iter().position(|l| l == "v1").expect("the room");
        assert_eq!(
            &lines[at..at + 5],
            [
                "v1",
                "  - Rhaenyra",
                "  - Septimus",
                "  - galep",
                "# Direct messages"
            ]
        );
        // Without a call model (or with none for the room) nothing is added.
        assert_eq!(built(None), built_with_calls(&CallModel::default(), None));
    }

    #[test]
    fn the_ninth_member_onward_becomes_and_n_more() {
        let many = |n: usize| -> Vec<CallMemberView> {
            (0..n).map(|i| cmember(&format!("u{i}"), None)).collect()
        };
        let count_people =
            |lines: &[String]| lines.iter().filter(|l| l.starts_with("  - u")).count();
        let lines = built_with_calls(&calls_with("v1", many(8)), None);
        assert_eq!(count_people(&lines), 8);
        assert!(!lines.iter().any(|l| l.contains(" more")), "eight fit");
        let lines = built_with_calls(&calls_with("v1", many(9)), None);
        assert_eq!(count_people(&lines), 8);
        assert!(lines.contains(&"  - and 1 more".to_owned()), "{lines:?}");
        let lines = built_with_calls(&calls_with("v1", many(25)), None);
        assert_eq!(count_people(&lines), 8);
        assert!(lines.contains(&"  - and 17 more".to_owned()), "{lines:?}");
        // The first eight are the first eight, in order.
        let at = lines.iter().position(|l| l == "  - u0").expect("first");
        assert_eq!(lines[at + 7], "  - u7");
        assert_eq!(lines[at + 8], "  - and 17 more");
    }

    #[test]
    fn only_a_voice_room_lists_its_people() {
        // `chan1` is not a voice room: a DM call's members are not listed.
        let calls = calls_with("chan1", vec![cmember("x", None)]);
        assert_eq!(built_with_calls(&calls, None), built(None));
    }

    #[test]
    fn two_people_with_one_name_are_told_apart_by_their_localparts() {
        let calls = calls_with(
            "v1",
            vec![
                cmember("sam1", Some("Sam")),
                cmember("sam2", Some("Sam")),
                cmember("alice", Some("Alice")),
                cmember("ghost", Some("\u{202E}")),
            ],
        );
        let lines = built_with_calls(&calls, None);
        let at = lines.iter().position(|l| l == "v1").expect("the room");
        assert_eq!(
            &lines[at + 1..at + 5],
            ["  - Sam (sam1)", "  - Sam (sam2)", "  - Alice", "  - ghost"],
            "{lines:?}"
        );
    }

    #[test]
    fn people_beyond_the_ninth_still_count_for_telling_names_apart() {
        // The ninth person is not drawn, but a name shared with a drawn one
        // is told apart all the same, so the list does not change as people
        // join or leave behind the "and N more" line.
        let mut members: Vec<_> = (0..MAX_PEOPLE_SHOWN)
            .map(|i| cmember(&format!("p{i}"), Some(&format!("Person {i}"))))
            .collect();
        members[0] = cmember("first", Some("Twin"));
        members.push(cmember("second", Some("Twin")));
        let calls = calls_with("v1", members);
        let lines = built_with_calls(&calls, None);
        assert!(lines.contains(&"  - Twin (first)".to_owned()), "{lines:?}");
    }

    #[test]
    fn a_collapsed_section_hides_the_people_with_its_rooms() {
        let calls = calls_with("v1", vec![cmember("x", Some("Xavier"))]);
        let layout = SpaceLayout {
            collapsed: vec![SectionKey::SubSpace(rid("voice"))],
            ..SpaceLayout::new(rid("sp"))
        };
        let lines = built_with_calls(&calls, Some(&layout));
        assert!(
            !lines.iter().any(|l| l.contains("Xavier") || l == "v1"),
            "{lines:?}"
        );
    }

    #[test]
    fn marks_are_drawn_only_for_the_call_this_device_is_in() {
        use crate::calls::CurrentCall;
        use twicecat_core::api::{CleaningStatus, DeviceId as Dev, Participant, ParticipantId};
        let person = |n: &str, speaking: bool, muted: bool| Participant {
            id: ParticipantId::from(format!("{n}-dev")),
            user_id: UserId::from(format!("@{n}:x")),
            device_id: Dev::from("DEV"),
            display_name: None,
            is_local: false,
            muted,
            speaking,
            volume_percent: 100,
            share_volume_percent: 100,
            cleaning: CleaningStatus::Off,
        };
        let mut calls = calls_with(
            "v1",
            vec![
                cmember("rhae", None),
                cmember("sept", None),
                cmember("quiet", None),
            ],
        );
        // The same people are in a second voice room's call.
        calls.members.insert(
            rid("t1"),
            vec![cmember("rhae", None), cmember("sept", None)],
        );
        // This device is in `t1`, not in `v1`.
        let mut ours = CurrentCall::new(rid("t1"), crate::calls::CallPhase::Joining);
        ours.participants = vec![person("rhae", true, false), person("sept", false, true)];
        calls.call = Some(ours);
        let lines = built_with_calls(&calls, None);
        assert!(
            !lines
                .iter()
                .any(|l| l.contains("*speaking") || l.contains("!muted")),
            "marks leaked to a room that is not our call: {lines:?}"
        );
        // Now `v1` is our call: the marks show there.
        let mut ours = CurrentCall::new(rid("v1"), crate::calls::CallPhase::Joining);
        ours.participants = vec![person("rhae", true, false), person("sept", false, true)];
        calls.call = Some(ours);
        let lines = built_with_calls(&calls, None);
        assert!(
            lines.contains(&"  - rhae *speaking".to_owned()),
            "{lines:?}"
        );
        assert!(lines.contains(&"  - sept !muted".to_owned()), "{lines:?}");
        assert!(lines.contains(&"  - quiet".to_owned()), "{lines:?}");
    }

    #[test]
    fn debug_of_the_people_lines_shows_no_user_name_or_room() {
        let secret = CallMemberView {
            user_id: UserId::from("@secretuser:x"),
            display_name: Some("Secret Name".to_owned()),
            avatar: None,
        };
        let room = RoomId::from("!secretroom:x");
        let lines = [
            Line::Participant(Person {
                room: &room,
                member: &secret,
                name: "Secret Name".to_owned(),
                speaking: true,
                muted: false,
            }),
            Line::More(More {
                room: &room,
                hidden: 3,
            }),
        ];
        let all = format!("{lines:?}");
        for hidden in ["secretuser", "Secret", "secretroom"] {
            assert!(!all.contains(hidden), "{hidden} is in {all}");
        }
        assert!(
            all.contains("speaking: true") && all.contains("hidden: 3"),
            "{all}"
        );
    }

    #[test]
    fn people_lines_have_their_own_heights_and_are_not_rooms() {
        let calls = calls_with("v1", vec![cmember("x", None)]);
        let sp = spaces();
        let rs = rooms();
        let lines = build_with(&sp[0], &sp, &rs, None, Some(&calls));
        let person = lines
            .iter()
            .find(|l| matches!(l, Line::Participant(_)))
            .expect("a person");
        assert_eq!(line_height(person, Density::Comfortable), PERSON_HEIGHT);
        assert_eq!(line_height(person, Density::Compact), COMPACT_PERSON_HEIGHT);
        // The room's spot does not count the person as a room.
        let spot = spot_of(&lines, &rid("v1")).expect("drawn");
        assert_eq!(spot.rooms, vec![rid("v1")]);
        assert_eq!(
            drawn_sections(&lines).len(),
            drawn_sections(&build(&sp[0], &sp, &rs, None)).len()
        );
    }

    fn active_call(app: &mut App, room_n: &str, members: Vec<CallMemberView>) {
        feed(
            app,
            CoreEvent::ActiveCall {
                room_id: rid(room_n),
                participants: u32::try_from(members.len()).expect("few"),
                members,
            },
        );
    }

    #[test]
    fn the_app_lists_people_from_active_call_and_removes_them_at_zero() {
        let (mut app, _rx) = space_app(None);
        active_call(&mut app, "v1", vec![cmember("bob", Some("Bob"))]);
        assert!(drawn_names(&app).contains(&"  - Bob".to_owned()));
        active_call(&mut app, "v1", Vec::new());
        assert!(!drawn_names(&app).iter().any(|l| l.starts_with("  - ")));
    }

    #[test]
    fn the_flat_list_shows_no_people() {
        let (mut app, _rx) = space_app(None);
        active_call(&mut app, "v1", vec![cmember("bob", Some("Bob"))]);
        let _ = app.update(Message::SearchChanged("v".to_owned()));
        assert!(app.section_lines().is_none());
        assert!(app.rooms_as_drawn().iter().all(|r| !r.name.contains("Bob")));
        // The flat list still has the call pill's count.
        assert_eq!(app.calls.count(&rid("v1")), 1);
    }

    #[test]
    fn the_avatars_of_the_people_are_asked_for_once() {
        use twicecat_core::api::AvatarId;
        let (mut app, mut rx) = space_app(None);
        app.list_viewport_h = 2000.0;
        let _ = drain(&mut rx);
        let mut bob = cmember("bob", Some("Bob"));
        bob.avatar = Some(AvatarId::from("mxc://x/bobpic"));
        active_call(&mut app, "v1", vec![bob.clone()]);
        let asked: Vec<Command> = drain(&mut rx);
        let fetches = |cmds: &[Command]| {
            cmds.iter()
                .filter(|c| matches!(c, Command::FetchAvatars(ids) if ids.contains(&AvatarId::from("mxc://x/bobpic"))))
                .count()
        };
        assert_eq!(fetches(&asked), 1, "{asked:?}");
        // The same set again, and a scroll: not asked again.
        active_call(&mut app, "v1", vec![bob]);
        let _ = app.update(Message::RoomListScrolled {
            offset_y: 0.0,
            viewport_h: 2000.0,
        });
        assert_eq!(fetches(&drain(&mut rx)), 0);
    }

    // ---- Discord categories: voice after text, a collapsed category keeps the open room and the call ----

    mod categories {
        use super::*;
        use crate::calls::{CallPhase, CurrentCall};
        use twicecat_core::api::RoomSortChoice;

        /// Space `s` with two text rooms (`b`, `d`) and two voice rooms
        /// (`a`, `c`), in the list's order d, c, b, a.
        fn fixture() -> (Vec<SpaceView>, Vec<RoomSummary>) {
            let voice = |n: &str| {
                let mut r = room(n);
                r.is_call_room = true;
                r
            };
            (
                vec![view("s", 0, &["a", "b", "c", "d"])],
                vec![room("d"), voice("c"), room("b"), voice("a")],
            )
        }

        fn under_sort(
            layout: Option<&SpaceLayout>,
            by: RoomSortChoice,
            unread: bool,
        ) -> Vec<String> {
            let (sp, rs) = fixture();
            let sorting = Sorting {
                by,
                unread_first: unread,
            };
            lines_of(&build_sorted(&sp[0], &sp, &rs, layout, None, sorting))
        }

        #[test]
        fn text_rooms_come_first_in_channels_under_every_sort() {
            assert_eq!(
                under_sort(None, RoomSortChoice::Activity, false),
                ["# Channels", "d", "b", "c", "a"]
            );
            assert_eq!(
                under_sort(None, RoomSortChoice::Name, false),
                ["# Channels", "b", "d", "a", "c"]
            );
            assert_eq!(
                under_sort(None, RoomSortChoice::Activity, true),
                ["# Channels", "d", "b", "c", "a"]
            );
            // An unread voice room still comes after the text rooms.
            let (sp, mut rs) = fixture();
            rs[3].unread_messages = 1;
            let sorting = Sorting {
                by: RoomSortChoice::Name,
                unread_first: true,
            };
            let out = lines_of(&build_sorted(&sp[0], &sp, &rs, None, None, sorting));
            assert_eq!(out, ["# Channels", "b", "d", "a", "c"]);
        }

        #[test]
        fn low_priority_puts_voice_after_text_under_two_sorts() {
            let (sp, mut rs) = fixture();
            for r in &mut rs {
                r.is_low_priority = true;
            }
            let run = |by| {
                let sorting = Sorting {
                    by,
                    unread_first: false,
                };
                lines_of(&build_sorted(&sp[0], &sp, &rs, None, None, sorting))
            };
            assert_eq!(
                run(RoomSortChoice::Activity),
                ["# Low priority", "d", "b", "c", "a"]
            );
            assert_eq!(
                run(RoomSortChoice::Name),
                ["# Low priority", "b", "d", "a", "c"]
            );
        }

        #[test]
        fn every_section_the_user_does_not_order_puts_voice_after_text() {
            let (sp, mut rs) = fixture();
            for r in &mut rs {
                r.is_favourite = true;
            }
            let out = lines_of(&build(&sp[0], &sp, &rs, None));
            assert_eq!(out, ["# Favourites", "d", "b", "c", "a"]);
        }

        #[test]
        fn a_custom_section_keeps_the_users_order_with_a_voice_room_first() {
            let mut layout = SpaceLayout::new(rid("s"));
            layout.custom = vec![CustomSection {
                id: 1,
                name: label("Mine"),
                rooms: vec![rid("a"), rid("b"), rid("c")],
            }];
            for by in [RoomSortChoice::Activity, RoomSortChoice::Name] {
                let out = under_sort(Some(&layout), by, true);
                let at = out.iter().position(|l| l == "# Mine").expect("drawn");
                assert_eq!(&out[at + 1..at + 4], ["a", "b", "c"], "{by:?}");
            }
        }

        #[test]
        fn starred_keeps_the_users_order_with_a_voice_room_first() {
            let mut layout = SpaceLayout::new(rid("s"));
            layout.starred = vec![rid("c"), rid("d")];
            let out = under_sort(Some(&layout), RoomSortChoice::Name, false);
            assert_eq!(&out[..3], ["# Starred", "c", "d"]);
        }

        fn collapsed_layout() -> SpaceLayout {
            let mut layout = SpaceLayout::new(rid("s"));
            layout.collapsed = vec![SectionKey::Channels];
            layout
        }

        fn open_lines(selected: Option<&RoomId>, calls: Option<&CallModel>) -> Vec<String> {
            let (sp, rs) = fixture();
            let layout = collapsed_layout();
            lines_of(&build_open(
                &sp[0],
                &sp,
                &rs,
                Some(&layout),
                calls,
                Sorting::default(),
                selected,
            ))
        }

        fn in_call(room_n: &str, members: Vec<CallMemberView>) -> CallModel {
            let mut calls = calls_with(room_n, members);
            calls.call = Some(CurrentCall::new(rid(room_n), CallPhase::Joining));
            calls
        }

        #[test]
        fn a_collapsed_section_with_the_selected_room_draws_its_header_and_that_room_only() {
            assert_eq!(open_lines(Some(&rid("b")), None), ["# Channels", "b"]);
            assert_eq!(open_lines(Some(&rid("a")), None), ["# Channels", "a"]);
        }

        #[test]
        fn a_collapsed_section_with_neither_room_draws_the_header_only() {
            assert_eq!(open_lines(None, None), ["# Channels"]);
            assert_eq!(open_lines(Some(&rid("elsewhere")), None), ["# Channels"]);
            // A call that is in another room keeps nothing here.
            let calls = in_call("elsewhere", vec![cmember("x", None)]);
            assert_eq!(open_lines(None, Some(&calls)), ["# Channels"]);
        }

        #[test]
        fn with_this_devices_call_in_it_a_collapsed_section_draws_the_voice_room_and_its_people() {
            let calls = in_call("c", vec![cmember("x", Some("Xavier")), cmember("y", None)]);
            assert_eq!(
                open_lines(None, Some(&calls)),
                ["# Channels", "c", "  - Xavier", "  - y"]
            );
            // Together with the open room: text before voice, as expanded.
            assert_eq!(
                open_lines(Some(&rid("b")), Some(&calls)),
                ["# Channels", "b", "c", "  - Xavier", "  - y"]
            );
            // The open room is the call room: one line, once.
            assert_eq!(
                open_lines(Some(&rid("c")), Some(&calls)),
                ["# Channels", "c", "  - Xavier", "  - y"]
            );
        }

        #[test]
        fn someone_elses_call_keeps_nothing_in_a_collapsed_section() {
            let calls = calls_with("c", vec![cmember("x", None)]);
            assert_eq!(open_lines(None, Some(&calls)), ["# Channels"]);
        }

        #[test]
        fn the_header_badge_counts_every_room_of_the_section_kept_or_not() {
            let (sp, mut rs) = fixture();
            rs[0].unread_notifications = 2; // d, hidden
            rs[2].unread_notifications = 3; // b, kept
            let layout = collapsed_layout();
            let open = rid("b");
            let lines = build_open(
                &sp[0],
                &sp,
                &rs,
                Some(&layout),
                None,
                Sorting::default(),
                Some(&open),
            );
            let Line::Header(header) = &lines[0] else {
                panic!("not a header");
            };
            assert_eq!(header.badge, Badge::Count(5));
            assert_eq!(lines_of(&lines), ["# Channels", "b"]);
        }

        #[test]
        fn an_unread_room_that_is_not_open_is_not_kept() {
            // The Q3 default: unread rooms stay hidden and the header counts them.
            let (sp, mut rs) = fixture();
            for r in &mut rs {
                r.unread_messages = 4;
                r.highlights = 1;
            }
            let layout = collapsed_layout();
            let lines = build_open(
                &sp[0],
                &sp,
                &rs,
                Some(&layout),
                None,
                Sorting::default(),
                None,
            );
            assert_eq!(lines_of(&lines), ["# Channels"]);
        }

        #[test]
        fn a_collapsed_custom_section_keeps_the_open_room_in_the_users_order() {
            let mut layout = SpaceLayout::new(rid("s"));
            layout.custom = vec![CustomSection {
                id: 1,
                name: label("Mine"),
                rooms: vec![rid("a"), rid("b"), rid("c")],
            }];
            layout.collapsed = vec![SectionKey::Custom(1)];
            let (sp, rs) = fixture();
            let calls = in_call("a", Vec::new());
            let open = rid("c");
            let lines = build_open(
                &sp[0],
                &sp,
                &rs,
                Some(&layout),
                Some(&calls),
                Sorting::default(),
                Some(&open),
            );
            assert_eq!(
                lines_of(&lines)[..],
                ["# Channels", "d", "# Mine", "a", "c"],
                "no hint, a then c as the user ordered them"
            );
        }

        #[test]
        fn kept_rooms_are_ordinary_lines_for_heights_and_spots() {
            let (sp, rs) = fixture();
            let layout = collapsed_layout();
            let open = rid("b");
            let lines = build_open(
                &sp[0],
                &sp,
                &rs,
                Some(&layout),
                None,
                Sorting::default(),
                Some(&open),
            );
            let offsets = line_offsets(&lines, Density::Comfortable);
            assert_eq!(offsets.len(), lines.len() + 1);
            assert_eq!(
                offsets.last().copied(),
                Some(HEADER_HEIGHT + row_height(Density::Comfortable))
            );
            let spot = spot_of(&lines, &rid("b")).expect("drawn");
            assert_eq!(
                (spot.section, spot.rooms),
                (SectionKey::Channels, vec![rid("b")])
            );
            assert!(spot_of(&lines, &rid("d")).is_none(), "hidden");
        }
    }

    mod stepping_a_collapsed_category {
        use super::*;
        use crate::keys::Shortcut;

        #[test]
        fn alt_down_skips_the_hidden_rooms_and_reaches_the_kept_one() {
            use crate::calls::{CallPhase, CurrentCall};
            let (mut app, _rx) = space_app(None);
            // Direct messages is collapsed with this device's call in dm1's
            // section: the call room is the kept one, drawn after the hidden
            // rooms of Channels and Tech, and the hidden rooms of Voice sit
            // between the call room and the room drawn before it.
            collapse(&mut app, SectionKey::SubSpace(rid("tech")));
            collapse(&mut app, SectionKey::SubSpace(rid("voice")));
            app.calls.call = Some(CurrentCall::new(rid("v1"), CallPhase::Joining));
            let _ = app.update(Message::SelectRoom(rid("starred1")));
            assert_eq!(
                drawn_names(&app),
                [
                    "# Channels",
                    "chan1",
                    "chan2",
                    "starred1",
                    "# tech",
                    "# voice",
                    "v1",
                    "# Direct messages",
                    "dm1"
                ]
            );
            // Tech's t1, t2, rustroom and techdm are hidden between the rooms
            // of Channels and Voice: Alt+Down from starred1 lands on v1.
            press(&mut app, Shortcut::NextRoom);
            assert_eq!(app.selected, Some(rid("v1")), "t1 .. techdm are skipped");
            // v1 is open now and still kept; the next drawn room is dm1.
            press(&mut app, Shortcut::NextRoom);
            assert_eq!(app.selected, Some(rid("dm1")));
        }

        #[test]
        fn alt_up_skips_the_hidden_rooms_and_reaches_the_kept_call_room() {
            use crate::calls::{CallPhase, CurrentCall};
            let (mut app, _rx) = space_app(None);
            collapse(&mut app, SectionKey::Channels);
            app.calls.call = Some(CurrentCall::new(rid("chan2"), CallPhase::Joining));
            let _ = app.update(Message::SelectRoom(rid("t1")));
            press(&mut app, Shortcut::PreviousRoom);
            assert_eq!(
                app.selected,
                Some(rid("chan2")),
                "chan1 and starred1 are hidden, the call room is not"
            );
            press(&mut app, Shortcut::NextRoom);
            assert_eq!(app.selected, Some(rid("t1")));
        }
    }
}
