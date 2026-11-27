//! The room list pane: filter chips, search and the windowed list of rows (plan Q12).
//!
//! Only the rows near the viewport are built, between two spacers that keep
//! the scrollable's content `len * row height` tall (the row height is the
//! density's, or the rail's fixed 56 px). Names and previews come from other
//! people, so they pass through [`crate::text_safety`] first (see
//! [`row_name`] and [`preview_line`]).
//!
//! The pane is either expanded or collapsed to a rail
//! ([`RoomListChoice`]). The column is always `[actions, chips, search, list,
//! call bar]` in both, so the list keeps its place when Ctrl+B switches.

use chrono::{Local, TimeZone};
use iced::widget::text::Wrapping;
use iced::widget::tooltip::{Position, Tooltip};
use iced::widget::{
    Column, button, checkbox, column, container, image, mouse_area, pick_list, row, scrollable,
    space, stack, text, text_input,
};
use iced::{Background, Color, Element, Fill, Length, border, mouse};
use twicecat_core::api::{
    RoomId, RoomListChoice, RoomListTab, RoomNotificationModeView, RoomSummary, SectionKey,
};

use crate::avatars::{AvatarState, Avatars, NEUTRAL_INITIALS, initials, placeholder_color};
use crate::calls::CallPill;
use crate::dates::{day_label, time_label};
use crate::drag::Zone;
use crate::list_drop::{self, ListKey};
use crate::notifications::muted_mark;
use crate::room_list::{
    Badge, Emphasis, OVERSCAN_ROWS, RAIL_ROW_HEIGHT, SortOption, badge_for, channel_emphasis,
    empty_label, preview_line, row_height, row_name, spacer_heights, visible_range, voice_badge,
};
use crate::rooms::RoomsMessage;
use crate::screens::{call, main_view};
use crate::sections;
use crate::sections::{
    EMPTY_HINT, Header, Line, More, Person, line_height, line_offsets, line_spacers, visible_lines,
};
use crate::settings::SettingsMessage;
use crate::sidebar::SidebarState;
use crate::sidebar_menu::SidebarMenuMessage;
use crate::spaces::{GhostItem, SpacesMessage};
use crate::state::{App, Message};
use crate::theme;
use crate::ui::fonts;
use crate::ui::icons::{self, Icon};
use crate::ui::kit;
use crate::ui::tokens::{self, Density};

/// Width of the collapsed room list, in logical pixels.
pub const RAIL_WIDTH: f32 = 72.0;

/// The id of the list's scrollable, so a change of row height can move it.
pub const LIST_SCROLL_ID: &str = "room-list";

/// Room kept free at the right of the list for its scrollbar, so the bar
/// never covers a count or the end of a preview (the bar is 10 px wide).
pub const SCROLLBAR_GUTTER: f32 = 12.0;

/// Width of the accent bar at the left of the selected row.
const ACCENT_BAR: f32 = 3.0;

/// The width of the pane: 300 px expanded, 72 px as the rail.
pub const fn pane_width(list: RoomListChoice) -> f32 {
    match list {
        RoomListChoice::Expanded => main_view::LIST_PANE_WIDTH,
        RoomListChoice::Rail => RAIL_WIDTH,
    }
}

/// Whether a row has a second line with the preview: Compact rows are one
/// line.
pub const fn shows_preview(density: Density) -> bool {
    matches!(density, Density::Comfortable)
}

/// **New room**, **Direct message** and **Join**, above the chips.
fn room_actions<'a>() -> Element<'a, Message> {
    let action = |label: &'static str, message: RoomsMessage| {
        button(text(label).size(tokens::CAPTION).wrapping(Wrapping::None))
            .on_press(Message::Rooms(message))
            .padding([tokens::XS, tokens::S])
            .style(kit::secondary_style)
    };
    row![
        action("New room", RoomsMessage::OpenCreate),
        action("Direct message", RoomsMessage::OpenDm),
        action("Join", RoomsMessage::OpenJoin),
    ]
    .spacing(tokens::XS)
    .into()
}

/// The same three actions as icons in a column, for the rail. Each has a
/// tooltip with its words.
fn rail_actions<'a>() -> Element<'a, Message> {
    let action = |icon: Icon, label: &'static str, message: RoomsMessage| {
        kit::tip(
            button(icons::icon(icon))
                .on_press(Message::Rooms(message))
                .padding(tokens::S)
                .style(kit::icon_style),
            label,
        )
    };
    container(
        column![
            action(Icon::Plus, "New room", RoomsMessage::OpenCreate),
            action(Icon::Members, "Direct message", RoomsMessage::OpenDm),
            action(Icon::ChevronRight, "Join a room", RoomsMessage::OpenJoin),
        ]
        .spacing(tokens::XS)
        .align_x(iced::Center),
    )
    .center_x(Fill)
    .into()
}

/// The filter chips, left to right (plan Q11).
pub const TABS: [RoomListTab; 9] = [
    RoomListTab::All,
    RoomListTab::Unreads,
    RoomListTab::Mentions,
    RoomListTab::People,
    RoomListTab::Rooms,
    RoomListTab::Favourites,
    RoomListTab::LowPriority,
    RoomListTab::Invites,
    RoomListTab::Spaces,
];

/// Side of the round avatar left of each row.
pub const AVATAR_SLOT: f32 = 40.0;
/// Side of the round avatar of a Compact row.
pub const COMPACT_AVATAR_SLOT: f32 = 28.0;

/// The label of a tab.
pub fn tab_label(tab: RoomListTab) -> &'static str {
    match tab {
        RoomListTab::All => "All",
        RoomListTab::People => "DMs",
        RoomListTab::Rooms => "Rooms",
        RoomListTab::Spaces => "Spaces",
        RoomListTab::Unreads => "Unreads",
        RoomListTab::Mentions => "Mentions",
        RoomListTab::Favourites => "Favourites",
        RoomListTab::LowPriority => "Low priority",
        RoomListTab::Invites => "Invites",
    }
}

/// A badge count as text: `99+` above 99.
///
/// ```
/// use twicecat_app::screens::room_list_pane::count_label;
/// assert_eq!(count_label(7), "7");
/// assert_eq!(count_label(100), "99+");
/// ```
pub fn count_label(count: u64) -> String {
    if count > 99 {
        "99+".to_owned()
    } else {
        count.to_string()
    }
}

/// The mark of an encrypted room, in the list row and the room header.
pub const LOCK_MARK: &str = Icon::Lock.glyph();
/// The word at the right of the Starred header: the stars are the user's own.
pub const PERSONAL_LABEL: &str = "personal";
/// The tag of a voice room (MSC3417 call room).
pub const VOICE_TAG: &str = "Voice";

/// The lock of `room`, when it is known to be encrypted. The core reports
/// `is_encrypted` as false while the SDK does not know the room's encryption
/// yet, so a missing lock does not say "not encrypted": only the lock is
/// ever drawn, never a mark for the opposite.
pub fn encrypted_mark(room: &RoomSummary) -> Option<&'static str> {
    room.is_encrypted.then_some(LOCK_MARK)
}

/// The time shown on a row: the clock for a message of today, else the day
/// ("Yesterday", a weekday, `2 October`).
pub fn row_time<Tz: TimeZone>(timestamp_ms: u64, now_ms: u64, tz: &Tz) -> String {
    let day = day_label(timestamp_ms, now_ms, tz);
    if day == "Today" {
        time_label(timestamp_ms, tz)
    } else {
        day
    }
}

/// What the rail's tooltip says about a room: its name, then the words for
/// the dots on its avatar (unread, mentions, call) and the muted mark.
pub fn rail_tip(
    name: &str,
    badge: Badge,
    call: Option<CallPill>,
    muted: Option<&str>,
    marks: &[RailMark],
) -> String {
    let mut parts = vec![name.to_owned()];
    match badge {
        Badge::None => {}
        Badge::Highlight(1) => parts.push("1 mention".to_owned()),
        Badge::Highlight(count) => parts.push(format!("{} mentions", count_label(count))),
        Badge::Count(count) => parts.push(format!("{} unread", count_label(count))),
        Badge::Dot => parts.push("unread".to_owned()),
    }
    if let Some(call) = call {
        parts.push(if call.ringing { "ringing" } else { "in a call" }.to_owned());
    }
    parts.extend(marks.iter().map(|mark| mark.words().to_owned()));
    parts.extend(muted.map(str::to_owned));
    parts.join(", ")
}

/// A mark a rail row draws on its avatar besides the dots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailMark {
    /// An invitation to join.
    Invite,
    /// A voice room (MSC3417 call room).
    Voice,
    /// An encrypted room.
    Encrypted,
}

impl RailMark {
    /// The words of the mark, for the row's tooltip.
    pub const fn words(self) -> &'static str {
        match self {
            Self::Invite => "invite",
            Self::Voice => "voice room",
            Self::Encrypted => "encrypted",
        }
    }

    /// The icon drawn on the avatar.
    pub const fn icon(self) -> Icon {
        match self {
            Self::Invite => Icon::Plus,
            Self::Voice => Icon::Call,
            Self::Encrypted => Icon::Lock,
        }
    }
}

/// The marks of a rail row, Invite first.
pub fn rail_marks(room: &RoomSummary) -> Vec<RailMark> {
    let mut marks = Vec::new();
    if room.is_invite {
        marks.push(RailMark::Invite);
    }
    if room.is_call_room {
        marks.push(RailMark::Voice);
    }
    if encrypted_mark(room).is_some() {
        marks.push(RailMark::Encrypted);
    }
    marks
}

/// The words of the call pill: the number of devices in the call, or what
/// the call is when nobody is counted.
fn call_label(call: CallPill) -> String {
    if call.count > 0 {
        call.count.to_string()
    } else if call.ringing {
        "Ringing".to_owned()
    } else {
        "Call".to_owned()
    }
}

/// What an empty list adds to its title: one line per chip. With a search
/// (`label` is "No match") it asks for another one. The Mentions hint does not
/// say that rooms marked unread are left out: the SDK lists them there too.
pub fn empty_hint(tab: RoomListTab, label: &str) -> &'static str {
    if label == "No match" {
        return "Try another search";
    }
    match tab {
        RoomListTab::All | RoomListTab::Rooms => "Rooms you join will appear here",
        RoomListTab::People => "Direct messages will appear here",
        RoomListTab::Spaces => "Spaces you join will appear here",
        RoomListTab::Unreads => "Rooms with unread messages will appear here",
        RoomListTab::Mentions => "Rooms where you are mentioned will appear here",
        RoomListTab::Favourites => "Rooms you favourite will appear here",
        RoomListTab::LowPriority => "Rooms you mark as low priority will appear here",
        RoomListTab::Invites => "Invitations will appear here",
    }
}

