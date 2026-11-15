//! The spaces rail (plan Q15 Task 13; sidebar plan Task 5, "RailStates"):
//! **Home**, then the top-level spaces and folders in the user's order.
//!
//! Each item is a 72 x 48 slot holding a 44 px icon, a pill at the left
//! edge, an unread badge at the bottom right and a tooltip with the name.
//! One `mouse_area` per item reports hover, right-click and release; a press
//! spy reports the press without taking it, so that a click is "pressed and
//! released on the same item" while the drag belongs to a sweeten column (see
//! `rail_reorder.rs`). There is no button inside (a button would swallow the
//! press, and iced 0.14 gives buttons no keyboard focus anyway). Sub-spaces
//! are not on the rail: they are sections of the room list.
//!
//! A top-level space that has level 1 sub-spaces shows a chevron at the bottom
//! left of its icon (on hover, and while open). Clicking it lists the
//! sub-spaces under the space in a tinted band, 32 px each. They are plain
//! clicks (a slot there takes the press, so the column that reorders never
//! picks one) and are not dragged: their order is the server's. Which spaces
//! are open is kept in memory only. Under **Home** a short line parts it from
//! the spaces, as Discord's rail does.
//!
//! Dragging reorders only: the top-level items are one sweeten column, and
//! each open folder's spaces are another. Making a folder, filling it and
//! emptying it are done in the options card. An open folder and a space whose
//! sub-spaces are shown are not dragged (a tall item would lift a
//! neighbour): they move with the card, or are closed first.
//!
//! The sidebar is always one slot of the main row: an empty space when the
//! user is in no space, so the room list and the timeline keep their place in
//! the widget tree. A space only filters the room list; the sidebar says
//! nothing about who may see or join a room.

use std::time::Duration;

use iced::widget::{
    Column, Stack, column, container, image, mouse_area, row, scrollable, space, text, tooltip,
};
use iced::{Background, Color, Element, Padding, border, mouse};
use sweeten::widget::column::Style as DragStyle;
use twicecat_core::api::{RoomId, SpaceView, UnreadCountsView};

use crate::avatars::{AvatarState, Avatars, NEUTRAL_INITIALS, initials, placeholder_color};
use crate::rail_reorder::RailColumn;
use crate::sidebar::SidebarState;
use crate::sidebar_menu::SidebarMenuMessage;
use crate::spaces::{RailFolder, RailItem, RailKey, SpacesMessage};
use crate::state::{App, Message};
use crate::text_safety::{MAX_NAME_CHARS, name_or_placeholder};
use crate::theme;
use crate::ui::icons::Icon;
use crate::ui::kit;
use crate::ui::press_spy::press_spy;
use crate::ui::tokens;

/// Width of the sidebar, in pixels.
pub const SIDEBAR_WIDTH: f32 = 72.0;

/// Height of an item's slot.
const SLOT_HEIGHT: f32 = 48.0;

/// Side of an item's icon.
const ICON: f32 = 44.0;

/// Width of the pill at the slot's left edge.
const PILL_WIDTH: f32 = 4.0;

/// Height and smallest width of the badge.
const BADGE: f32 = 18.0;

/// Width of the badge's outline, in the colour of the rail.
const BADGE_OUTLINE: f32 = 2.0;

/// Width of the ring around the selected icon.
const RING: f32 = 2.0;

/// Indent of the spaces of an open folder, in pixels.
const NESTED_INDENT: f32 = 4.0;

/// Width and height of the line under **Home**.
const SEPARATOR: (f32, f32) = (32.0, 2.0);

/// The least alpha of the separator's text colour (plan R1: 20 %).
const SEPARATOR_ALPHA_MIN: f32 = 0.20;

/// The least contrast the separator must have against the rail (WCAG
/// non-text contrast). A 20 % wash of the text colour is far below it in every
/// theme, so the alpha is raised, in steps of 5 %, until it holds.
const SEPARATOR_CONTRAST: f32 = 3.0;

/// Height of a sub-space's slot, and the side of its icon.
const SUB_SLOT_HEIGHT: f32 = 40.0;
const SUB_ICON: f32 = 32.0;

/// How far the sub-spaces of a space are indented under it, in pixels.
const SUB_INDENT: f32 = 8.0;

/// Size of the chevron's glyph, and of the round mark that holds it (the
/// click target).
const CHEVRON: f32 = 12.0;
const CHEVRON_MARK: f32 = 16.0;

/// Side of a space's icon in a folder's 2 x 2 preview.
const MINI: f32 = 18.0;

