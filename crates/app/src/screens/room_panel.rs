//! The room panel (UI review fixes, Task 3): a column beside the room pane
//! with a tab strip (Members, Info, and Thread while a thread is open). The
//! Thread tab holds the thread's timeline, a text field and **Send**
//! (plan Q16, Task 13); Members and Info arrive in Tasks 4 and 5.
//!
//! The panel is always one slot of the main row (an empty space when closed)
//! and the thread view always has three children (header, timeline,
//! composer), so opening or closing it never resets the room timeline's
//! scroll or the room composer's focus.

use chrono::Local;
use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, button, column, container, row, space, text, text_input};
use iced::{Element, Fill, Length};
use twicecat_core::api::MessageText;

use crate::layout::PaneLayout;
use crate::room_panel::{RoomPanelMessage, RoomPanelTab};
use crate::screens::main_view::{self, rich_env};
use crate::screens::timeline_pane::{self, Pane};
use crate::state::{App, Message};
use crate::threads::{PANEL_WIDTH, THREAD_INPUT_ID, ThreadPanel, ThreadsMessage};
use crate::ui::icons::Icon;
use crate::ui::kit;
use crate::ui::tokens;

/// The panel, or an empty slot when it is closed.
pub fn slot(app: &App, layout: PaneLayout) -> Element<'_, Message> {
    if layout.room_panel && app.room_panel_shown() {
        view(app, layout.panel_fills)
    } else {
        space().width(0).into()
    }
}

/// The tab strip and the close button. With `fills` (a narrow window) the
/// header also carries the room's name and lock and the trust tag of a direct
/// message, since the room header is out of sight.
fn header<'a>(app: &'a App, fills: bool) -> Element<'a, Message> {
    let mut head: Vec<Element<'a, Message>> = Vec::new();
    if fills {
        head.push(main_view::panel_title(
            app,
            text(tab_label(app.room_panel.tab)).size(tokens::TITLE),
            true,
        ));
        if let Some(trust) = main_view::trust_strip(app) {
            head.push(trust);
        }
    }
    head.push(tab_strip(app));
    head.push(space().width(Fill).into());
    head.push(kit::icon_button(
        Icon::Close,
        Message::RoomPanel(RoomPanelMessage::Close),
    ));
    Row::with_children(head)
        .spacing(tokens::S)
        .align_y(iced::Center)
        .into()
}

/// The tabs: Members, Info, and Thread only while a thread is open.
fn tab_strip(app: &App) -> Element<'_, Message> {
    let thread = app.threads.panel.is_some().then_some(RoomPanelTab::Thread);
    let tabs = [RoomPanelTab::Members, RoomPanelTab::Info]
        .into_iter()
        .chain(thread);
    row(tabs.map(|tab| {
        button(
            text(tab_label(tab))
                .size(tokens::CAPTION)
                .wrapping(Wrapping::None),
        )
        .on_press(Message::RoomPanel(RoomPanelMessage::Tab(tab)))
        .padding([tokens::XS, tokens::S])
        .style(kit::filter_chip_style(tab == app.room_panel.tab))
        .into()
    }))
    .spacing(tokens::XS)
    .into()
}

fn tab_label(tab: RoomPanelTab) -> &'static str {
    match tab {
        RoomPanelTab::Members => "Members",
        RoomPanelTab::Info => "Info",
        RoomPanelTab::Thread => "Thread",
    }
}

/// The panel. With `fills` it takes the room pane's width (a narrow window).
fn view(app: &App, fills: bool) -> Element<'_, Message> {
    let content: Element<'_, Message> = match (app.room_panel.tab, &app.threads.panel) {
        (RoomPanelTab::Thread, Some(panel)) => thread_body(app, panel),
        // Members and Info are filled in by Tasks 4 and 5.
        (tab, _) => text(tab_label(tab)).size(tokens::BODY).into(),
    };
    let body: Column<'_, Message> = Column::from_vec(vec![header(app, fills), content])
        .spacing(8)
        .width(Fill)
        .height(Fill);
    row![
        kit::vertical_divider(),
        container(body)
            .padding(tokens::S)
            .width(Fill)
            .height(Fill)
            .style(kit::surface_style)
    ]
    .width(if fills {
        Length::Fill
    } else {
        Length::Fixed(PANEL_WIDTH)
    })
    .height(Fill)
    .into()
}

/// The thread's timeline and its composer.
fn thread_body<'a>(app: &'a App, panel: &'a ThreadPanel) -> Element<'a, Message> {
    let theme = app.current_theme();
    let now = Local::now().timestamp_millis();
    let timeline = container(main_view::timeline_layer(
        timeline_pane::view_pane(
            &panel.timeline,
            u64::try_from(now).unwrap_or(0),
            &Local,
            rich_env(app, &theme),
            Pane::THREAD,
        ),
        main_view::picker_layer(app, true),
    ))
    .height(Fill);
    Column::from_vec(vec![timeline.into(), composer(panel)])
        .spacing(8)
        .width(Fill)
        .height(Fill)
        .into()
}

/// The text field with **Attach...** and **Send**. Enter sends; the field
/// holds one line (the room's composer is the place for long text).
fn composer(panel: &ThreadPanel) -> Element<'_, Message> {
    let attach = kit::secondary_button(
        "Attach\u{2026}",
        Some(Message::Threads(ThreadsMessage::Attach)),
    );
    let field = text_input("Reply in thread", &panel.draft)
        .style(kit::text_input_style)
        .id(THREAD_INPUT_ID)
        .on_input(|text| Message::Threads(ThreadsMessage::Draft(MessageText::from(text))))
        .on_submit(Message::Threads(ThreadsMessage::Submit));
    let send = kit::primary_button(
        "Send",
        (!panel.draft.trim().is_empty()).then_some(Message::Threads(ThreadsMessage::Submit)),
    );
    column![row![field, send].spacing(6), attach]
        .spacing(6)
        .into()
}