/// The filter chips as one wrapping row of pills, a single choice. The chosen
/// chip is filled and outlined; clicking it goes back to All. A chip never
/// wraps its own text, so a narrow pane puts chips on more lines instead.
pub fn chips<'a>(active: RoomListTab) -> Element<'a, Message> {
    row(TABS.into_iter().map(|tab| {
        button(
            text(tab_label(tab))
                .size(tokens::CAPTION)
                .wrapping(Wrapping::None),
        )
        .on_press(Message::ChipClicked(tab))
        .padding([tokens::XS, tokens::S])
        .style(kit::filter_chip_style(tab == active))
        .into()
    }))
    .spacing(tokens::XS)
    .wrap()
    .vertical_spacing(tokens::XS)
    .into()
}

/// The words under the Sort card's choices: which rooms the sort leaves
/// alone.
pub const SORT_NOTE: &str = "Starred rooms and the sections you made keep your own order.";

/// The Sort card (plan L2): the order of the rooms, and "Unreads first". The
/// list is a `pick_list` inside [`kit::menu`], so Ctrl + wheel (the zoom
/// gesture) cannot change the sort. Every choice is kept at once; the card
/// stays open for the next one.
pub fn sort_card(app: &App, window_height: f32) -> Element<'_, Message> {
    let close = || Message::SidebarMenu(SidebarMenuMessage::Close);
    let body = column![
        kit::field(
            "Order",
            kit::menu(
                pick_list(
                    SortOption::ALL,
                    Some(SortOption(app.ui_settings.room_sort)),
                    |option: SortOption| Message::SidebarMenu(SidebarMenuMessage::SetSort(
                        option.0
                    )),
                )
                .style(kit::menu_style)
                .width(Fill),
            ),
        ),
        checkbox(app.ui_settings.unread_first)
            .style(kit::checkbox_style)
            .label("Unreads first")
            .on_toggle(|on| Message::SidebarMenu(SidebarMenuMessage::UnreadFirst(on))),
        kit::words(SORT_NOTE)
            .size(tokens::CAPTION)
            .style(kit::muted),
    ]
    .spacing(tokens::M);
    kit::dialog(
        kit::dialog_header("Sort rooms", close()),
        body.into(),
        Some(kit::dialog_footer(vec![
            kit::secondary_button("Close", Some(close())).into(),
        ])),
        crate::screens::sidebar_menu::CARD_WIDTH,
        window_height,
    )
}

/// The search field with a search icon and, once there is text, a button
/// that clears it. The clear slot is a space while empty, so the field keeps
/// its place and its focus.
fn search_field(app: &App) -> Element<'_, Message> {
    let clear: Element<'_, Message> = if app.search_input.is_empty() {
        space().width(0).into()
    } else {
        kit::tip(
            button(icons::icon(Icon::Close))
                .on_press(Message::SearchChanged(String::new()))
                .padding(tokens::S)
                .style(kit::icon_style),
            "Clear search",
        )
    };
    row![
        icons::icon(Icon::Search),
        text_input("Search rooms", &app.search_input)
            .style(kit::text_input_style)
            .on_input(Message::SearchChanged)
            .padding(tokens::S),
        clear,
        sort_button(),
    ]
    .spacing(tokens::S)
    .align_y(iced::Center)
    .into()
}

/// **Sort**, which opens the Sort card.
fn sort_button<'a>() -> Element<'a, Message> {
    button(text("Sort").size(tokens::CAPTION).wrapping(Wrapping::None))
        .on_press(Message::SidebarMenu(SidebarMenuMessage::OpenSort))
        .padding([tokens::XS, tokens::S])
        .style(kit::secondary_style)
        .into()
}

/// The rail's search: a button that expands the list, where the field is.
fn rail_search<'a>() -> Element<'a, Message> {
    container(kit::tip(
        button(icons::icon(Icon::Search))
            .on_press(Message::Settings(SettingsMessage::RoomList(
                RoomListChoice::Expanded,
            )))
            .padding(tokens::S)
            .style(kit::icon_style),
        "Search rooms (expands the list)",
    ))
    .center_x(Fill)
    .into()
}

/// What an empty list says.
fn empty_view(app: &App, rail: bool) -> Element<'_, Message> {
    let label = empty_label(app.query.tab, &app.query.search);
    if rail {
        // 56 px wide: the title in small type, no hint.
        return container(
            text(label)
                .size(tokens::CAPTION)
                .style(kit::muted)
                .align_x(iced::Center),
        )
        .center_x(Fill)
        .padding([tokens::L, 0.0])
        .into();
    }
    kit::empty_state(label, empty_hint(app.query.tab, label))
}

/// The pane: chips, search, then the list or an empty-state line.
pub fn view(app: &App) -> Element<'_, Message> {
    let rail = app.pane_layout().list == RoomListChoice::Rail;
    let density = app.density();
    let list = if app.rooms.is_empty() {
        empty_view(app, rail)
    } else {
        windowed_list(app)
    };
    let chips_slot: Element<'_, Message>;
    let (actions, search, dock) = if rail {
        chips_slot = space().height(0).into();
        (rail_actions(), rail_search(), call::rail_dock(app))
    } else {
        chips_slot = chips(app.query.tab);
        (room_actions(), search_field(app), call::dock(app))
    };
    // The call bar is last, so the list keeps its place when a call starts.
    // `from_vec` keeps the empty slots (`push` drops a zero-sized child), so
    // the five slots stay put between the expanded list and the rail.
    let slots = Column::from_vec(vec![actions, chips_slot, search, list, dock])
        .spacing(density.gap())
        .width(Fill)
        .height(Fill);
    container(slots)
        .width(Length::Fixed(pane_width(app.pane_layout().list)))
        .height(Fill)
        .padding(if rail { tokens::XS } else { tokens::S })
        .into()
}

/// What a row needs besides its room.
struct RowEnv<'a> {
    avatars: &'a Avatars,
    density: Density,
    /// The clock, for the time of the last message.
    now_ms: u64,
}

/// The scrollable with only the rows near the viewport built.
fn windowed_list(app: &App) -> Element<'_, Message> {
    if let Some(lines) = app.section_lines() {
        return sectioned_list(app, &lines);
    }
    let len = app.rooms.len();
    let rail = app.pane_layout().list == RoomListChoice::Rail;
    let row_h = app.room_row_height();
    let range = visible_range(
        app.list_offset_y,
        app.list_viewport_h,
        row_h,
        len,
        OVERSCAN_ROWS,
    );
    let (top, bottom) = spacer_heights(&range, len, row_h);
    let env = RowEnv {
        avatars: &app.avatars,
        density: app.density(),
        now_ms: u64::try_from(Local::now().timestamp_millis()).unwrap_or(0),
    };
    // The rows as shown: in the sort's order.
    let order = app.flat_order();
    let rows = range.map(|at| &app.rooms[order.as_ref().map_or(at, |order| order[at])]);
    let rows = rows.map(|room| {
        let selected = app.selected.as_ref() == Some(&room.id);
        let mode = app.effective_room_mode(&room.id);
        let call = app.calls.pill(&room.id);
        if rail {
            rail_row(room, env.avatars, selected, mode, call)
        } else {
            flat_room_row(room_row(room, &env, selected, mode, call), room)
        }
    });
    scroller(top, rows, bottom, None)
}

/// A row of the flat list with its options: a right-click opens the room's
/// card (the tags). The area asks for nothing else, so the row's button keeps
/// the left press and the scrollable keeps the wheel: iced's `mouse_area`
/// updates its content first and only looks at the right button, and a button
/// takes only the left one.
fn flat_room_row<'a>(row_view: Element<'a, Message>, room: &RoomSummary) -> Element<'a, Message> {
    mouse_area(row_view)
        .on_right_press(Message::SidebarMenu(SidebarMenuMessage::OpenFlatRoom(
            room.id.clone(),
        )))
        .into()
}

/// The scrollable of both lists: `rows` between two spacers that keep its
/// content as tall as the whole list, with room for the scrollbar.
fn scroller<'a>(
    top: f32,
    rows: impl IntoIterator<Item = Element<'a, Message>>,
    bottom: f32,
    tail: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let content = column![space().height(top)]
        .extend(rows)
        .push(space().height(bottom))
        .extend(tail)
        .padding(iced::Padding {
            right: SCROLLBAR_GUTTER,
            ..iced::Padding::ZERO
        });
    scrollable(content)
        .id(LIST_SCROLL_ID)
        .on_scroll(|viewport| Message::RoomListScrolled {
            offset_y: viewport.absolute_offset().y,
            viewport_h: viewport.bounds().height,
        })
        .height(Fill)
        .into()
}

/// How tall the free space under the last line of the sectioned list is: the
/// rest of the viewport when the lines are shorter than it, and at least one
/// row, so there is always some to right-click.
pub fn filler_height(content: f32, viewport: f32, row: f32) -> f32 {
    (viewport - content).max(row)
}

/// The free space under the sectioned list: a right-click on it asks for the
/// card with "New section". It is below the spacer, so it never stands for a
/// line; it takes no left press and is no drop target, and a wheel turn is not
/// taken by it. A row, a header and their buttons are inside their own areas,
/// which take the right-click first (iced's `mouse_area` lets its content
/// capture before it acts).
fn list_filler<'a>(content: f32, viewport: f32, row: f32) -> Element<'a, Message> {
    mouse_area(
        space()
            .width(Fill)
            .height(filler_height(content, viewport, row)),
    )
    .on_right_press(Message::SidebarMenu(SidebarMenuMessage::OpenList))
    .into()
}