/// How far the pointer must move after the press before a rail item is
/// picked up, in pixels (sweeten picks on "more than" this, so a click that
/// wobbles by 4 px stays a click).
const DEADBAND: f32 = 4.0;

/// How much larger the item under the pointer is drawn while it is dragged.
const DRAG_SCALE: f32 = 1.05;

/// How long the pointer rests on an item before its tooltip shows.
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

/// How an item looks. The strongest of selected, hover and unread wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemState {
    /// Read and quiet: a circle, no pill.
    Default,
    /// Something unread: an 8 px pill.
    Unread,
    /// Under the pointer: a squircle and a 20 px pill.
    Hover,
    /// The space shown in the room list: a squircle, a 32 px pill, a ring.
    Selected,
}

impl ItemState {
    /// The look for an item: selected wins over hover, and hover over unread.
    pub fn of(selected: bool, hover: bool, unread: bool) -> Self {
        if selected {
            Self::Selected
        } else if hover {
            Self::Hover
        } else if unread {
            Self::Unread
        } else {
            Self::Default
        }
    }
}

/// Height of the pill: 0, 8, 20 or 32 px.
pub fn pill_height(state: ItemState) -> f32 {
    match state {
        ItemState::Default => 0.0,
        ItemState::Unread => 8.0,
        ItemState::Hover => 20.0,
        ItemState::Selected => 32.0,
    }
}

/// Corner radius of the icon: a circle (22) idle, a squircle (14) when
/// hovered or selected.
pub fn icon_radius(state: ItemState) -> f32 {
    match state {
        ItemState::Default | ItemState::Unread => ICON / 2.0,
        ItemState::Hover | ItemState::Selected => 14.0,
    }
}

/// Whether the counts call for the pill: an unread room, a notification or a
/// mention.
pub fn is_unread(counts: &UnreadCountsView) -> bool {
    counts.unread || counts.notifications > 0 || counts.highlights > 0
}

/// The count badge of an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailBadge {
    /// The number, or "99+".
    pub text: String,
    /// Mentions: drawn in the highlight colour instead of the count colour.
    pub mention: bool,
}

/// The badge for `counts`: the mentions when there are any, else the
/// notifications; "99+" above 99. Unread alone gets none (the pill says it).
pub fn badge_text(counts: &UnreadCountsView) -> Option<RailBadge> {
    let (count, mention) = if counts.highlights > 0 {
        (counts.highlights, true)
    } else if counts.notifications > 0 {
        (counts.notifications, false)
    } else {
        return None;
    };
    Some(RailBadge {
        text: kit::badge_text(count),
        mention,
    })
}

/// What **Home** counts: the rooms that are in no space.
pub fn home_counts(app: &App) -> UnreadCountsView {
    app.spaces.outside()
}

/// The sidebar, or an empty slot when the user is in no space.
pub fn view(app: &App) -> Element<'_, Message> {
    if app.spaces.is_empty() {
        return space().width(0).into();
    }
    let selected = app.query.space.as_ref();
    let hover = app.rail_hover.as_ref();
    let home = home_counts(app);
    // Home is outside the reorderable column: it cannot be picked up, and
    // nothing can be dropped before it.
    let items: Column<'_, Message> = column![
        slot(Slot {
            kind: Kind::Item,
            chevron: None,
            cue: cue(app, &RailKey::Home),
            key: RailKey::Home,
            icon: round(Icon::Home.glyph().to_owned(), None, ICON / 2.0),
            state: ItemState::of(
                selected.is_none(),
                hover == Some(&RailKey::Home),
                is_unread(&home),
            ),
            counts: home,
            name: "Home".to_owned(),
            width: SIDEBAR_WIDTH,
        }),
        separator(),
        rail_column(app),
    ]
    .spacing(4)
    .padding([6, 0]);
    row![
        container(scrollable(items))
            .width(SIDEBAR_WIDTH)
            .height(iced::Fill)
            .style(kit::sidebar_style),
        kit::vertical_divider(),
    ]
    .height(iced::Fill)
    .into()
}

/// The line under **Home**: 32 x 2 px, centred, in the text colour. It is
/// outside the column that reorders, so it is no part of any drag.
fn separator<'a>() -> Element<'a, Message> {
    container(
        container(space().width(SEPARATOR.0).height(SEPARATOR.1)).style(|t: &iced::Theme| {
            container::Style {
                background: Some(Background::Color(separator_color(t))),
                border: border::rounded(SEPARATOR.1 / 2.0),
                ..container::Style::default()
            }
        }),
    )
    .width(SIDEBAR_WIDTH)
    .center_x(SIDEBAR_WIDTH)
    .into()
}

