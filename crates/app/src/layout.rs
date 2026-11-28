//! Which panes the main window shows at its width (UI/UX rework, Task 8).
//!
//! A pure function of the window width, the user's saved room list choice and
//! which side panels are open. The saved choice never changes because of the
//! width: only what is shown does.

use twicecat_core::api::{RoomListChoice, SpacesRailChoice};

use crate::screens::room_list_pane;
use crate::screens::spaces_sidebar::RAIL_EXPANDED_WIDTH;

/// From this width (logical pixels) the user's choice applies and the side
/// panels are side panes.
pub const WIDE_MIN: f32 = 1100.0;

/// From this width (and under [`WIDE_MIN`]) the list is a rail while a side
/// panel is open. Under it the sidebar goes and a side panel takes the room
/// pane's width.
pub const MEDIUM_MIN: f32 = 800.0;

/// The width the window opens with, and the width the app assumes until the
/// first resize event arrives.
pub const DEFAULT_WINDOW_WIDTH: f32 = 1280.0;

/// The least width (logical pixels) the room pane keeps beside the wide
/// spaces rail. With less, the rail is shown collapsed, whatever the user
/// chose.
pub const ROOM_PANE_MIN_BESIDE_WIDE_RAIL: f32 = 480.0;

/// A panel beside (or, in a narrow window, instead of) the room pane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SidePanel {
    /// The room panel (members, info, thread).
    #[default]
    Room,
    /// The participants panel of a call.
    Call,
}

/// What the main window shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneLayout {
    /// Whether the spaces sidebar slot has width (it keeps its place either way).
    pub sidebar: bool,
    /// The room list as shown (the saved choice, unless the width forces the rail).
    pub list: RoomListChoice,
    /// Whether the room panel is shown.
    pub room_panel: bool,
    /// Whether the participants panel is shown.
    pub call: bool,
    /// Whether a shown panel takes the room pane's whole width (the room
    /// pane stays in the tree with no width).
    pub panel_fills: bool,
}

/// The layout for `width`. `newest` is the panel opened last: when both are
/// open and there is no room for both, it is the one shown.
pub fn layout_for(
    width: f32,
    choice: RoomListChoice,
    room_panel_open: bool,
    call_panel_open: bool,
    newest: SidePanel,
) -> PaneLayout {
    // `>=` is false for NaN, so a width that is not a number counts as narrow.
    let wide = width >= WIDE_MIN;
    let medium = width >= MEDIUM_MIN;
    let (room_panel, call) = if wide || !(room_panel_open && call_panel_open) {
        (room_panel_open, call_panel_open)
    } else {
        (newest == SidePanel::Room, newest == SidePanel::Call)
    };
    let any = room_panel || call;
    PaneLayout {
        sidebar: medium,
        list: if wide || (medium && !any) {
            choice
        } else {
            RoomListChoice::Rail
        },
        room_panel,
        call,
        panel_fills: !medium && any,
    }
}

/// Whether the spaces rail is shown in its wide form (the icon and the name
/// side by side) rather than the 72 px icon rail.
///
/// Only when the user chose it, the window is at least [`WIDE_MIN`] (so
/// every open side panel is a side pane) and the room pane would keep at
/// least [`ROOM_PANE_MIN_BESIDE_WIDE_RAIL`] beside the wide rail, the room
/// list as shown (`list`) and the open panels. Otherwise the narrow rail is
/// shown and the saved choice does not change: only what is shown does.
pub fn rail_expanded_shown(
    window_width: f32,
    choice: SpacesRailChoice,
    list: RoomListChoice,
    room_panel_open: bool,
    call_panel_open: bool,
) -> bool {
    // `>=` is false for NaN, so a width that is not a number counts as narrow.
    let wide = window_width >= WIDE_MIN;
    if choice != SpacesRailChoice::Expanded || !wide {
        return false;
    }
    // The rule and the list's slot, as `room_pane_width` takes them off.
    let mut room_pane = window_width - (RAIL_EXPANDED_WIDTH + 1.0);
    room_pane -= room_list_pane::pane_width(list) + 1.0;
    if room_panel_open {
        room_pane -= crate::threads::PANEL_WIDTH;
    }
    if call_panel_open {
        room_pane -= crate::calls::PANEL_WIDTH;
    }
    room_pane >= ROOM_PANE_MIN_BESIDE_WIDE_RAIL
}

#[cfg(test)]
mod tests {
    use RoomListChoice::{Expanded, Rail};
    use SidePanel::{Call, Room};

    use twicecat_core::api::SpacesRailChoice;

    use super::*;

    fn shown(
        width: f32,
        choice: RoomListChoice,
        t: bool,
        c: bool,
        newest: SidePanel,
    ) -> PaneLayout {
        layout_for(width, choice, t, c, newest)
    }