/// The sectioned list (a space is selected): headers, rooms and hints, each
/// at its own height, with only the lines near the viewport built. A channel
/// is drawn by [`channel_row`]; a DM keeps today's row.
fn sectioned_list<'a>(app: &'a App, lines: &[Line<'a>]) -> Element<'a, Message> {
    let density = app.density();
    let offsets = line_offsets(lines, density);
    let range = visible_lines(
        &offsets,
        app.list_offset_y,
        app.list_viewport_h,
        OVERSCAN_ROWS,
    );
    let (top, bottom) = line_spacers(&offsets, &range);
    let env = RowEnv {
        avatars: &app.avatars,
        density,
        now_ms: u64::try_from(Local::now().timestamp_millis()).unwrap_or(0),
    };
    let rows = lines[range].iter().map(|line| match line {
        Line::Header(header) => section_header(app, header, density),
        Line::Room(room) => {
            let selected = app.selected.as_ref() == Some(&room.id);
            let mode = app.effective_room_mode(&room.id);
            let call = app.calls.pill(&room.id);
            let cue = list_cue(app, &ListKey::Room(room.id.clone()));
            let pointer = Pointer::Area {
                hovered: cue.hovered,
                dragged: cue.dragged,
            };
            // A DM keeps its avatar and today's row; any other room is a
            // channel of its category (plan C2).
            let row_view = if room.is_dm {
                room_row_in(room, &env, selected, mode, call, pointer)
            } else {
                channel_row(room, &env, selected, mode, call, cue)
            };
            sectioned_room_row(row_view, room, selected, density, cue)
        }
        Line::Participant(person) => person_line(app, person, density),
        Line::More(more) => more_line(more, density),
        Line::EmptyHint(key) => {
            let target = ListKey::Header(key.clone());
            let cue = list_cue(app, &target);
            let height = line_height(line, density);
            let hint = container(text(EMPTY_HINT).size(tokens::CAPTION).style(kit::muted))
                .width(Fill)
                .padding([0.0, tokens::S])
                .center_y(Length::Fixed(height))
                .style(move |t: &iced::Theme| mark_style(t, cue.mark, false));
            // A drop target only: a press on the hint does nothing, and the
            // section's own header is what clicks and drags.
            let mut area = mouse_area(hint)
                .on_enter(Message::Spaces(SpacesMessage::ListEnter(target.clone())))
                .on_exit(Message::Spaces(SpacesMessage::ListExit(target.clone())));
            if cue.dragging {
                let at = target;
                area = area.interaction(cue.cursor).on_move(move |point| {
                    Message::Spaces(SpacesMessage::ListOver(
                        at.clone(),
                        list_drop::halves(point.y, height),
                    ))
                });
            }
            area.into()
        }
    });
    let filler = list_filler(
        offsets.last().copied().unwrap_or(0.0),
        app.list_viewport_h,
        row_height(density),
    );
    scroller(top, rows, bottom, Some(filler))
}

/// Thickness of the line that shows where a drop would put the dragged item.
const DROP_LINE: f32 = 3.0;

/// Width of the ring around a header that takes a drop, or refuses it.
const MARK_RING: f32 = 2.0;

/// What dragging shows on one row or header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    /// Nothing.
    None,
    /// A line above: a drop would put the item before this one.
    Before,
    /// A line below.
    After,
    /// A ring: a drop would put the item into this section.
    Into,
    /// A red ring and the words "Not allowed": the item may not go here.
    Refused,
}

/// What dragging shows on and does to one row or header.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ListCue {
    /// Where a drop here would land, while a drag runs.
    mark: Mark,
    /// The cursor: `Grab` for what can be dragged, `Grabbing` during a drag,
    /// `NotAllowed` over a refused place.
    cursor: mouse::Interaction,
    /// A drag runs: the area reports the pointer's place in it.
    dragging: bool,
    /// The pointer is over it, and no drag runs.
    hovered: bool,
    /// It is the one being dragged.
    dragged: bool,
    /// The button is down on it and no drag has started: the area reports
    /// where the pointer is in it once, for the ghost's grab offset.
    pressed: bool,
}

/// The cue of the row or header `key`.
fn list_cue(app: &App, key: &ListKey) -> ListCue {
    let dragging = app.list_drag.is_dragging();
    let may_drag = matches!(
        app.sidebar.state(),
        SidebarState::Loaded | SidebarState::Full
    );
    let refused = app.list_drag.refused().is_some_and(|t| t.item == *key);
    let mark = if refused {
        Mark::Refused
    } else {
        match app.list_drag.target().filter(|t| t.item == *key) {
            Some(t) if t.zone == Zone::Before => Mark::Before,
            Some(t) if t.zone == Zone::After => Mark::After,
            Some(_) => Mark::Into,
            None => Mark::None,
        }
    };
    let cursor = if refused {
        mouse::Interaction::NotAllowed
    } else if dragging {
        mouse::Interaction::Grabbing
    } else if may_drag {
        mouse::Interaction::Grab
    } else {
        mouse::Interaction::Pointer
    };
    ListCue {
        mark,
        cursor,
        dragging,
        hovered: !dragging && app.list_hover.as_ref() == Some(key),
        dragged: app.list_drag.dragged() == Some(key),
        pressed: app.list_drag.pressed() == Some(key),
    }
}

/// The look of a row or header under the pointer or a drag. The colours are
/// the theme's: the selection, the hover and the focus ring of a target, the
/// danger colour of a refused place.
fn mark_style(t: &iced::Theme, mark: Mark, hovered: bool) -> container::Style {
    let sem = theme::semantic(t);
    match mark {
        Mark::Into => container::Style {
            background: Some(Background::Color(sem.selection)),
            border: border::rounded(tokens::SMALL)
                .color(sem.focus)
                .width(MARK_RING),
            ..container::Style::default()
        },
        Mark::Refused => container::Style {
            background: Some(Background::Color(sem.danger_soft)),
            border: border::rounded(tokens::SMALL)
                .color(t.palette().danger)
                .width(MARK_RING),
            ..container::Style::default()
        },
        Mark::None | Mark::Before | Mark::After => container::Style {
            background: hovered.then_some(Background::Color(sem.message_hover)),
            border: border::rounded(tokens::SMALL),
            ..container::Style::default()
        },
    }
}

/// What `with_mark` lays over a line for a mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    /// Nothing.
    Nothing,
    /// The line over the top edge.
    LineAbove,
    /// The line over the bottom edge.
    LineBelow,
    /// The ring of a target around the whole line.
    Ring,
    /// The red ring and the words of a refusal around the whole line.
    RefusedRing,
}

/// The overlay of `mark`.
fn overlay_of(mark: Mark) -> Overlay {
    match mark {
        Mark::None => Overlay::Nothing,
        Mark::Before => Overlay::LineAbove,
        Mark::After => Overlay::LineBelow,
        Mark::Into => Overlay::Ring,
        Mark::Refused => Overlay::RefusedRing,
    }
}

/// The mark a header hands to `list_area`: the line of a before or after
/// (nothing else draws it), and nothing for a ring or a refusal, which the
/// header draws itself (`mark_style` and its "Not allowed" words), so there
/// is no second ring over it.
fn header_mark(mark: Mark) -> Mark {
    match mark {
        Mark::Before | Mark::After => mark,
        Mark::None | Mark::Into | Mark::Refused => Mark::None,
    }
}

/// `content`, `height` tall, with the line of a before or after mark over its
/// top or bottom edge. Always a stack, so the widget tree keeps its shape
/// when a mark comes and goes.
fn with_mark<'a>(content: Element<'a, Message>, mark: Mark, height: f32) -> Element<'a, Message> {
    let line = |top: bool| -> Element<'a, Message> {
        let bar = container(space().width(Fill).height(DROP_LINE)).style(|t: &iced::Theme| {
            container::Style {
                background: Some(Background::Color(theme::semantic(t).focus)),
                border: border::rounded(DROP_LINE / 2.0),
                ..container::Style::default()
            }
        });
        // Clear of the accent bar at the left of a row.
        let layer = container(bar)
            .width(Fill)
            .height(Length::Fixed(height))
            .padding(iced::Padding {
                left: ACCENT_BAR,
                ..iced::Padding::ZERO
            });
        if top {
            layer.align_top(Length::Fixed(height)).into()
        } else {
            layer.align_bottom(Length::Fixed(height)).into()
        }
    };
    // A ring over the whole row: a row of a section that is not the user's
    // own is a target like its header (it takes the room into the section).
    let ring = |refused: bool| -> Element<'a, Message> {
        let outline = container(space().width(Fill).height(Fill))
            .width(Fill)
            .height(Length::Fixed(height))
            .style(move |t: &iced::Theme| container::Style {
                border: border::rounded(tokens::SMALL)
                    .color(if refused {
                        t.palette().danger
                    } else {
                        theme::semantic(t).focus
                    })
                    .width(MARK_RING),
                ..container::Style::default()
            });
        if refused {
            // Not by colour alone.
            let words = container(
                text(NOT_ALLOWED)
                    .size(tokens::CAPTION)
                    .style(iced::widget::text::danger)
                    .wrapping(Wrapping::None),
            )
            .width(Fill)
            .height(Length::Fixed(height))
            .padding([0.0, tokens::S])
            .align_right(Fill)
            .center_y(Length::Fixed(height));
            stack![outline, words].into()
        } else {
            outline.into()
        }
    };
    let mut layers = stack![content].width(Fill).height(Length::Fixed(height));
    match overlay_of(mark) {
        Overlay::LineAbove => layers = layers.push(line(true)),
        Overlay::LineBelow => layers = layers.push(line(false)),
        Overlay::Ring => layers = layers.push(ring(false)),
        Overlay::RefusedRing => layers = layers.push(ring(true)),
        Overlay::Nothing => {}
    }
    layers.into()
}

/// The area that makes a row or header a click and a drag: one press that is
/// a click if the pointer stays and a drag if it goes (`drag.rs`), a
/// right-click for the options card, and while a drag runs the pointer's
/// place in it.
fn list_area<'a>(
    content: Element<'a, Message>,
    key: ListKey,
    cue: ListCue,
    height: f32,
    open: Message,
) -> Element<'a, Message> {
    let mut area = mouse_area(with_mark(content, cue.mark, height))
        .interaction(cue.cursor)
        .on_enter(Message::Spaces(SpacesMessage::ListEnter(key.clone())))
        .on_exit(Message::Spaces(SpacesMessage::ListExit(key.clone())))
        .on_press(Message::Spaces(SpacesMessage::ListPress(key.clone())))
        .on_release(Message::Spaces(SpacesMessage::ListRelease(key.clone())))
        .on_right_press(open);
    if cue.dragging {
        area = area.on_move(move |point| {
            Message::Spaces(SpacesMessage::ListOver(
                key.clone(),
                list_drop::halves(point.y, height),
            ))
        });
    } else if cue.pressed {
        area =
            area.on_move(move |point| Message::Spaces(SpacesMessage::ListGrab(key.clone(), point)));
    }
    area.into()
}

/// A room row of the sectioned list with its options: a right-click opens the
/// room's card, and the selected row also has the "⋯" button at its right. A
/// click opens the room and a drag moves it (see [`list_area`]).
fn sectioned_room_row<'a>(
    row_view: Element<'a, Message>,
    room: &RoomSummary,
    selected: bool,
    density: Density,
    cue: ListCue,
) -> Element<'a, Message> {
    let open = Message::SidebarMenu(SidebarMenuMessage::OpenRoom(room.id.clone()));
    let content: Element<'a, Message> = if selected {
        row![
            row_view,
            container(kit::compact_icon_button(Icon::More, open.clone()))
                .center_y(Length::Fixed(row_height(density)))
        ]
        .into()
    } else {
        row_view
    };
    list_area(
        content,
        ListKey::Room(room.id.clone()),
        cue,
        row_height(density),
        open,
    )
}