/// The colour of `top` washed over `base` at `alpha`.
fn wash(top: Color, base: Color, alpha: f32) -> Color {
    Color::from_rgb(
        base.r + (top.r - base.r) * alpha,
        base.g + (top.g - base.g) * alpha,
        base.b + (top.b - base.b) * alpha,
    )
}

/// The separator's alpha in `t`: 20 %, or the least more that gives it
/// [`SEPARATOR_CONTRAST`] against the rail.
fn separator_alpha(t: &iced::Theme) -> f32 {
    let text = t.palette().text;
    let rail = theme::semantic(t).sidebar;
    let mut percent: u8 = 20;
    // `SEPARATOR_ALPHA_MIN` as a whole percent, then 5 % at a time.
    debug_assert!((SEPARATOR_ALPHA_MIN * 100.0 - f32::from(percent)).abs() < 0.01);
    while percent < 100
        && theme::contrast(wash(text, rail, f32::from(percent) / 100.0), rail) < SEPARATOR_CONTRAST
    {
        percent += 5;
    }
    f32::from(percent) / 100.0
}

/// The separator's colour: the text colour at [`separator_alpha`].
fn separator_color(t: &iced::Theme) -> Color {
    Color {
        a: separator_alpha(t),
        ..t.palette().text
    }
}

/// Whether the layout can be changed, so that the rail can be dragged.
fn customizable(app: &App) -> bool {
    matches!(
        app.sidebar.state(),
        SidebarState::Loaded | SidebarState::Full
    )
}

/// The top-level items in one sweeten column. When the app has cancelled a
/// drag (`rail_generation` is odd), the column is wrapped in one more
/// widget: iced then drops sweeten's state, which ends the drag it holds
/// (sweeten has no cancel; plan S3). A `container` would not do, it forwards
/// its content's tag.
fn rail_column(app: &App) -> Element<'_, Message> {
    let mut items = sweeten::widget::Column::new()
        .spacing(4)
        .deadband_zone(DEADBAND)
        .style(drag_style);
    for item in app.spaces.rail_items(app.sidebar.layout()) {
        items = match item {
            RailItem::Space(space) => items.push(space_item(app, space, SIDEBAR_WIDTH, false)),
            RailItem::Folder(folder) => items.push(folder_view(app, &folder)),
        };
    }
    if customizable(app) {
        items = items.on_drag(|event| {
            Message::Spaces(SpacesMessage::RailReorder(RailColumn::Top, event.into()))
        });
    }
    let items: Element<'_, Message> = items.into();
    if app.rail_generation % 2 == 1 {
        column![items].into()
    } else {
        items
    }
}

/// How a dragged item and the ones it moves look: the focus colour marks
/// where it came from, and the selection wash tints what slides aside.
fn drag_style(t: &iced::Theme) -> DragStyle {
    let sem = theme::semantic(t);
    DragStyle {
        scale: DRAG_SCALE,
        moved_item_overlay: sem.selection,
        ghost_border: border::rounded(tokens::LARGE).color(sem.focus).width(RING),
        ghost_background: Background::Color(sem.selection),
    }
}

/// The label of a space for its tooltip: one line, capped, a placeholder
/// when blank.
pub fn space_name(space: &SpaceView) -> String {
    name_or_placeholder(&space.name, MAX_NAME_CHARS)
}

/// The label of a folder: its name, or "Folder".
fn folder_name(folder: &RailFolder<'_>) -> String {
    match folder.name {
        Some(name) => name_or_placeholder(name, MAX_NAME_CHARS),
        None => "Folder".to_owned(),
    }
}

/// What the pointer shows over one slot.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Cue {
    /// The cursor over the slot: `Grab` for what can be dragged, `Grabbing`
    /// during a drag, `Pointer` otherwise.
    cursor: mouse::Interaction,
}

/// The cue of the slot of `key`.
fn cue(app: &App, key: &RailKey) -> Cue {
    let cursor = if app.rail_reorder.is_some() {
        mouse::Interaction::Grabbing
    } else if *key != RailKey::Home && customizable(app) {
        mouse::Interaction::Grab
    } else {
        mouse::Interaction::Pointer
    };
    Cue { cursor }
}

/// What kind of slot it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A top-level item (Home, a space, a folder): 48 px high with a 44 px
    /// icon. It reports the press without taking it, so the column that
    /// reorders can pick it, and a click is its release.
    Item,
    /// A sub-space under its space: 40 px high with a 32 px icon. It takes
    /// the press (a click is the press), so nothing is picked from it.
    Sub,
}