    fn l(
        sidebar: bool,
        list: RoomListChoice,
        room_panel: bool,
        call: bool,
        fills: bool,
    ) -> PaneLayout {
        PaneLayout {
            sidebar,
            list,
            room_panel,
            call,
            panel_fills: fills,
        }
    }

    #[test]
    fn wide_windows_apply_the_choice_and_show_every_open_panel() {
        for width in [1100.0, 1280.0, 4000.0] {
            for choice in [Expanded, Rail] {
                assert_eq!(
                    shown(width, choice, false, false, Room),
                    l(true, choice, false, false, false)
                );
                assert_eq!(
                    shown(width, choice, true, false, Room),
                    l(true, choice, true, false, false)
                );
                assert_eq!(
                    shown(width, choice, false, true, Call),
                    l(true, choice, false, true, false)
                );
                for newest in [Room, Call] {
                    assert_eq!(
                        shown(width, choice, true, true, newest),
                        l(true, choice, true, true, false)
                    );
                }
            }
        }
    }

    #[test]
    fn medium_windows_force_the_rail_only_while_a_panel_is_open() {
        for width in [800.0, 950.0, 1099.9] {
            assert_eq!(
                shown(width, Expanded, false, false, Room),
                l(true, Expanded, false, false, false)
            );
            assert_eq!(
                shown(width, Rail, false, false, Room),
                l(true, Rail, false, false, false)
            );
            assert_eq!(
                shown(width, Expanded, true, false, Room),
                l(true, Rail, true, false, false)
            );
            assert_eq!(
                shown(width, Expanded, false, true, Call),
                l(true, Rail, false, true, false)
            );
            assert_eq!(
                shown(width, Rail, true, false, Room),
                l(true, Rail, true, false, false)
            );
        }
    }

    #[test]
    fn with_both_open_the_one_opened_last_is_shown_and_the_other_waits() {
        for width in [800.0, 1099.9, 500.0, 799.9] {
            assert!(shown(width, Expanded, true, true, Call).call, "{width}");
            assert!(
                !shown(width, Expanded, true, true, Call).room_panel,
                "{width}"
            );
            assert!(
                shown(width, Expanded, true, true, Room).room_panel,
                "{width}"
            );
            assert!(!shown(width, Expanded, true, true, Room).call, "{width}");
        }
    }

    #[test]
    fn narrow_windows_drop_the_sidebar_use_the_rail_and_give_a_panel_the_room_pane() {
        for width in [799.9, 720.0, 400.0, 0.0] {
            for choice in [Expanded, Rail] {
                assert_eq!(
                    shown(width, choice, false, false, Room),
                    l(false, Rail, false, false, false)
                );
                assert_eq!(
                    shown(width, choice, true, false, Room),
                    l(false, Rail, true, false, true)
                );
                assert_eq!(
                    shown(width, choice, false, true, Call),
                    l(false, Rail, false, true, true)
                );
            }
        }
    }

    fn rail(width: f32, list: RoomListChoice, room_panel: bool, call: bool) -> bool {
        rail_expanded_shown(width, SpacesRailChoice::Expanded, list, room_panel, call)
    }

    #[test]
    fn the_wide_rail_is_shown_when_the_choice_the_width_and_the_room_pane_allow() {
        // 1280 - 201 - 301 = 778 px left for the room pane.
        assert!(rail(1280.0, Expanded, false, false));
        assert!(rail(1100.0, Expanded, false, false), "WIDE_MIN itself");
        // 1280 - 201 - 301 - 340 = 438: too little.
        assert!(!rail(1280.0, Expanded, true, false));
        assert!(!rail(1280.0, Expanded, false, true), "the call panel too");
        // The list's own rail leaves more room: 1280 - 201 - 73 - 340 = 666.
        assert!(rail(1280.0, Rail, true, false));
        // 1920 - 201 - 301 - 340 - 320 = 758.
        assert!(rail(1920.0, Expanded, true, true));
        assert!(!rail(1099.0, Expanded, false, false));
        assert!(!rail(799.0, Rail, false, false));
        assert!(!rail(f32::NAN, Expanded, false, false));
    }

    #[test]
    fn the_wide_rail_needs_exactly_480_px_for_the_room_pane() {
        let edge = ROOM_PANE_MIN_BESIDE_WIDE_RAIL + 201.0 + 301.0 + 340.0;
        assert!(rail(edge, Expanded, true, false));
        assert!(!rail(edge - 0.1, Expanded, true, false));
    }

    #[test]
    fn a_collapsed_choice_never_shows_the_wide_rail() {
        for width in [1100.0, 1920.0, 4000.0] {
            assert!(!rail_expanded_shown(
                width,
                SpacesRailChoice::Collapsed,
                Expanded,
                false,
                false
            ));
        }
    }

    #[test]
    fn a_width_that_is_not_a_number_is_treated_as_the_narrowest() {
        assert_eq!(
            shown(f32::NAN, Expanded, true, false, Room),
            l(false, Rail, true, false, true)
        );
    }
}