/// The header of a section: a chevron, the name, "personal" on Starred, the
/// rolled-up badge while it is collapsed, and the "⋯" button. A click on
/// the name collapses or expands it; a right-click or the button opens the
/// section's card.
fn section_header<'a>(app: &App, header: &Header<'a>, density: Density) -> Element<'a, Message> {
    let key = ListKey::Header(header.key.clone());
    let cue = list_cue(app, &key);
    let chevron = if header.collapsed {
        Icon::SectionClosed
    } else {
        Icon::SectionOpen
    };
    // The name is in capitals and turns to the text colour under the pointer
    // (plan C1); a drag over it counts as the pointer.
    let lit = cue.hovered || cue.dragged;
    let name_style = move |t: &iced::Theme| text::Style {
        color: Some(header_name_color(t, lit)),
    };
    let mut content = row![
        text(chevron.glyph())
            .size(tokens::CAPTION)
            .style(name_style)
            .width(Length::Fixed(14.0))
            .wrapping(Wrapping::None),
        container(
            text(header_label(header.name))
                .size(tokens::CAPTION)
                .font(fonts::BODY_SEMIBOLD)
                .style(name_style)
                .wrapping(Wrapping::None)
        )
        .width(Fill)
        .clip(true),
    ]
    .spacing(tokens::XS)
    .align_y(iced::Center);
    if header.key == SectionKey::Starred {
        content = content.push(
            text(PERSONAL_LABEL)
                .size(tokens::CAPTION)
                .style(kit::muted)
                .wrapping(Wrapping::None),
        );
    }
    if header.collapsed {
        content = content.push(badge_view(header.badge, false));
    }
    if cue.mark == Mark::Refused {
        // Not by colour alone.
        content = content.push(
            text(NOT_ALLOWED)
                .size(tokens::CAPTION)
                .style(iced::widget::text::danger)
                .wrapping(Wrapping::None),
        );
    }
    let height = line_height(&Line::Header(header.clone()), density);
    let open = Message::SidebarMenu(SidebarMenuMessage::OpenSection(header.key.clone()));
    let (mark, hovered) = (cue.mark, cue.hovered || cue.dragged);
    // A click on the name collapses or expands it; a drag moves the section.
    // No button: it would swallow the press (see `list_area`).
    let toggle = container(content)
        .width(Fill)
        .center_y(Length::Fixed(height))
        .padding([0.0, tokens::S])
        .style(move |t: &iced::Theme| mark_style(t, mark, hovered));
    let line = row![
        toggle,
        container(kit::compact_icon_button(Icon::More, open.clone()))
            .center_y(Length::Fixed(height))
    ]
    .align_y(iced::Center)
    .height(Length::Fixed(height));
    // The header draws its own ring above: `list_area` adds no ring over it,
    // only the line of a before or after mark.
    let cue = ListCue {
        mark: header_mark(cue.mark),
        ..cue
    };
    list_area(line.into(), key, cue, height, open)
}

/// Width of a person's picture under a voice room.
pub const PERSON_AVATAR: f32 = 20.0;

/// How far a person's line is moved in, so it sits under the room's name.
const PERSON_INDENT: f32 = ACCENT_BAR + tokens::S + tokens::XL;

/// The words of the "and N more" line.
pub fn more_label(hidden: usize) -> String {
    format!("and {hidden} more")
}

/// A person in a voice room's call: a 20 px picture (ringed while they
/// speak), the name and, when muted, the muted icon in the danger colour. A
/// click opens the voice room. Speaking and muted are known only for the call
/// this device is in, so the other rooms' people carry neither mark.
fn person_line<'a>(app: &App, person: &Person<'a>, density: Density) -> Element<'a, Message> {
    let name = person.name.clone();
    let picture = kit::speaking_ring_sized(
        kit::avatar_sized(
            &person.member.user_id,
            &name,
            person.member.avatar.as_ref(),
            &app.avatars,
            PERSON_AVATAR,
        ),
        PERSON_AVATAR,
        person.speaking,
    );
    let mut content = row![
        picture,
        container(text(name).size(tokens::CAPTION).wrapping(Wrapping::None))
            .width(Fill)
            .clip(true),
    ]
    .spacing(tokens::S)
    .align_y(iced::Center);
    if person.muted {
        content = content.push(
            text(Icon::Muted.glyph())
                .size(tokens::BODY)
                .wrapping(Wrapping::None)
                .style(iced::widget::text::danger),
        );
    }
    person_button(content.into(), person.room, density)
}

/// "and N more" under a voice room.
fn more_line<'a>(more: &More<'a>, density: Density) -> Element<'a, Message> {
    let content = text(more_label(more.hidden))
        .size(tokens::CAPTION)
        .wrapping(Wrapping::None)
        .style(kit::muted);
    person_button(content.into(), more.room, density)
}

/// A line under a voice room: indented, quiet, and a click selects the room.
fn person_button<'a>(
    content: Element<'a, Message>,
    room: &RoomId,
    density: Density,
) -> Element<'a, Message> {
    let height = match density {
        Density::Comfortable => sections::PERSON_HEIGHT,
        Density::Compact => sections::COMPACT_PERSON_HEIGHT,
    };
    button(content)
        .on_press(Message::SelectRoom(room.clone()))
        .width(Fill)
        .height(Length::Fixed(height))
        .padding(iced::Padding {
            top: 0.0,
            right: tokens::S,
            bottom: 0.0,
            left: PERSON_INDENT,
        })
        .clip(true)
        .style(kit::ghost_style)
        .into()
}

/// What a refused header says, besides its red ring.
pub const NOT_ALLOWED: &str = "Not allowed";

/// The 3 px bar at the left of a row: the accent colour while selected,
/// clear otherwise, so the row keeps its shape.
fn accent_bar<'a>(selected: bool) -> Element<'a, Message> {
    container(space().width(ACCENT_BAR).height(Fill))
        .style(move |t: &iced::Theme| container::Style {
            background: selected.then(|| Background::Color(t.palette().primary)),
            ..container::Style::default()
        })
        .into()
}

/// The style of a row's button: quiet, with the selection colour while the
/// room is open.
fn row_style(selected: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |t, status| {
        let base = kit::ghost_style(t, status);
        if selected {
            button::Style {
                background: Some(Background::Color(theme::semantic(t).selection)),
                ..base
            }
        } else {
            base
        }
    }
}

/// One fixed-height row: avatar, name, time, the lock, the preview (not in
/// Compact), the Voice, Invite and Muted tags, the call pill and the count.
/// `mode` is the room's notification mode as the app knows it; `call` the
/// room's call, if any.
fn room_row<'a>(
    room: &'a RoomSummary,
    env: &RowEnv<'_>,
    selected: bool,
    mode: Option<RoomNotificationModeView>,
    call: Option<CallPill>,
) -> Element<'a, Message> {
    room_row_in(room, env, selected, mode, call, Pointer::Button)
}

/// How a row takes the pointer.
#[derive(Debug, Clone, Copy)]
enum Pointer {
    /// A button: a click opens the room. The flat list, where nothing drags.
    Button,
    /// A plain box inside the sectioned list's `mouse_area`, which decides
    /// between a click and a drag. A button would swallow the press.
    Area {
        /// The pointer is over it.
        hovered: bool,
        /// It is being dragged.
        dragged: bool,
    },
}

/// [`room_row`] for the way the row takes the pointer.
fn room_row_in<'a>(
    room: &'a RoomSummary,
    env: &RowEnv<'_>,
    selected: bool,
    mode: Option<RoomNotificationModeView>,
    call: Option<CallPill>,
    pointer: Pointer,
) -> Element<'a, Message> {
    let comfortable = shows_preview(env.density);
    let name = row_name(&room.name);
    let avatar = avatar_view(
        room,
        &name,
        env.avatars,
        if comfortable {
            AVATAR_SLOT
        } else {
            COMPACT_AVATAR_SLOT
        },
    );
    // Text that does not wrap is drawn past its box, so the box clips it:
    // the end of a long name never lands under the lock, the time, the tags
    // or the scrollbar.
    let mut title = row![
        container(text(name).size(tokens::BODY).wrapping(Wrapping::None))
            .width(Fill)
            .clip(true)
    ]
    .spacing(tokens::S)
    .align_y(iced::Center);
    if let Some(lock) = encrypted_mark(room) {
        title = title.push(text(lock).size(tokens::CAPTION));
    }
    let mut marks = row![].spacing(tokens::XS).align_y(iced::Center);
    if room.is_call_room {
        marks = marks.push(kit::tag(VOICE_TAG));
    }
    if room.is_invite {
        marks = marks.push(kit::tag("Invite"));
    }
    if let Some(mark) = muted_mark(mode) {
        marks = marks.push(kit::tag(mark));
    }
    marks = marks
        .push(call_pill(call))
        .push(badge_view(badge_for(room), muted_mark(mode).is_some()));
    let body: Element<'a, Message> = if comfortable {
        if let Some(last) = &room.last_message {
            title = title.push(
                text(row_time(last.timestamp_ms, env.now_ms, &Local))
                    .size(tokens::CAPTION)
                    .wrapping(Wrapping::None)
                    .style(kit::muted),
            );
        }
        let preview = preview_line(room).unwrap_or_default();
        column![
            title,
            row![
                container(
                    text(preview)
                        .size(tokens::CAPTION)
                        .wrapping(Wrapping::None)
                        .style(kit::muted)
                )
                .width(Fill)
                .clip(true),
                marks
            ]
            .spacing(tokens::S)
            .align_y(iced::Center)
        ]
        .spacing(2)
        .width(Fill)
        .into()
    } else {
        title.push(marks).into()
    };
    let content = row![avatar, body].spacing(tokens::S).align_y(iced::Center);
    let height = row_height(env.density);
    let body: Element<'a, Message> = match pointer {
        Pointer::Button => button(content)
            .on_press(Message::SelectRoom(room.id.clone()))
            .width(Fill)
            .height(Length::Fixed(height))
            .padding([tokens::XS, tokens::S])
            .clip(true)
            .style(row_style(selected))
            .into(),
        Pointer::Area { hovered, dragged } => container(content)
            .width(Fill)
            .height(Length::Fixed(height))
            .padding([tokens::XS, tokens::S])
            .clip(true)
            .style(row_box_style(selected, hovered || dragged))
            .into(),
    };
    row![accent_bar(selected), body]
        .height(Length::Fixed(height))
        .into()
}

/// The colour of a channel row's name (plan C2): the text colour when
/// unread; the muted colour when read, pulled toward the background only as
/// far as needed for it never to outshine the text colour on any background a
/// row has (the muted colour of a theme with grey text is brighter than its
/// text); and, in Mute mode, that colour dimmed further, as far as it keeps
/// 3:1 on every background a row has.
pub(crate) fn channel_name_color(theme: &iced::Theme, emphasis: Emphasis) -> Color {
    match emphasis {
        Emphasis::Unread => theme.palette().text,
        Emphasis::Read => read_color(theme),
        Emphasis::Muted => dimmed(theme, read_color(theme)),
    }
}