impl Kind {
    /// The side of the icon.
    fn icon(self) -> f32 {
        match self {
            Self::Item => ICON,
            Self::Sub => SUB_ICON,
        }
    }

    /// The height of the slot.
    fn height(self) -> f32 {
        match self {
            Self::Item => SLOT_HEIGHT,
            Self::Sub => SUB_SLOT_HEIGHT,
        }
    }
}

/// The chevron of a space that has sub-spaces.
struct Chevron {
    space: RoomId,
    /// Its sub-spaces are shown.
    open: bool,
    /// The pointer is on the slot.
    hover: bool,
}

/// Everything one slot needs.
struct Slot<'a> {
    kind: Kind,
    cue: Cue,
    key: RailKey,
    icon: Element<'a, Message>,
    state: ItemState,
    counts: UnreadCountsView,
    name: String,
    width: f32,
    chevron: Option<Chevron>,
}

/// A top-level space, or one inside an open folder (`nested`): its slot, and
/// when its tree is open the band of its sub-spaces under it. The two are one
/// item of the column that reorders, so they move together.
fn space_item<'a>(
    app: &'a App,
    parent: &'a SpaceView,
    width: f32,
    nested: bool,
) -> Element<'a, Message> {
    let subs = app.spaces.sub_spaces(&parent.id);
    if subs.is_empty() {
        return space_slot(app, parent, width, None);
    }
    let open = app.rail_expanded.contains(&parent.id);
    let head = space_slot(app, parent, width, Some(open));
    if !open {
        return head;
    }
    let slots = subs
        .into_iter()
        .map(|sub| sub_slot(app, sub, width - SUB_INDENT));
    let band = container(row![
        space().width(SUB_INDENT),
        Column::with_children(slots).spacing(2)
    ])
    .width(width)
    .padding([4, 0])
    .style(move |t: &iced::Theme| tree_band_style(t, nested));
    // An expanded space is not dragged: the top column picks the item whose
    // centre is nearest, and a tall item's centre is far from its head and
    // its padding, so a neighbour could be lifted. The whole item takes the
    // press instead (as an open folder does); it moves from its options
    // card, or is collapsed first.
    mouse_area(column![head, band].spacing(4))
        .on_press(Message::Spaces(SpacesMessage::RailPressed(RailKey::Space(
            parent.id.clone(),
        ))))
        .into()
}

/// How the slot of a top-level `space` looks. `tree` is `None` when the
/// space has no sub-spaces, else whether its tree is open: a closed parent
/// stands for the sub-space that is selected, as a closed folder does for the
/// space it holds.
pub fn space_state(app: &App, space: &SpaceView, tree: Option<bool>) -> ItemState {
    let key = RailKey::Space(space.id.clone());
    let selected = app.query.space.as_ref() == Some(&space.id)
        || (tree == Some(false)
            && app
                .spaces
                .holds_selected(&space.id, app.query.space.as_ref()));
    ItemState::of(
        selected,
        app.rail_hover.as_ref() == Some(&key),
        is_unread(&app.spaces.unread_for(&space.id)),
    )
}

/// `tree`: `None` when the space has no sub-spaces, else whether its tree is
/// open.
fn space_slot<'a>(
    app: &'a App,
    space: &'a SpaceView,
    width: f32,
    tree: Option<bool>,
) -> Element<'a, Message> {
    let key = RailKey::Space(space.id.clone());
    let counts = app.spaces.unread_for(&space.id);
    let hover = app.rail_hover.as_ref() == Some(&key);
    let state = space_state(app, space, tree);
    let mut cue = cue(app, &key);
    if tree == Some(true) && app.rail_reorder.is_none() {
        // An expanded space cannot be picked up (see `space_item`).
        cue.cursor = mouse::Interaction::Pointer;
    }
    slot(Slot {
        kind: Kind::Item,
        cue,
        key,
        icon: space_icon(space, ICON, icon_radius(state), &app.avatars),
        state,
        counts,
        name: space_name(space),
        width,
        chevron: tree.map(|open| Chevron {
            space: space.id.clone(),
            open,
            hover,
        }),
    })
}

