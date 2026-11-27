//! The Info tab of the room panel (plan Q13, Task 12; restyled in UI/UX
//! rework Task 15; moved from a modal into the room panel in UI review fixes
//! Task 5). From the top: the room's picture and name, its topic in full, its
//! address, whether it is encrypted (in words), how many members it has and
//! its notification menu, all read-only. Below them are the fields the room's
//! power levels allow (name, topic, picture), then turning encryption on and
//! leaving. Each control works only when `RoomDetails.permissions` allows it.
//!
//! The two questions that cannot be undone (turn encryption on, leave) keep
//! their confirmation: the button that asks sits at the left, in a different
//! place from the one that confirms, which is red, has words, and sits at the
//! right with at least [`tokens::M`] between it and **Cancel**. Esc cancels the
//! question first and only then closes the panel.

use iced::widget::{Column, column, container, row, scrollable, text_input};
use iced::{Element, Fill};
use twicecat_core::api::{MessageText, RoomEncryption};

use crate::room_list::row_name;
use crate::rooms::{RoomsMessage, SettingsPanel, room_encryption};
use crate::screens::main_view::notification_menu;
use crate::screens::room_dialogs::{error_line, msg};
use crate::screens::room_list_pane::avatar_view;
use crate::state::{App, Message};
use crate::text_safety::multi_line_capped;
use crate::ui::kit::{
    self, danger_button, dialog_footer, divider, empty_state, field, muted, secondary_button, words,
};
use crate::ui::tokens;
use crate::verification::elide_middle;

/// Longest address shown, in characters; a longer one is cut in the middle,
/// so its server name stays.
const ADDRESS_CHARS: usize = 128;
/// Longest topic shown, in characters (Matrix allows far more than a page).
const TOPIC_CHARS: usize = 2000;
/// Most lines of the topic shown.
const TOPIC_LINES: usize = 40;
/// Side of the room's picture at the top of the tab.
const PICTURE: f32 = 56.0;
/// The note under the picture button of an encrypted room.
pub const AVATAR_NOTE: &str =
    "Room pictures are stored unencrypted on the server, even in an encrypted room.";
/// The warning before turning encryption on.
pub const ENCRYPT_WARNING: &str = "Turning on encryption cannot be undone. Messages sent \
    before stay readable by the server, and bots or bridges that cannot use encryption may \
    stop working in this room.";

/// The Info state of the selected room, only while it is that room's: a panel
/// left over for another room is never drawn.
pub fn shown(app: &App) -> Option<&SettingsPanel> {
    app.rooms_ui
        .settings
        .as_ref()
        .filter(|panel| app.selected.as_ref() == Some(&panel.room))
}

/// The Info tab of the room panel.
pub fn view(app: &App) -> Element<'_, Message> {
    match shown(app) {
        Some(panel) => card(app, panel),
        None => empty_state(
            "Loading\u{2026}",
            "Asking the server for the room's details.",
        ),
    }
}

/// The name of the panel's room as the room list has it (cleaned), or its
/// ID, cut in the middle.
fn room_title(app: &App, panel: &SettingsPanel) -> String {
    app.rooms
        .iter()
        .find(|room| room.id == panel.room)
        .map_or_else(
            || crate::verification::elide_middle(panel.room.as_str(), 64),
            |room| row_name(&room.name),
        )
}

/// The Info tab's line on encryption. Only a room known to be unencrypted
/// gets the warning; an unknown one gets no claim either way.
fn encryption_words(encryption: RoomEncryption) -> &'static str {
    match encryption {
        RoomEncryption::Encrypted => {
            "Encrypted: messages can only be read by the people in the room."
        }
        RoomEncryption::NotEncrypted => "Not encrypted: the server can read messages in this room.",
        RoomEncryption::Unknown => "Encryption not known yet",
    }
}

/// The picture and the name of the room, then the read-only facts.
fn summary<'a>(app: &'a App, panel: &'a SettingsPanel) -> Element<'a, Message> {
    let details = app
        .rooms_ui
        .details
        .as_ref()
        .filter(|d| d.room_id == panel.room);
    let name = room_title(app, panel);
    let picture: Element<'a, Message> = match app.rooms.iter().find(|r| r.id == panel.room) {
        Some(room) => avatar_view(room, &name, &app.avatars, PICTURE),
        None => iced::widget::space().width(PICTURE).into(),
    };
    let mut facts = Column::new().spacing(tokens::XS).width(Fill);
    if let Some(topic) = details.and_then(|d| d.topic.as_deref()) {
        facts = facts
            .push(words(multi_line_capped(topic, TOPIC_CHARS, TOPIC_LINES)).size(tokens::BODY));
    }
    if let Some(alias) = details.and_then(|d| d.alias.as_deref()) {
        facts = facts.push(
            words(format!("Address: {}", elide_middle(alias, ADDRESS_CHARS)))
                .size(tokens::BODY)
                .style(muted),
        );
    }
    let encryption = encryption_words(room_encryption(&app.rooms, &panel.room));
    facts = facts.push(words(encryption).size(tokens::BODY).style(muted));
    let members = match app.rooms_ui.known.as_ref().filter(|k| k.room == panel.room) {
        Some(known) if known.truncated => format!("Members: {}+", known.members.len()),
        Some(known) => format!("Members: {}", known.members.len()),
        None => "Members: counting\u{2026}".to_owned(),
    };
    facts = facts.push(words(members).size(tokens::BODY).style(muted));
    column![
        row![picture, words(name).size(tokens::HEADLINE).width(Fill)]
            .spacing(tokens::M)
            .align_y(iced::Center),
        facts,
        notification_menu(app, &panel.room),
    ]
    .spacing(tokens::M)
    .into()
}