/// The backgrounds a row's name can sit on: the list's, the hover and the
/// selection.
fn row_grounds(theme: &iced::Theme) -> [Color; 3] {
    let sem = theme::semantic(theme);
    [theme.palette().background, sem.message_hover, sem.selection]
}

/// `color` moved `t` of the way to the list's background.
fn toward_background(theme: &iced::Theme, color: Color, t: f32) -> Color {
    let back = theme.palette().background;
    Color::from_rgb(
        color.r + (back.r - color.r) * t,
        color.g + (back.g - color.g) * t,
        color.b + (back.b - color.b) * t,
    )
}

/// The muted colour, mixed toward the background in 5 % steps until its
/// contrast on every row background is no more than the text colour's.
fn read_color(theme: &iced::Theme) -> Color {
    let muted = theme::semantic(theme).muted;
    let text = theme.palette().text;
    let grounds = row_grounds(theme);
    (0..=20u8)
        .map(|step| toward_background(theme, muted, f32::from(step) / 20.0))
        .find(|c| {
            grounds
                .iter()
                .all(|g| theme::contrast(*c, *g) <= theme::contrast(text, *g))
        })
        .unwrap_or(muted)
}

/// The words of a category header: the name in capitals, then cleaned and
/// capped, so what is drawn is within the cap and the combining-mark limit
/// whatever the capitals did to the length (plan C1).
pub(crate) fn header_label(name: &str) -> String {
    row_name(&name.to_uppercase())
}

/// The colour of a category header's name: muted, the text colour under the
/// pointer (plan C1).
pub(crate) fn header_name_color(theme: &iced::Theme, hovered: bool) -> Color {
    if hovered {
        theme.palette().text
    } else {
        theme::semantic(theme).muted
    }
}

/// `color` mixed toward the list's background in 5 % steps up to 40 %, the
/// furthest that still reads at [`DIM_CONTRAST`] on the list, the hover and
/// the selection. `color` itself when no step does.
fn dimmed(theme: &iced::Theme, color: Color) -> Color {
    let grounds = row_grounds(theme);
    (1..=8u8)
        .rev()
        .map(|step| toward_background(theme, color, f32::from(step) / 20.0))
        .find(|mixed| {
            grounds
                .iter()
                .all(|ground| theme::contrast(*mixed, *ground) >= DIM_CONTRAST)
        })
        .unwrap_or(color)
}

/// The least contrast a dimmed (muted-mode) name keeps: the 3:1 that a read
/// name must reach, so the dimming never turns a name unreadable.
const DIM_CONTRAST: f32 = 3.0;

/// What a channel row shows at its right, besides the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChannelMarks {
    /// The badge: always [`badge_for`]'s.
    pub badge: Badge,
    /// The room is an invite.
    pub invite: bool,
    /// "Muted", while the room is in Mute mode.
    pub muted: Option<&'static str>,
    /// The lock, when the room is known to be encrypted.
    pub lock: Option<&'static str>,
}

/// The marks of the channel row of `room`.
pub(crate) fn channel_marks(
    room: &RoomSummary,
    mode: Option<RoomNotificationModeView>,
) -> ChannelMarks {
    ChannelMarks {
        badge: badge_for(room),
        invite: room.is_invite,
        muted: muted_mark(mode),
        lock: encrypted_mark(room),
    }
}

/// Width of the slot at a channel row's left edge: the unread pill, or the
/// selection's accent bar.
const CHANNEL_EDGE: f32 = 4.0;

/// The size of a channel row's avatar: the flat list's slot in each density,
/// so a room looks the same in both lists.
pub const fn channel_avatar_size(density: Density) -> f32 {
    if shows_preview(density) {
        AVATAR_SLOT
    } else {
        COMPACT_AVATAR_SLOT
    }
}

/// The side of the speaker badge on a voice room's avatar.
const fn voice_badge_size(density: Density) -> f32 {
    if shows_preview(density) { 16.0 } else { 14.0 }
}

/// `color` at `alpha` of its own opacity.
fn with_alpha(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

/// What the row behind a channel's avatar looks like: selected, under the
/// pointer, or plain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lit {
    /// The list's own background.
    Plain,
    /// The pointer is over the row (or it is dragged).
    Hovered,
    /// The room is open.
    Selected,
}

/// The fill of the speaker badge's circle: the colour of the row behind it
/// (as [`row_box_style`] paints it), so the circle never shows as a disc.
fn badge_fill(theme: &iced::Theme, lit: Lit) -> Color {
    let sem = theme::semantic(theme);
    match lit {
        Lit::Plain => theme.palette().background,
        Lit::Hovered => sem.message_hover,
        Lit::Selected => sem.selection,
    }
}

/// A room's avatar (its image, or its first letter) and, for a voice room,
/// the small speaker at its bottom right corner: a filled circle in the
/// list's background colour behind the icon, so it reads on an image and on a
/// letter alike. The badge sits inside the avatar's square, so the slot and
/// the row height do not change.
fn channel_avatar<'a>(
    room: &RoomSummary,
    name: &str,
    avatars: &Avatars,
    density: Density,
    lit: Lit,
    alpha: f32,
) -> Element<'a, Message> {
    let size = channel_avatar_size(density);
    let avatar = avatar_view_alpha(room, name, avatars, size, alpha);
    if !voice_badge(room) {
        return container(avatar).width(size).height(size).into();
    }
    let side = voice_badge_size(density);
    let badge = container(
        text(Icon::Speaker.glyph())
            .size(side - 6.0)
            .wrapping(Wrapping::None),
    )
    .center_x(side)
    .center_y(side)
    .style(move |t: &iced::Theme| container::Style {
        background: Some(Background::Color(with_alpha(badge_fill(t, lit), alpha))),
        text_color: Some(with_alpha(t.palette().text, alpha)),
        border: border::rounded(side / 2.0)
            .color(with_alpha(theme::semantic(t).border, alpha))
            .width(1.0),
        ..container::Style::default()
    });
    let corner = container(badge)
        .width(size)
        .height(size)
        .align_right(size)
        .align_bottom(size);
    stack![avatar, corner].width(size).height(size).into()
}

/// The 4 x 8 px pill at the list's left edge of an unread channel, or the
/// 3 px accent bar of the selected one, in a slot that is always the same
/// width so the row keeps its shape.
fn channel_edge<'a>(selected: bool, emphasis: Emphasis) -> Element<'a, Message> {
    let slot = container(accent_bar(selected))
        .width(CHANNEL_EDGE)
        .height(Fill);
    if selected || emphasis != Emphasis::Unread {
        return slot.into();
    }
    container(
        container(space().width(CHANNEL_EDGE).height(8.0)).style(|t: &iced::Theme| {
            container::Style {
                background: Some(Background::Color(t.palette().text)),
                border: border::rounded(CHANNEL_EDGE / 2.0),
                ..container::Style::default()
            }
        }),
    )
    .width(CHANNEL_EDGE)
    .height(Fill)
    .center_y(Fill)
    .into()
}

/// One row of a space's list: the room's avatar (its first letter without an
/// image; a voice room has a small speaker at its corner), the name, and the
/// marks. No preview and no time. An
/// unread room's name is semibold in the text colour with the pill at the
/// left edge, a read one's is muted, and a muted room's is dimmed. The row is
/// as tall as [`room_row`], so the windowing is the same (plan C2).
fn channel_row<'a>(
    room: &'a RoomSummary,
    env: &RowEnv<'_>,
    selected: bool,
    mode: Option<RoomNotificationModeView>,
    call: Option<CallPill>,
    cue: ListCue,
) -> Element<'a, Message> {
    let name = row_name(&room.name);
    let emphasis = channel_emphasis(room, mode);
    let marks = channel_marks(room, mode);
    let lit = if selected {
        Lit::Selected
    } else if cue.hovered || cue.dragged {
        Lit::Hovered
    } else {
        Lit::Plain
    };
    let glyph = channel_avatar(room, &name, env.avatars, env.density, lit, 1.0);
    let weight = if emphasis == Emphasis::Unread {
        fonts::BODY_SEMIBOLD
    } else {
        fonts::BODY
    };
    let mut line = row![
        glyph,
        container(
            text(name)
                .size(tokens::BODY)
                .font(weight)
                .wrapping(Wrapping::None)
                .style(move |t: &iced::Theme| text::Style {
                    color: Some(channel_name_color(t, emphasis)),
                })
        )
        .width(Fill)
        .clip(true),
    ]
    .spacing(tokens::S)
    .align_y(iced::Center);
    if let Some(lock) = marks.lock {
        line = line.push(text(lock).size(tokens::CAPTION));
    }
    if marks.invite {
        line = line.push(kit::tag("Invite"));
    }
    if let Some(mark) = marks.muted {
        line = line.push(kit::tag(mark));
    }
    line = line
        .push(call_pill(call))
        .push(badge_view(marks.badge, marks.muted.is_some()));
    let height = row_height(env.density);
    let body = container(line)
        .width(Fill)
        .center_y(Length::Fixed(height))
        .padding([tokens::XS, tokens::S])
        .clip(true)
        .style(row_box_style(selected, cue.hovered || cue.dragged));
    row![channel_edge(selected, emphasis), body]
        .height(Length::Fixed(height))
        .into()
}

/// How opaque the ghost tile of a dragged row or header is.
pub const GHOST_ALPHA: f32 = 0.7;

/// The width of a room's ghost tile: a row of the list, less the scrollbar's
/// gutter.
pub const GHOST_ROOM_WIDTH: f32 = main_view::LIST_PANE_WIDTH - SCROLLBAR_GUTTER;

/// The width of a header's ghost tile.
pub const GHOST_HEADER_WIDTH: f32 = 180.0;

/// The size of the ghost tile of `item` in `density`: the row's own height.
pub fn ghost_size(item: &GhostItem<'_>, density: Density) -> iced::Size {
    match item {
        GhostItem::Room(_) => iced::Size::new(GHOST_ROOM_WIDTH, row_height(density)),
        GhostItem::Header(header) => iced::Size::new(
            GHOST_HEADER_WIDTH,
            line_height(&Line::Header(header.clone()), density),
        ),
    }
}