/// A sub-space under its space: selected by a click.
fn sub_slot<'a>(app: &'a App, sub: &'a SpaceView, width: f32) -> Element<'a, Message> {
    let key = RailKey::Space(sub.id.clone());
    let counts = app.spaces.unread_for(&sub.id);
    let state = ItemState::of(
        app.query.space.as_ref() == Some(&sub.id),
        app.rail_hover.as_ref() == Some(&key),
        is_unread(&counts),
    );
    slot(Slot {
        kind: Kind::Sub,
        cue: Cue {
            cursor: mouse::Interaction::Pointer,
        },
        key,
        icon: space_icon(sub, SUB_ICON, sub_icon_radius(state), &app.avatars),
        state,
        counts,
        name: space_name(sub),
        width,
        chevron: None,
    })
}

/// Corner radius of a sub-space's icon: a circle idle, a squircle when hovered
/// or selected (as [`icon_radius`], for the smaller icon).
fn sub_icon_radius(state: ItemState) -> f32 {
    match state {
        ItemState::Default | ItemState::Unread => SUB_ICON / 2.0,
        ItemState::Hover | ItemState::Selected => 10.0,
    }
}

/// The band behind the sub-spaces of a space: the open folder's tint, one
/// step up inside a folder (so that it parts from the folder's own band).
fn tree_band_style(t: &iced::Theme, nested: bool) -> container::Style {
    let sem = theme::semantic(t);
    container::Style {
        background: Some(Background::Color(if nested {
            sem.surface_raised
        } else {
            sem.surface
        })),
        border: border::rounded(tokens::LARGE),
        ..container::Style::default()
    }
}

/// A folder: its icon, and when it is open its spaces in a tinted band.
fn folder_view<'a>(app: &'a App, folder: &RailFolder<'a>) -> Element<'a, Message> {
    let key = RailKey::Folder(folder.id);
    let counts = app.spaces.folder_counts(folder);
    // A closed folder holding the selected space stands for it.
    let holds_selected = app.query.space.as_ref().is_some_and(|sel| {
        folder
            .spaces
            .iter()
            .any(|space| space.id == *sel || app.spaces.holds_selected(&space.id, Some(sel)))
    });
    let state = ItemState::of(
        !folder.open && holds_selected,
        app.rail_hover.as_ref() == Some(&key),
        is_unread(&counts),
    );
    let head = slot(Slot {
        kind: Kind::Item,
        chevron: None,
        cue: cue(app, &key),
        key,
        icon: folder_icon(folder, icon_radius(state), &app.avatars),
        state,
        counts,
        name: folder_name(folder),
        width: SIDEBAR_WIDTH,
    });
    if !folder.open {
        return head;
    }
    // The spaces of an open folder reorder in a column of their own: a press
    // on one is taken by it, so the top-level column never picks from here.
    let id = folder.id;
    let mut inside = sweeten::widget::Column::new()
        .spacing(4)
        .deadband_zone(DEADBAND)
        .style(drag_style);
    for space in &folder.spaces {
        inside = inside.push(space_item(app, space, SIDEBAR_WIDTH - NESTED_INDENT, true));
    }
    if customizable(app) {
        inside = inside.on_drag(move |event| {
            Message::Spaces(SpacesMessage::RailReorder(
                RailColumn::Folder(id),
                event.into(),
            ))
        });
    }
    // An open folder is not dragged by its head: the top-level column picks
    // the item whose centre is nearest, and the centre of a tall open folder
    // is far from its head, so a neighbour could be picked. The whole item
    // takes the press instead; the folder moves from its options card.
    mouse_area(
        column![
            head,
            container(row![space().width(NESTED_INDENT), inside])
                .width(SIDEBAR_WIDTH)
                .padding([4, 0])
                .style(band_style),
        ]
        .spacing(4),
    )
    .on_press(Message::Spaces(SpacesMessage::RailPressed(
        RailKey::Folder(id),
    )))
    .into()
}

/// The tinted band behind the spaces of an open folder.
fn band_style(t: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme::semantic(t).surface)),
        border: border::rounded(tokens::LARGE),
        ..container::Style::default()
    }
}