fn card<'a>(app: &'a App, panel: &'a SettingsPanel) -> Element<'a, Message> {
    let details = app
        .rooms_ui
        .details
        .as_ref()
        .filter(|d| d.room_id == panel.room);
    let perms = details.map(|d| d.permissions).unwrap_or_default();

    let save_row = |label: &'static str,
                    value: &str,
                    allowed: bool,
                    input: fn(MessageText) -> RoomsMessage,
                    save: RoomsMessage| {
        field(
            label,
            row![
                text_input(label, value)
                    .style(crate::ui::kit::text_input_style)
                    .on_input_maybe(
                        allowed.then_some(move |s: String| msg(input(MessageText::from(s)))),
                    )
                    .on_submit_maybe(allowed.then(|| msg(save.clone())))
                    .padding(tokens::S),
                secondary_button("Save", allowed.then(|| msg(save))),
            ]
            .spacing(tokens::S)
            .align_y(iced::Center),
        )
    };

    let mut body = Column::new().spacing(tokens::M).width(Fill);
    body = body.push(summary(app, panel));
    body = body.push(divider());
    body = body.push(slot(
        details
            .is_none()
            .then(|| words("Loading\u{2026}").style(muted).into()),
    ));
    body = body
        .push(save_row(
            "Name",
            &panel.name,
            perms.can_set_name,
            RoomsMessage::NameInput,
            RoomsMessage::SaveName,
        ))
        .push(save_row(
            "Topic",
            &panel.topic,
            perms.can_set_topic,
            RoomsMessage::TopicInput,
            RoomsMessage::SaveTopic,
        ))
        .push(secondary_button(
            if panel.avatar_busy {
                "Working\u{2026}"
            } else {
                "Change picture\u{2026}"
            },
            (perms.can_set_avatar && !panel.avatar_busy).then(|| msg(RoomsMessage::ChooseAvatar)),
        ));
    body =
        body.push(slot(details.is_some_and(|d| d.is_encrypted).then(|| {
            words(AVATAR_NOTE).size(tokens::CAPTION).style(muted).into()
        })));
    body = body.push(encryption_section(
        panel,
        room_encryption(&app.rooms, &panel.room),
        perms.can_enable_encryption,
    ));
    body = body.push(error_line(panel.error));
    body = body.push(divider());
    body = body.push(leave_section(app, panel));
    // The scrollbar sits beside the content, not over it.
    scrollable(container(body).padding(iced::Padding::ZERO.right(tokens::M)))
        .height(Fill)
        .into()
}

/// `part`, or an empty slot: a column keeps its widget types as parts come
/// and go.
fn slot<'a>(part: Option<Element<'a, Message>>) -> Element<'a, Message> {
    part.unwrap_or_else(|| iced::widget::space().into())
}

/// A question that cannot be undone: a titled card with its words, and
/// **Cancel** then the red action, `tokens::M` apart.
fn question<'a>(
    title: &'static str,
    lines: Vec<String>,
    cancel: RoomsMessage,
    confirm_label: &'static str,
    confirm: RoomsMessage,
) -> Element<'a, Message> {
    let mut body = Column::new()
        .spacing(tokens::S)
        .width(Fill)
        .push(words(title).size(tokens::TITLE));
    for line in lines {
        body = body.push(words(line).size(tokens::BODY));
    }
    body = body.push(dialog_footer(vec![
        secondary_button("Cancel", Some(msg(cancel))).into(),
        danger_button(confirm_label, Some(msg(confirm))).into(),
    ]));
    container(body)
        .padding(tokens::M)
        .width(Fill)
        .style(kit::card_style)
        .into()
}

fn encryption_section(
    panel: &SettingsPanel,
    encryption: RoomEncryption,
    allowed: bool,
) -> Element<'_, Message> {
    match encryption {
        RoomEncryption::Encrypted => words("This room is encrypted. This cannot be turned off.")
            .size(tokens::BODY)
            .style(muted)
            .into(),
        RoomEncryption::Unknown => iced::widget::space().into(),
        RoomEncryption::NotEncrypted if panel.encrypt_ask => question(
            "Turn on encryption?",
            vec![ENCRYPT_WARNING.to_owned()],
            RoomsMessage::EncryptCancel,
            "Enable encryption",
            RoomsMessage::EncryptConfirm,
        ),
        RoomEncryption::NotEncrypted => row![secondary_button(
            "Enable encryption\u{2026}",
            allowed.then(|| msg(RoomsMessage::EncryptAsk)),
        )]
        .into(),
    }
}

fn leave_section<'a>(app: &'a App, panel: &SettingsPanel) -> Element<'a, Message> {
    if !panel.leave_ask {
        // The question's button is at the left; the red **Leave** that
        // confirms comes up at the right, so one click never does both.
        return row![danger_button(
            "Leave room\u{2026}",
            Some(msg(RoomsMessage::LeaveAsk))
        )]
        .into();
    }
    question(
        "Leave this room?",
        app.leave_warning_now()
            .lines()
            .into_iter()
            .map(str::to_owned)
            .collect(),
        RoomsMessage::LeaveCancel,
        "Leave",
        RoomsMessage::LeaveConfirm,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_warnings_say_what_cannot_be_undone() {
        assert!(ENCRYPT_WARNING.contains("cannot be undone"));
        assert!(AVATAR_NOTE.contains("unencrypted"));
    }

    #[test]
    fn the_info_tab_words_each_of_the_three_states() {
        assert!(encryption_words(RoomEncryption::Encrypted).starts_with("Encrypted:"));
        assert!(encryption_words(RoomEncryption::NotEncrypted).starts_with("Not encrypted:"));
        let unknown = encryption_words(RoomEncryption::Unknown);
        assert_eq!(unknown, "Encryption not known yet");
        assert!(!unknown.contains("server can read"));
    }
}