/// The ghost tile of a dragged row or header: a copy at [`GHOST_ALPHA`] on
/// the raised surface with the quiet border, so it reads in every theme. A
/// room shows its avatar (with the speaker on a voice room) and its name; a
/// header its name in capitals. It holds no widget that takes the pointer, so
/// whatever is under it still gets every pointer event.
pub fn ghost_tile<'a>(
    app: &'a App,
    item: &GhostItem<'a>,
    density: Density,
) -> Element<'a, Message> {
    let size = ghost_size(item, density);
    let content: Element<'a, Message> = match item {
        GhostItem::Room(room) => {
            let name = row_name(&room.name);
            let emphasis = channel_emphasis(room, app.effective_room_mode(&room.id));
            let weight = if emphasis == Emphasis::Unread {
                fonts::BODY_SEMIBOLD
            } else {
                fonts::BODY
            };
            let avatar =
                channel_avatar(room, &name, &app.avatars, density, Lit::Plain, GHOST_ALPHA);
            row![
                avatar,
                container(
                    text(name)
                        .size(tokens::BODY)
                        .font(weight)
                        .wrapping(Wrapping::None)
                        .style(move |t: &iced::Theme| text::Style {
                            color: Some(with_alpha(channel_name_color(t, emphasis), GHOST_ALPHA)),
                        })
                )
                .width(Fill)
                .clip(true),
            ]
            .spacing(tokens::S)
            .align_y(iced::Center)
            .into()
        }
        GhostItem::Header(header) => container(
            text(header_label(header.name))
                .size(tokens::CAPTION)
                .font(fonts::BODY_SEMIBOLD)
                .wrapping(Wrapping::None)
                .style(|t: &iced::Theme| text::Style {
                    color: Some(with_alpha(t.palette().text, GHOST_ALPHA)),
                }),
        )
        .width(Fill)
        .clip(true)
        .into(),
    };
    container(content)
        .width(size.width)
        .height(size.height)
        .center_y(size.height)
        .padding([tokens::XS, tokens::S])
        .clip(true)
        .style(|t: &iced::Theme| {
            let sem = theme::semantic(t);
            container::Style {
                background: Some(Background::Color(with_alpha(
                    sem.surface_raised,
                    GHOST_ALPHA,
                ))),
                border: border::rounded(tokens::SMALL)
                    .color(with_alpha(sem.border, GHOST_ALPHA))
                    .width(1.0),
                ..container::Style::default()
            }
        })
        .into()
}

/// `base` with the ghost tile of the dragged row or header over it, or with
/// nothing over it: always a stack of two, so the widget tree keeps its shape
/// when the ghost comes and goes. The layer is the window, holds nothing that
/// takes the pointer, and is below every card and dialog (those wrap this).
pub fn ghost_layer<'a>(base: Element<'a, Message>, app: &'a App) -> Element<'a, Message> {
    let top: Element<'a, Message> = match app.ghost() {
        None => space().into(),
        Some(ghost) => iced::widget::responsive(move |window| {
            let density = app.density();
            let tile = ghost_tile(app, &ghost.item, density);
            let size = ghost_size(&ghost.item, density);
            // The grab is kept on the tile (a header's is narrower than its row).
            let grab = crate::drag::ghost_grab(ghost.grab, size);
            let at = crate::drag::ghost_position(ghost.pointer, grab, size, window);
            iced::widget::pin(tile).x(at.x).y(at.y).into()
        })
        .into(),
    };
    stack([base, top]).into()
}

/// A channel row on its own, for the layout tests: no pointer cue, no
/// avatars loaded.
pub fn channel_row_alone<'a>(
    room: &'a RoomSummary,
    density: Density,
    selected: bool,
    mode: Option<RoomNotificationModeView>,
) -> Element<'a, Message> {
    let avatars = Avatars::default();
    let env = RowEnv {
        avatars: &avatars,
        density,
        now_ms: 0,
    };
    let cue = ListCue {
        mark: Mark::None,
        cursor: mouse::Interaction::Grab,
        dragging: false,
        hovered: false,
        dragged: false,
        pressed: false,
    };
    channel_row(room, &env, selected, mode, None, cue)
}

/// The look of a row that is a box, not a button: the selection colour while
/// the room is open, the hover colour under the pointer (as [`row_style`]).
fn row_box_style(selected: bool, hovered: bool) -> impl Fn(&iced::Theme) -> container::Style {
    move |t| {
        let sem = theme::semantic(t);
        let fill = if selected {
            Some(sem.selection)
        } else if hovered {
            Some(sem.message_hover)
        } else {
            None
        };
        container::Style {
            text_color: Some(t.palette().text),
            background: fill.map(Background::Color),
            border: border::rounded(tokens::SMALL),
            ..container::Style::default()
        }
    }
}

/// One row of the rail: the avatar with an unread dot and, in a call, a
/// speaker dot, and a tooltip with the room's name and what the dots say.
fn rail_row<'a>(
    room: &'a RoomSummary,
    avatars: &Avatars,
    selected: bool,
    mode: Option<RoomNotificationModeView>,
    call: Option<CallPill>,
) -> Element<'a, Message> {
    let name = row_name(&room.name);
    let badge = badge_for(room);
    let marks = rail_marks(room);
    let words = rail_tip(&name, badge, call, muted_mark(mode), &marks);
    let avatar = avatar_view(room, &name, avatars, AVATAR_SLOT);
    let corner = |dot: Element<'a, Message>, top: bool| {
        let layer = container(dot).width(Fill).height(Fill).align_right(Fill);
        if top {
            layer.align_top(Fill)
        } else {
            layer.align_bottom(Fill)
        }
    };
    let picture = stack![
        avatar,
        corner(unread_dot(badge, muted_mark(mode).is_some()), true),
        corner(speaker_dot(call), false),
        rail_mark_layer(&marks),
    ];
    let height = Length::Fixed(RAIL_ROW_HEIGHT);
    let entry = button(container(picture).center(Fill))
        .on_press(Message::SelectRoom(room.id.clone()))
        .width(Fill)
        .height(height)
        .padding(tokens::XS)
        .style(row_style(selected));
    row![
        accent_bar(selected),
        Tooltip::new(entry, text(words).size(tokens::CAPTION), Position::Right)
            .gap(tokens::XS)
            .padding(tokens::S)
            .style(kit::card_style),
    ]
    .height(height)
    .into()
}

/// The unread dot on a rail avatar: the highlight colour for mentions, the
/// badge colour for other unread messages, an outline for a muted room, and
/// nothing when all is read.
fn unread_dot<'a>(badge: Badge, muted: bool) -> Element<'a, Message> {
    let size = 12.0;
    match badge {
        Badge::None => space().width(0).into(),
        badge => container(space().width(size).height(size))
            .style(move |t: &iced::Theme| {
                let sem = theme::semantic(t);
                let (fill, ring) = match badge {
                    Badge::Highlight(_) => (Some(sem.highlight), t.palette().background),
                    _ if muted => (None, sem.muted),
                    _ => (Some(sem.badge), t.palette().background),
                };
                container::Style {
                    background: fill.map(Background::Color),
                    border: border::rounded(size / 2.0).color(ring).width(2),
                    ..container::Style::default()
                }
            })
            .into(),
    }
}

/// Side of a mark or call chip on a rail avatar. Two chips along the bottom
/// left and the call chip at the bottom right must fit the 40 px avatar with
/// room between them: 12 + 1 + 12 and 12 leave 3 px.
const CHIP: f32 = 12.0;

/// The marks of a rail row on the avatar: Invite at the top left, the voice
/// and lock marks along the bottom left.
fn rail_mark_layer<'a>(marks: &[RailMark]) -> Element<'a, Message> {
    let chip = |mark: RailMark| -> Element<'a, Message> {
        let invite = mark == RailMark::Invite;
        container(
            text(mark.icon().glyph())
                .size(8)
                .wrapping(Wrapping::None)
                .center(),
        )
        .width(CHIP)
        .height(CHIP)
        .center(CHIP)
        .style(move |t: &iced::Theme| {
            let sem = theme::semantic(t);
            let (fill, on) = if invite {
                (sem.highlight, sem.highlight_text)
            } else {
                (sem.surface_raised, t.palette().text)
            };
            container::Style {
                background: Some(Background::Color(fill)),
                text_color: Some(on),
                border: border::rounded(CHIP / 2.0).color(sem.muted).width(1),
                ..container::Style::default()
            }
        })
        .into()
    };
    let top: Element<'a, Message> = if marks.contains(&RailMark::Invite) {
        chip(RailMark::Invite)
    } else {
        space().width(0).into()
    };
    let bottom = marks
        .iter()
        .filter(|m| **m != RailMark::Invite)
        .fold(row![].spacing(1), |r, m| r.push(chip(*m)));
    column![
        container(top).width(Fill).align_left(Fill),
        space().height(Fill),
        container(bottom).width(Fill).align_left(Fill),
    ]
    .width(Fill)
    .height(Fill)
    .into()
}

/// The speaker dot on a rail avatar while the room has a call: the highlight
/// colour while it rings, the badge colour otherwise.
fn speaker_dot<'a>(call: Option<CallPill>) -> Element<'a, Message> {
    let Some(call) = call else {
        return space().width(0).into();
    };
    container(
        text(Icon::Speaker.glyph())
            .size(8)
            .wrapping(Wrapping::None)
            .center(),
    )
    .width(CHIP)
    .height(CHIP)
    .center(CHIP)
    .style(move |t: &iced::Theme| {
        let sem = theme::semantic(t);
        let (fill, on) = if call.ringing {
            (sem.highlight, sem.highlight_text)
        } else {
            (sem.badge, sem.badge_text)
        };
        container::Style {
            background: Some(Background::Color(fill)),
            text_color: Some(on),
            border: border::rounded(CHIP / 2.0),
            ..container::Style::default()
        }
    })
    .into()
}

/// The room's avatar, `size` wide: the decoded image once it arrived,
/// otherwise (and when it is unavailable) a round placeholder with the
/// initials of the already sanitised `name`. `alpha` is its opacity (1.0, or
/// the ghost's).
fn avatar_view_alpha<'a>(
    room: &RoomSummary,
    name: &str,
    avatars: &Avatars,
    size: f32,
    alpha: f32,
) -> Element<'a, Message> {
    let ready = room
        .avatar
        .as_ref()
        .and_then(|id| avatars.state(id))
        .and_then(|state| match state {
            AvatarState::Ready(handle) => Some(handle),
            AvatarState::Unavailable => None,
        });
    if let Some(handle) = ready {
        return image(handle.clone())
            .width(size)
            .height(size)
            .border_radius(size / 2.0)
            .opacity(alpha)
            .into();
    }
    let label = initials(name);
    let fill = if label == NEUTRAL_INITIALS {
        None
    } else {
        Some(placeholder_color(&room.id))
    };
    container(
        text(label)
            .size(if size > COMPACT_AVATAR_SLOT {
                tokens::BODY_LARGE
            } else {
                tokens::CAPTION
            })
            .color(Color {
                a: alpha,
                ..Color::WHITE
            }),
    )
    .width(size)
    .height(size)
    .center_x(size)
    .center_y(size)
    .style(move |t| container::Style {
        background: Some(Background::Color(Color {
            a: alpha,
            ..fill.unwrap_or_else(|| theme::semantic(t).muted)
        })),
        border: border::rounded(size / 2.0),
        ..container::Style::default()
    })
    .into()
}