/// One item: the icon, with the pill under the left edge and the badge over
/// the bottom right, in a `mouse_area` and a tooltip.
fn slot(slot: Slot<'_>) -> Element<'_, Message> {
    let Slot {
        kind,
        cue,
        key,
        icon,
        state,
        counts,
        name,
        width,
        chevron,
    } = slot;
    let (icon_side, height) = (kind.icon(), kind.height());
    let radius = match kind {
        Kind::Item => icon_radius(state),
        Kind::Sub => sub_icon_radius(state),
    };
    let icon = if state == ItemState::Selected {
        iced::widget::stack![icon, ring(icon_side, radius)].into()
    } else {
        icon
    };
    let mut layers: Stack<'_, Message> = Stack::new()
        .width(width)
        .height(height)
        .push(container(icon).center_x(width).center_y(height));
    let pill = pill_height(state);
    if pill > 0.0 {
        layers = layers.push(
            container(container(space().width(PILL_WIDTH).height(pill)).style(pill_style))
                .height(height)
                .center_y(height),
        );
    }
    if let Some(badge) = badge_text(&counts) {
        // The badge sits 4 px beyond the icon's right edge.
        layers = layers.push(
            container(badge_view(badge))
                .align_right(width)
                .align_bottom(height)
                .padding(Padding {
                    right: (width - icon_side) / 2.0 - 4.0,
                    ..Padding::ZERO
                }),
        );
    }
    // The chevron is the topmost layer and takes its own press, so a click on
    // it neither picks the item up nor selects it.
    if let Some(chevron) = chevron.filter(|c| c.open || c.hover) {
        layers = layers.push(chevron_view(chevron, width, height));
    }
    let mut area = mouse_area(layers)
        .interaction(cue.cursor)
        .on_enter(Message::Spaces(SpacesMessage::Enter(key.clone())))
        .on_exit(Message::Spaces(SpacesMessage::Exit(key.clone())));
    let reported: Element<'_, Message> = match (kind, &key) {
        (Kind::Sub, RailKey::Space(id)) => {
            // A click is the press: this takes it, so the column that
            // reorders the rail never picks from a sub-space.
            let select = Message::Spaces(SpacesMessage::Select(Some(id.clone())));
            area.on_press(select).into()
        }
        _ => {
            area = area.on_release(Message::Spaces(SpacesMessage::Release(key.clone())));
            let pressed = Message::Spaces(SpacesMessage::RailPressed(key.clone()));
            // A right-click opens the item's options card (Home has none).
            if key != RailKey::Home {
                area = area.on_right_press(Message::SidebarMenu(SidebarMenuMessage::Open(key)));
            }
            press_spy(area, pressed)
        }
    };
    tooltip(
        reported,
        container(text(name).size(tokens::CAPTION))
            .padding(tokens::S)
            .style(kit::card_style),
        tooltip::Position::Right,
    )
    .delay(TOOLTIP_DELAY)
    .into()
}

/// The chevron of a slot: a small round mark at the icon's bottom left that
/// shows or hides the sub-spaces.
fn chevron_view<'a>(chevron: Chevron, width: f32, height: f32) -> Element<'a, Message> {
    let glyph = if chevron.open {
        Icon::SectionOpen
    } else {
        Icon::SectionClosed
    };
    let mark = container(text(glyph.glyph()).size(CHEVRON))
        .width(CHEVRON_MARK)
        .height(CHEVRON_MARK)
        .center_x(CHEVRON_MARK)
        .center_y(CHEVRON_MARK)
        .style(|t: &iced::Theme| container::Style {
            background: Some(Background::Color(theme::semantic(t).sidebar)),
            text_color: Some(t.palette().text),
            border: border::rounded(CHEVRON_MARK / 2.0),
            ..container::Style::default()
        });
    let press = mouse_area(mark)
        .interaction(mouse::Interaction::Pointer)
        .on_press(Message::Spaces(SpacesMessage::ToggleTree(chevron.space)));
    container(press)
        .width(width)
        .height(height)
        .align_x(iced::alignment::Horizontal::Left)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(Padding {
            left: (width - ICON) / 2.0 - 4.0,
            bottom: 2.0,
            ..Padding::ZERO
        })
        .into()
}

/// The pill: the text colour, round on its right side only.
fn pill_style(t: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(t.palette().text)),
        border: iced::Border {
            radius: border::right(PILL_WIDTH),
            ..iced::Border::default()
        },
        ..container::Style::default()
    }
}

/// The ring of the selected icon: the focus colour, inside the icon's box.
fn ring<'a>(side: f32, radius: f32) -> Element<'a, Message> {
    container(space())
        .width(side)
        .height(side)
        .style(move |t| container::Style {
            border: border::rounded(radius)
                .color(theme::semantic(t).focus)
                .width(RING),
            ..container::Style::default()
        })
        .into()
}

