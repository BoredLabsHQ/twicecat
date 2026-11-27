//! The room panel (UI review fixes, Task 3): one column on the right of the
//! room pane with tabs for the room's members, its info and the open thread.
//!
//! This is view state only. The thread itself lives in [`crate::threads`];
//! the panel decides whether the column is shown and which tab it is on.

use crate::state::{App, Message};
use iced::Task;

/// A tab of the room panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoomPanelTab {
    /// The room's members.
    #[default]
    Members,
    /// The room's info and settings.
    Info,
    /// The open thread (only while one is open).
    Thread,
}

/// Whether the room panel is shown, and on which tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoomPanel {
    /// The tab shown.
    pub tab: RoomPanelTab,
    /// Whether the panel is open. The app also needs a selected room for it
    /// to be shown (see [`App::room_panel_shown`]).
    pub open: bool,
}

/// A control of the room panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomPanelMessage {
    /// Opens the panel on a tab.
    Open(RoomPanelTab),
    /// Switches to a tab of the open panel; the open thread stays.
    Tab(RoomPanelTab),
    /// Closes the panel, and the thread with it.
    Close,
}

impl App {
    /// Whether the room panel is shown: open, and a room is selected.
    pub fn room_panel_shown(&self) -> bool {
        self.room_panel.open && self.selected.is_some()
    }

    /// Whether `tab` can be shown now: the thread tab needs an open thread.
    fn tab_available(&self, tab: RoomPanelTab) -> bool {
        tab != RoomPanelTab::Thread || self.threads.panel.is_some()
    }

    /// Whether the Members tab is on screen.
    fn members_tab_shown(&self) -> bool {
        self.room_panel_shown() && self.room_panel.tab == RoomPanelTab::Members
    }

    /// Forgets the Members tab's state when the tab is not on screen, so a
    /// hidden tab never keeps (or fills in) a room's member list.
    pub(crate) fn sync_members_tab(&mut self) {
        if !self.members_tab_shown() {
            self.rooms_ui.close_members();
        }
    }

    /// A message of the room panel.
    pub(crate) fn on_room_panel(&mut self, message: RoomPanelMessage) -> Task<Message> {
        let task = match message {
            RoomPanelMessage::Open(tab) => {
                if self.selected.is_some() && self.tab_available(tab) {
                    self.room_panel = RoomPanel { tab, open: true };
                }
                Task::none()
            }
            RoomPanelMessage::Tab(tab) => {
                if self.room_panel_shown() && self.tab_available(tab) {
                    self.room_panel.tab = tab;
                }
                Task::none()
            }
            RoomPanelMessage::Close => {
                self.room_panel.open = false;
                self.close_thread_panel()
            }
        };
        self.sync_members_tab();
        if self.members_tab_shown() && self.rooms_ui.members_panel.is_none() {
            return Task::batch([task, self.open_members()]);
        }
        task
    }

    /// The open thread went away: a panel that showed it closes, one on
    /// another tab stays.
    pub(crate) fn thread_gone(&mut self) {
        if self.room_panel.tab == RoomPanelTab::Thread {
            self.room_panel.open = false;
        }
    }
}