/// [`avatar_view_alpha`] at full strength.
pub(crate) fn avatar_view<'a>(
    room: &RoomSummary,
    name: &str,
    avatars: &Avatars,
    size: f32,
) -> Element<'a, Message> {
    avatar_view_alpha(room, name, avatars, size, 1.0)
}

/// The call pill: the speaker and how many devices are in the room's call,
/// in the highlight colour while the room rings. An empty space without a
/// call, so a row keeps its widget types.
pub(crate) fn call_pill<'a>(call: Option<CallPill>) -> Element<'a, Message> {
    match call {
        None => space().width(0).into(),
        Some(call) => kit::icon_tag(Icon::Speaker, call_label(call), call.ringing),
    }
}

/// The badge at the right of a row. A muted room's count is drawn as an
/// outline in the muted text colour instead of a filled badge; mentions keep
/// their highlight.
pub(crate) fn badge_view<'a>(badge: Badge, muted: bool) -> Element<'a, Message> {
    match badge {
        Badge::None => space().width(0).into(),
        Badge::Highlight(count) => pill(count_label(count), Tone::Highlight),
        Badge::Count(count) if muted => pill(count_label(count), Tone::Muted),
        Badge::Count(count) => kit::count_badge(count),
        Badge::Dot => container(space().width(10).height(10))
            .style(|t| container::Style {
                background: Some(Background::Color(theme::semantic(t).badge)),
                border: border::rounded(5.0),
                ..container::Style::default()
            })
            .into(),
    }
}

/// How a count badge is drawn when it is not the plain [`kit::count_badge`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tone {
    /// Mentions.
    Highlight,
    /// Unread notifications of a muted room.
    Muted,
}