/// The count badge: highlight colours for mentions, the count colours
/// otherwise, outlined in the rail's colour so it parts from the icon.
fn badge_view<'a>(badge: RailBadge) -> Element<'a, Message> {
    let mention = badge.mention;
    let single = badge.text.chars().count() <= 1;
    let label = container(text(badge.text).size(tokens::CAPTION))
        .height(BADGE)
        .center_y(BADGE)
        .style(move |t| {
            let sem = theme::semantic(t);
            let (fill, ink) = if mention {
                (sem.highlight, sem.highlight_text)
            } else {
                (sem.badge, sem.badge_text)
            };
            container::Style {
                background: Some(Background::Color(fill)),
                text_color: Some(ink),
                border: border::rounded(BADGE / 2.0)
                    .color(sem.sidebar)
                    .width(BADGE_OUTLINE),
                ..container::Style::default()
            }
        });
    if single {
        label
            .width(BADGE)
            .align_x(iced::alignment::Horizontal::Center)
            .into()
    } else {
        label.padding([0.0, 5.0]).into()
    }
}

/// The avatar of `space` at `size`, or its initials on a colour.
fn space_icon<'a>(
    space: &SpaceView,
    size: f32,
    radius: f32,
    avatars: &Avatars,
) -> Element<'a, Message> {
    let ready = space
        .avatar
        .as_ref()
        .and_then(|id| avatars.state(id))
        .and_then(|state| match state {
            AvatarState::Ready(handle) => Some(handle),
            AvatarState::Unavailable => None,
        });
    match ready {
        Some(handle) => image(handle.clone())
            .width(size)
            .height(size)
            .border_radius(radius)
            .into(),
        None => {
            let name = space_name(space);
            let label = initials(&name);
            let fill = (label != NEUTRAL_INITIALS).then(|| placeholder_color(&space.id));
            if size <= MINI {
                // A small preview carries one letter only.
                let first: String = label.chars().take(1).collect();
                mini_round(first, fill, size, radius)
            } else if size < ICON {
                sized_round(label, fill, size, radius, 12)
            } else {
                round(label, fill, radius)
            }
        }
    }
}

/// The round (or squircle) placeholder: `label` on `fill` (the muted colour
/// when `None`).
fn round<'a>(label: String, fill: Option<Color>, radius: f32) -> Element<'a, Message> {
    sized_round(label, fill, ICON, radius, 15)
}

fn mini_round<'a>(
    label: String,
    fill: Option<Color>,
    size: f32,
    radius: f32,
) -> Element<'a, Message> {
    sized_round(label, fill, size, radius, 10)
}

fn sized_round<'a>(
    label: String,
    fill: Option<Color>,
    size: f32,
    radius: f32,
    font: u32,
) -> Element<'a, Message> {
    container(text(label).size(font).color(Color::WHITE))
        .width(size)
        .height(size)
        .center_x(size)
        .center_y(size)
        .style(move |t| container::Style {
            background: Some(Background::Color(
                fill.unwrap_or_else(|| theme::semantic(t).muted),
            )),
            border: border::rounded(radius),
            ..container::Style::default()
        })
        .into()
}