/// A rounded badge with a count; the highlight colour for mentions, an
/// outline for a muted room.
fn pill<'a>(label: String, tone: Tone) -> Element<'a, Message> {
    container(text(label).size(tokens::CAPTION))
        .padding([1.0, 7.0])
        .style(move |t| {
            let colours = theme::semantic(t);
            match tone {
                Tone::Highlight => container::Style {
                    text_color: Some(colours.highlight_text),
                    background: Some(Background::Color(colours.highlight)),
                    border: border::rounded(tokens::ROUND),
                    ..container::Style::default()
                },
                Tone::Muted => container::Style {
                    text_color: Some(colours.muted),
                    border: border::rounded(tokens::ROUND).color(colours.muted).width(1),
                    ..container::Style::default()
                },
            }
        })
        .into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_free_space_is_the_rest_of_the_viewport_and_at_least_one_row() {
        assert_eq!(filler_height(100.0, 600.0, 56.0), 500.0);
        assert_eq!(filler_height(590.0, 600.0, 56.0), 56.0);
        assert_eq!(filler_height(2000.0, 600.0, 56.0), 56.0);
        assert_eq!(filler_height(0.0, 0.0, 56.0), 56.0, "no viewport known yet");
    }

    const MARKS: [Mark; 5] = [
        Mark::None,
        Mark::Before,
        Mark::After,
        Mark::Into,
        Mark::Refused,
    ];

    #[test]
    fn a_header_hands_on_the_line_of_a_before_or_after_and_nothing_else() {
        let got = MARKS.map(header_mark);
        assert_eq!(
            got,
            [
                Mark::None,
                Mark::Before,
                Mark::After,
                Mark::None,
                Mark::None
            ]
        );
    }

    /// What each kind of line draws for each mark: a room row has its overlay
    /// from `with_mark` alone; a header has the line from `with_mark` and its
    /// ring or refusal from its own style, never both over each other.
    #[test]
    fn each_kind_of_line_draws_each_mark_once() {
        use Overlay::{LineAbove, LineBelow, Nothing, RefusedRing, Ring};
        let row = MARKS.map(overlay_of);
        assert_eq!(row, [Nothing, LineAbove, LineBelow, Ring, RefusedRing]);
        // A header: the overlay of `with_mark` plus its own ring (Into and
        // Refused only, see `mark_style`).
        let own_ring = |mark: Mark| matches!(mark, Mark::Into | Mark::Refused);
        let header: Vec<(Overlay, bool)> = MARKS
            .iter()
            .map(|m| (overlay_of(header_mark(*m)), own_ring(*m)))
            .collect();
        assert_eq!(
            header,
            [
                (Nothing, false),
                (LineAbove, false),
                (LineBelow, false),
                (Nothing, true),
                (Nothing, true)
            ]
        );
        // Every mark shows something on both kinds of line, exactly once.
        for (m, (over, ring)) in MARKS.iter().zip(&header).skip(1) {
            assert!(
                (*over != Nothing) ^ *ring,
                "{m:?} is drawn once on a header"
            );
        }
    }

    use super::*;

    fn env(avatars: &Avatars) -> RowEnv<'_> {
        RowEnv {
            avatars,
            density: Density::Comfortable,
            now_ms: 0,
        }
    }

    #[test]
    fn only_a_room_known_to_be_encrypted_has_a_lock() {
        let mut room = RoomSummary::new(twicecat_core::api::RoomId::from("!r:x"), "r".to_owned());
        assert_eq!(encrypted_mark(&room), None);
        room.is_encrypted = true;
        assert_eq!(encrypted_mark(&room), Some(LOCK_MARK));
    }

    #[test]
    fn rows_build_in_every_notification_mode() {
        let mut room = RoomSummary::new(twicecat_core::api::RoomId::from("!r:x"), "r".to_owned());
        room.unread_notifications = 3;
        let avatars = Avatars::default();
        for mode in [
            None,
            Some(RoomNotificationModeView::AllMessages),
            Some(RoomNotificationModeView::MentionsAndKeywords),
            Some(RoomNotificationModeView::Mute),
        ] {
            let _: Element<'_, Message> = room_row(&room, &env(&avatars), false, mode, None);
        }
        room.highlights = 1;
        let _: Element<'_, Message> = room_row(
            &room,
            &env(&avatars),
            true,
            Some(RoomNotificationModeView::Mute),
            None,
        );
    }

    #[test]
    fn rows_build_with_a_voice_tag_and_every_call_pill() {
        let mut room = RoomSummary::new(twicecat_core::api::RoomId::from("!r:x"), "r".to_owned());
        room.is_call_room = true;
        let avatars = Avatars::default();
        for call in [
            None,
            Some(CallPill {
                count: 2,
                ringing: false,
            }),
            Some(CallPill {
                count: 0,
                ringing: true,
            }),
        ] {
            let _: Element<'_, Message> = room_row(&room, &env(&avatars), false, None, call);
        }
    }

    #[test]
    fn the_speaking_ring_stands_out_from_every_row_background_in_every_theme() {
        // A person's line sits on the pane (the sidebar colour), takes the
        // hover colour under the pointer, and the selection colour is the
        // neighbouring room's: the ring (non-text, 3:1) must show on all.
        for t in theme::test_themes() {
            let sem = theme::semantic(&t);
            for (name, bg) in [
                ("sidebar", sem.sidebar),
                ("surface", sem.surface),
                ("surface-raised", sem.surface_raised),
                ("hover", sem.message_hover),
                ("selection", sem.selection),
            ] {
                let ratio = theme::contrast(sem.speaking, bg);
                assert!(ratio >= 3.0, "speaking ring on {name}: {ratio}");
            }
        }
    }

    #[test]
    fn the_more_line_says_how_many_are_not_shown() {
        assert_eq!(more_label(1), "and 1 more");
        assert_eq!(more_label(17), "and 17 more");
    }

    #[test]
    fn tab_labels() {
        let labels: Vec<_> = TABS.iter().map(|t| tab_label(*t)).collect();
        assert_eq!(
            labels,
            [
                "All",
                "Unreads",
                "Mentions",
                "DMs",
                "Rooms",
                "Favourites",
                "Low priority",
                "Invites",
                "Spaces"
            ]
        );
    }

    #[test]
    fn count_labels_cap_at_99() {
        assert_eq!(count_label(1), "1");
        assert_eq!(count_label(99), "99");
        assert_eq!(count_label(100), "99+");
        assert_eq!(count_label(u64::MAX), "99+");
    }

    #[test]
    fn avatar_views_build_for_every_state() {
        use twicecat_core::api::{AvatarId, AvatarImage, RoomId};

        let mut avatars = Avatars::default();
        let id = AvatarId::from("mxc://x/a");
        let _ = avatars.take_unrequested([id.clone()]);
        let mut room = RoomSummary::new(RoomId::from("!r:x"), String::new());
        room.avatar = Some(id.clone());
        // Requested, not answered: placeholder.
        let _: Element<'_, Message> = avatar_view(&room, "", &avatars, AVATAR_SLOT);
        // Unavailable: placeholder.
        avatars.answer(&id, None);
        let _: Element<'_, Message> = avatar_view(&room, "Room", &avatars, AVATAR_SLOT);
        // No avatar at all.
        room.avatar = None;
        let _: Element<'_, Message> = avatar_view(&room, "#x", &avatars, COMPACT_AVATAR_SLOT);
        // Ready: the image.
        let img = ::image::RgbaImage::from_pixel(2, 2, ::image::Rgba([1, 2, 3, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        img.write_to(&mut png, ::image::ImageFormat::Png)
            .expect("encode");
        let id2 = AvatarId::from("mxc://x/b");
        let _ = avatars.take_unrequested([id2.clone()]);
        avatars.answer(&id2, Some(AvatarImage::from(png.into_inner())));
        room.avatar = Some(id2);
        let _: Element<'_, Message> = avatar_view(&room, "Room", &avatars, AVATAR_SLOT);
    }

    #[test]
    fn the_pane_is_300_wide_and_the_rail_72() {
        use twicecat_core::api::RoomListChoice;
        assert_eq!(pane_width(RoomListChoice::Expanded), 300.0);
        assert_eq!(pane_width(RoomListChoice::Rail), 72.0);
    }

    #[test]
    fn only_comfortable_rows_show_a_preview() {
        assert!(shows_preview(Density::Comfortable));
        assert!(!shows_preview(Density::Compact));
    }

    #[test]
    fn the_last_message_time_is_the_clock_today_and_the_day_before() {
        use chrono::{FixedOffset, TimeZone};
        let tz = FixedOffset::east_opt(0).expect("offset");
        let at = |d, h, m| {
            u64::try_from(
                tz.with_ymd_and_hms(2026, 10, d, h, m, 0)
                    .single()
                    .expect("time")
                    .timestamp_millis(),
            )
            .expect("after 1970")
        };
        let now = at(14, 12, 0);
        assert_eq!(row_time(at(14, 9, 5), now, &tz), "09:05");
        assert_eq!(row_time(at(13, 9, 5), now, &tz), "Yesterday");
        assert_eq!(row_time(at(2, 9, 5), now, &tz), "2 October");
    }

    #[test]
    fn a_rail_tooltip_names_the_room_and_says_what_the_dots_mean() {
        let call = Some(CallPill {
            count: 2,
            ringing: false,
        });
        assert_eq!(rail_tip("Ops", Badge::None, None, None, &[]), "Ops");
        assert_eq!(
            rail_tip("Ops", Badge::Count(3), None, None, &[]),
            "Ops, 3 unread"
        );
        assert_eq!(
            rail_tip("Ops", Badge::Highlight(1), call, None, &[]),
            "Ops, 1 mention, in a call"
        );
        assert_eq!(rail_tip("Ops", Badge::Dot, None, None, &[]), "Ops, unread");
        assert_eq!(
            rail_tip(
                "Ops",
                Badge::Count(120),
                Some(CallPill {
                    count: 0,
                    ringing: true
                }),
                Some("Muted"),
                &[]
            ),
            "Ops, 99+ unread, ringing, Muted"
        );
    }

    #[test]
    fn a_rail_row_marks_invites_voice_rooms_and_encryption() {
        let mut room = RoomSummary::new(twicecat_core::api::RoomId::from("!r:x"), "r".to_owned());
        assert!(rail_marks(&room).is_empty());
        room.is_invite = true;
        room.is_call_room = true;
        room.is_encrypted = true;
        let marks = rail_marks(&room);
        assert_eq!(
            marks,
            [RailMark::Invite, RailMark::Voice, RailMark::Encrypted]
        );
        assert_eq!(
            rail_tip("Ops", Badge::None, None, Some("Muted"), &marks),
            "Ops, invite, voice room, encrypted, Muted"
        );
        let _: Element<'_, Message> = rail_mark_layer(&marks);
    }

    #[test]
    fn rows_and_rail_rows_build_in_every_density() {
        let mut room = RoomSummary::new(twicecat_core::api::RoomId::from("!r:x"), "r".to_owned());
        room.unread_notifications = 3;
        room.is_call_room = true;
        room.is_invite = true;
        room.is_encrypted = true;
        let avatars = Avatars::default();
        let call = Some(CallPill {
            count: 2,
            ringing: true,
        });
        for density in [Density::Comfortable, Density::Compact] {
            for selected in [false, true] {
                let _: Element<'_, Message> = room_row(
                    &room,
                    &RowEnv {
                        avatars: &avatars,
                        density,
                        now_ms: 0,
                    },
                    selected,
                    Some(RoomNotificationModeView::Mute),
                    call,
                );
            }
        }
        for selected in [false, true] {
            let _: Element<'_, Message> = rail_row(&room, &avatars, selected, None, call);
        }
    }

    // ---- channel rows and category headers (sidebar rework, Task 11) ----

    /// The backgrounds a row's name can sit on: the list's, the hover and the
    /// selection.
    fn row_backgrounds(theme: &iced::Theme) -> [Color; 3] {
        let sem = theme::semantic(theme);
        [theme.palette().background, sem.message_hover, sem.selection]
    }

    #[test]
    fn unread_names_read_at_4_5_and_read_names_and_headers_at_3_in_every_theme() {
        for theme in theme::test_themes() {
            let name = theme.to_string();
            for back in row_backgrounds(&theme) {
                let unread = channel_name_color(&theme, Emphasis::Unread);
                let ratio = theme::contrast(unread, back);
                assert!(ratio >= 4.5, "{name}: an unread name {ratio:.2}");
                for emphasis in [Emphasis::Read, Emphasis::Muted] {
                    let ratio = theme::contrast(channel_name_color(&theme, emphasis), back);
                    assert!(ratio >= 3.0, "{name}: a {emphasis:?} name {ratio:.2}");
                }
                // A read name never outshines an unread one, nor a muted one a
                // read one (Faint's muted colour is brighter than its text).
                let read = theme::contrast(channel_name_color(&theme, Emphasis::Read), back);
                let muted = theme::contrast(channel_name_color(&theme, Emphasis::Muted), back);
                let unread = theme::contrast(unread, back);
                assert!(
                    read <= unread + 0.005,
                    "{name}: read {read:.2} > unread {unread:.2}"
                );
                assert!(
                    muted <= read + 0.005,
                    "{name}: muted {muted:.2} > read {read:.2}"
                );
                let ratio = theme::contrast(header_name_color(&theme, false), back);
                assert!(ratio >= 3.0, "{name}: a header {ratio:.2}");
                let ratio = theme::contrast(header_name_color(&theme, true), back);
                assert!(ratio >= 4.5, "{name}: a hovered header {ratio:.2}");
            }
        }
    }

    #[test]
    fn the_contrast_probe_fails_a_worse_colour() {
        // The same check, on a colour pulled most of the way to the row's
        // background, must fail (so the test above can fail).
        for theme in theme::test_themes() {
            let back = theme.palette().background;
            let worse = |c: Color| {
                Color::from_rgb(
                    c.r + (back.r - c.r) * 0.9,
                    c.g + (back.g - c.g) * 0.9,
                    c.b + (back.b - c.b) * 0.9,
                )
            };
            assert!(
                theme::contrast(worse(channel_name_color(&theme, Emphasis::Unread)), back) < 4.5,
                "{theme}: unread"
            );
            assert!(
                theme::contrast(worse(channel_name_color(&theme, Emphasis::Read)), back) < 3.0,
                "{theme}: read"
            );
            assert!(
                theme::contrast(worse(header_name_color(&theme, false)), back) < 3.0,
                "{theme}: header"
            );
        }
    }

    #[test]
    fn emphasis_differs_in_colour_unread_from_read_and_muted_from_read() {
        for theme in theme::test_themes() {
            let c = |e| channel_name_color(&theme, e);
            assert_ne!(c(Emphasis::Unread), c(Emphasis::Read), "{theme}");
            assert_ne!(c(Emphasis::Muted), c(Emphasis::Read), "{theme}");
            assert_ne!(
                header_name_color(&theme, true),
                header_name_color(&theme, false),
                "{theme}"
            );
        }
    }

    #[test]
    fn the_badge_of_a_channel_row_is_badge_for_of_the_same_room() {
        use RoomNotificationModeView::{AllMessages, Mute};
        let room = |edit: fn(&mut RoomSummary)| {
            let mut r = RoomSummary::new(twicecat_core::api::RoomId::from("!r:x"), "r".to_owned());
            edit(&mut r);
            r
        };
        let rooms = [
            room(|_| {}),
            room(|r| r.unread_messages = 3),
            room(|r| r.unread_notifications = 4),
            room(|r| r.highlights = 2),
            room(|r| r.marked_unread = true),
            room(|r| {
                r.unread_notifications = 4;
                r.highlights = 1;
            }),
            room(|r| {
                r.is_call_room = true;
                r.unread_messages = 1;
            }),
            room(|r| r.is_invite = true),
        ];
        for r in &rooms {
            for mode in [None, Some(AllMessages), Some(Mute)] {
                assert_eq!(channel_marks(r, mode).badge, badge_for(r), "{mode:?}");
            }
        }
        let muted = channel_marks(&rooms[2], Some(Mute));
        assert_eq!(muted.badge, Badge::Count(4), "a muted room keeps its count");
        assert_eq!(muted.muted, Some("Muted"));
        assert!(channel_marks(&rooms[7], None).invite);
        assert_eq!(channel_marks(&rooms[0], None).lock, None);
    }

    #[test]
    fn channel_rows_and_headers_build_in_every_state() {
        let avatars = Avatars::default();
        let mut room = RoomSummary::new(twicecat_core::api::RoomId::from("!r:x"), "r".to_owned());
        for density in [Density::Comfortable, Density::Compact] {
            for (call, dm, unread) in [
                (false, false, false),
                (true, false, true),
                (false, true, true),
            ] {
                room.is_call_room = call;
                room.is_dm = dm;
                room.unread_messages = u64::from(unread);
                for selected in [false, true] {
                    let _: Element<'_, Message> = channel_row(
                        &room,
                        &RowEnv {
                            avatars: &avatars,
                            density,
                            now_ms: 0,
                        },
                        selected,
                        Some(RoomNotificationModeView::Mute),
                        None,
                        ListCue {
                            mark: Mark::None,
                            cursor: mouse::Interaction::Grab,
                            dragging: false,
                            hovered: selected,
                            dragged: false,
                            pressed: false,
                        },
                    );
                }
            }
        }
    }

    #[test]
    fn a_header_that_grows_in_capitals_is_still_within_the_cap_and_the_mark_limit() {
        use crate::text_safety::{MAX_COMBINING_MARKS, MAX_NAME_CHARS};
        let longest_marks = |text: &str| {
            let (mut run, mut most) = (0, 0);
            for c in text.chars() {
                if ('\u{0300}'..='\u{036F}').contains(&c) {
                    run += 1;
                    most = most.max(run);
                } else {
                    run = 0;
                }
            }
            most
        };
        // "ß" becomes "SS": 200 of them are 400 letters in capitals.
        let wide = header_label(&"ß".repeat(200));
        assert!(
            wide.chars().count() <= MAX_NAME_CHARS,
            "{}",
            wide.chars().count()
        );
        // "ΐ" becomes a capital with two marks, on top of four of its own.
        let marks = header_label(&format!("ΐ{}", "\u{0301}".repeat(4)));
        assert!(
            longest_marks(&marks) <= MAX_COMBINING_MARKS,
            "{}",
            longest_marks(&marks)
        );
        assert_eq!(header_label("Channels"), "CHANNELS");
    }

    #[test]
    fn the_speaker_badge_reads_at_3_to_1_on_every_row_background_in_every_theme() {
        for theme in theme::test_themes() {
            let name = theme.to_string();
            let icon = theme.palette().text;
            for lit in [Lit::Plain, Lit::Hovered, Lit::Selected] {
                let fill = badge_fill(&theme, lit);
                let ratio = theme::contrast(icon, fill);
                assert!(ratio >= 3.0, "{name} {lit:?}: the speaker {ratio:.2}");
                // The same check on a colour pulled most of the way to the
                // circle must fail, so the check bites.
                let worse = Color::from_rgb(
                    icon.r + (fill.r - icon.r) * 0.9,
                    icon.g + (fill.g - icon.g) * 0.9,
                    icon.b + (fill.b - icon.b) * 0.9,
                );
                assert!(theme::contrast(worse, fill) < 3.0, "{name} {lit:?}: probe");
                // The ring is told from the fill (a rim, not a second sign:
                // the speaker's contrast above is what carries the meaning,
                // so the ring has no 3:1 requirement).
                assert_ne!(theme::semantic(&theme).border, fill, "{name} {lit:?}");
            }
        }
    }
}