/// A folder's icon: a squircle with a 2 x 2 preview of its first four
/// spaces.
fn folder_icon<'a>(
    folder: &RailFolder<'a>,
    radius: f32,
    avatars: &Avatars,
) -> Element<'a, Message> {
    let cell = |at: usize| -> Element<'a, Message> {
        match folder.spaces.get(at) {
            Some(space) => space_icon(space, MINI, 5.0, avatars),
            None => space().width(MINI).height(MINI).into(),
        }
    };
    container(
        column![
            row![cell(0), cell(1)].spacing(2),
            row![cell(2), cell(3)].spacing(2)
        ]
        .spacing(2),
    )
    .width(ICON)
    .height(ICON)
    .center_x(ICON)
    .center_y(ICON)
    .style(move |t| container::Style {
        background: Some(Background::Color(theme::semantic(t).surface_raised)),
        border: border::rounded(radius),
        ..container::Style::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use twicecat_core::api::RoomId;

    use super::*;

    fn counts(notifications: u64, highlights: u64, unread: bool) -> UnreadCountsView {
        UnreadCountsView {
            notifications,
            highlights,
            unread,
        }
    }

    #[test]
    fn the_name_is_one_capped_line_with_a_placeholder() {
        let mut space = SpaceView {
            id: RoomId::from("!s:x"),
            name: "  ".into(),
            avatar: None,
            level: 0,
            children: 0,
            rooms: Vec::new(),
        };
        assert_eq!(space_name(&space), crate::text_safety::NAME_PLACEHOLDER);
        space.name = format!("a\nb{}", "z".repeat(1000));
        let name = space_name(&space);
        assert!(!name.contains('\n') && name.chars().count() <= MAX_NAME_CHARS);
    }

    #[test]
    fn the_pill_is_0_8_20_or_32_and_the_strongest_state_wins() {
        let h = |selected, hover, unread| pill_height(ItemState::of(selected, hover, unread));
        assert_eq!(h(false, false, false), 0.0);
        assert_eq!(h(false, false, true), 8.0);
        assert_eq!(h(false, true, false), 20.0);
        assert_eq!(h(true, false, false), 32.0);
        // Hover wins over unread; selected wins over hover.
        assert_eq!(h(false, true, true), 20.0);
        assert_eq!(h(true, true, false), 32.0);
        assert_eq!(h(true, true, true), 32.0);
        assert_eq!(h(true, false, true), 32.0);
    }

    #[test]
    fn the_icon_is_a_circle_idle_and_a_squircle_when_hovered_or_selected() {
        assert_eq!(icon_radius(ItemState::Default), 22.0);
        assert_eq!(icon_radius(ItemState::Unread), 22.0);
        assert_eq!(icon_radius(ItemState::Hover), 14.0);
        assert_eq!(icon_radius(ItemState::Selected), 14.0);
        // The radius of the idle circle is half the icon.
        assert_eq!(icon_radius(ItemState::Default) * 2.0, ICON);
    }

    #[test]
    fn the_badge_shows_mentions_first_then_notifications_and_never_unread_alone() {
        let b = |c| badge_text(&c);
        assert_eq!(
            b(counts(9, 2, true)),
            Some(RailBadge {
                text: "2".into(),
                mention: true
            })
        );
        assert_eq!(
            b(counts(9, 0, true)),
            Some(RailBadge {
                text: "9".into(),
                mention: false
            })
        );
        assert_eq!(b(counts(0, 0, true)), None, "the pill says plain unread");
        assert_eq!(b(counts(0, 0, false)), None);
    }

    #[test]
    fn the_badge_caps_at_99_plus() {
        let text = |n, h| badge_text(&counts(n, h, true)).expect("a badge").text;
        assert_eq!(text(99, 0), "99");
        assert_eq!(text(100, 0), "99+");
        assert_eq!(text(u64::MAX, 0), "99+");
        assert_eq!(text(0, 100), "99+");
        assert_eq!(
            text(500, 3),
            "3",
            "mentions first, whatever the notifications"
        );
    }

    #[test]
    fn the_pill_follows_any_sign_of_unread() {
        assert!(!is_unread(&counts(0, 0, false)));
        assert!(is_unread(&counts(0, 0, true)));
        assert!(is_unread(&counts(1, 0, false)));
        assert!(is_unread(&counts(0, 1, false)));
    }

    #[test]
    fn the_separator_is_at_least_3_to_1_against_the_rail_in_every_test_theme() {
        for t in theme::test_themes() {
            let rail = theme::semantic(&t).sidebar;
            let drawn = separator_color(&t);
            let alpha = separator_alpha(&t);
            assert_eq!(drawn.a, alpha);
            assert!(alpha >= SEPARATOR_ALPHA_MIN, "never fainter than 20 %");
            // What the renderer paints: the colour at that alpha over the rail.
            let seen = wash(drawn, rail, drawn.a);
            let ratio = theme::contrast(seen, rail);
            assert!(
                ratio >= 3.0,
                "separator on the rail: {ratio} (alpha {alpha})"
            );
            // The least alpha that does it: one step less would not.
            if alpha > SEPARATOR_ALPHA_MIN + 0.001 {
                let weaker = wash(drawn, rail, alpha - 0.05);
                assert!(
                    theme::contrast(weaker, rail) < 3.0,
                    "{alpha} is more than needed"
                );
            }
        }
    }

    #[test]
    fn the_rail_marks_read_on_the_rail_background_in_every_test_theme() {
        for t in theme::test_themes() {
            let p = t.palette();
            let s = theme::semantic(&t);
            let d = |a, b| theme::delta_e76(a, b);
            // The badges, the pill and the ring part from the rail.
            for (what, c) in [
                ("badge", s.badge),
                ("mention badge", s.highlight),
                ("pill", p.text),
                ("ring", s.focus),
            ] {
                assert!(
                    d(c, s.sidebar) >= 20.0,
                    "{what} on the rail: {}",
                    d(c, s.sidebar)
                );
            }
            // The label is readable on both badges.
            for fill in [s.badge, s.highlight] {
                assert!(theme::contrast(s.badge_text, fill) >= 4.5);
                assert!(theme::contrast(s.highlight_text, fill) >= 4.5);
            }
            // The folder tile and the band can be told from the rail.
            assert!(d(s.surface_raised, s.sidebar) >= 2.0, "folder tile");
            assert!(d(s.surface, s.sidebar) >= 2.0, "band");
        }
    }
}
