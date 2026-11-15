//! The state machine of dragging in the sidebar (plan decision 6): `Idle`,
//! then `Pressed`, then `Dragging` after 4 px of movement, then a drop or a
//! cancel.
//!
//! iced 0.14 has no drag-and-drop widget, so the sectioned room list builds
//! it from `mouse_area` events plus a window listener (see `spaces.rs`). This
//! module knows none of that: it is generic over the item and the target and
//! takes plain events, so it is tested without a window. The only iced type
//! it uses is [`Point`].
//!
//! It serves **the room list only** (`Drag<ListKey>`): the press that may
//! become a click is the same press that may become a drag, so a click and a
//! drag can never both happen. The rail no longer uses it: its items reorder
//! in sweeten columns (`rail_reorder.rs`), because a sweeten column can reorder
//! the children of one container and the room list is virtualized, so it
//! cannot be one.

use iced::{Point, Size};

/// How far the pointer must move after the press for the press to become a
/// drag, in pixels. Less than this is a click that wobbled.
pub const THRESHOLD: f32 = 4.0;

/// Which part of a slot the pointer is over: the top third, the middle or the
/// bottom third.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// The top third: before the item.
    Before,
    /// The middle third: into the item.
    Into,
    /// The bottom third: after the item.
    After,
}

impl Zone {
    /// The zone of height `y` inside a slot `height` tall. Outside the slot
    /// the nearest zone is used.
    pub fn of(y: f32, height: f32) -> Self {
        if height <= 0.0 || y < height / 3.0 {
            Self::Before
        } else if y < 2.0 * height / 3.0 {
            Self::Into
        } else {
            Self::After
        }
    }
}

/// Where a drag would drop: on an item, in one of its zones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target<I> {
    /// The item under the pointer.
    pub item: I,
    /// The part of it.
    pub zone: Zone,
}

/// What an event did, for the caller to act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<I> {
    /// Nothing to do.
    Nothing,
    /// A press and a release with no movement: select or toggle the item.
    Click(I),
    /// The press became a drag.
    Started,
    /// The drag ended on a target: do it.
    Drop {
        /// What was dragged.
        item: I,
        /// Where it was dropped.
        target: Target<I>,
    },
    /// A drag ended with no drop (Esc, focus lost, or no target): nothing is
    /// done and nothing is saved.
    Cancelled,
}

/// Where the drag stands.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Drag<I> {
    /// No button is down on an item.
    #[default]
    Idle,
    /// The left button went down on `item` and has not moved far enough yet.
    Pressed {
        /// The item.
        item: I,
        /// Where the pointer was at the first movement after the press
        /// (window coordinates); `None` until there is one.
        at: Option<Point>,
        /// Whether the item may be dragged at all (Home may not).
        draggable: bool,
        /// Whether the pointer has left the item since the press: the click
        /// is then over.
        left: bool,
        /// Where the pointer was inside the item (item-local coordinates)
        /// when first seen after the press: the grab offset of the ghost.
        grab: Option<Point>,
    },
    /// A drag is running.
    Dragging {
        /// What is dragged.
        item: I,
        /// Where it would drop now.
        target: Option<Target<I>>,
        /// The last target worked out, so the same one is not worked out
        /// again on every pointer move.
        seen: Option<Target<I>>,
        /// A place the pointer is over where the dragged item may not go
        /// (drawn as not allowed). Never a target: a release there cancels.
        refused: Option<Target<I>>,
        /// Where the pointer is now (window coordinates): the ghost follows it.
        pointer: Point,
        /// Where the pointer was inside the item at the press, if it was
        /// seen (item-local coordinates).
        grab: Option<Point>,
    },
}

/// Where the top left corner of the ghost tile goes: the pointer minus the
/// grab offset (so the pointer stays at the same place on the tile as at the
/// press), kept inside the window. A tile larger than the window sits at 0.
///
/// ```
/// use iced::{Point, Size};
/// use twicecat_app::drag::ghost_position;
/// let at = ghost_position(
///     Point::new(100.0, 50.0),
///     Point::new(20.0, 10.0),
///     Size::new(200.0, 40.0),
///     Size::new(800.0, 600.0),
/// );
/// assert_eq!(at, Point::new(80.0, 40.0));
/// ```
pub fn ghost_position(pointer: Point, grab: Point, tile: Size, window: Size) -> Point {
    // `max` returns the other operand for NaN, so the bound is never NaN.
    let place = |at: f32, size: f32, limit: f32| {
        let top = (limit - size).max(0.0);
        if at.is_nan() { 0.0 } else { at.clamp(0.0, top) }
    };
    Point::new(
        place(pointer.x - grab.x, tile.width, window.width),
        place(pointer.y - grab.y, tile.height, window.height),
    )
}

impl<I: Clone + PartialEq> Drag<I> {
    /// Whether a button is down on an item: the window listener is needed.
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Idle)
    }

    /// Whether a drag (not just a press) is running.
    pub fn is_dragging(&self) -> bool {
        matches!(self, Self::Dragging { .. })
    }

    /// The item the button is down on while no drag has started (a click that
    /// may still become a drag).
    pub fn pressed(&self) -> Option<&I> {
        match self {
            Self::Pressed { item, .. } => Some(item),
            Self::Idle | Self::Dragging { .. } => None,
        }
    }

    /// What is dragged.
    pub fn dragged(&self) -> Option<&I> {
        match self {
            Self::Dragging { item, .. } => Some(item),
            Self::Idle | Self::Pressed { .. } => None,
        }
    }

    /// Where the drag would drop.
    pub fn target(&self) -> Option<&Target<I>> {
        match self {
            Self::Dragging { target, .. } => target.as_ref(),
            Self::Idle | Self::Pressed { .. } => None,
        }
    }

    /// The left button went down on `item`. Ignored during a drag; a second
    /// press while only pressed replaces the first (the earlier release was
    /// lost).
    pub fn press(&mut self, item: I, draggable: bool) {
        if self.is_dragging() {
            return;
        }
        *self = Self::Pressed {
            item,
            at: None,
            draggable,
            left: false,
            grab: None,
        };
    }

    /// The pointer moved to `at` (window coordinates) with the button down.
    pub fn moved(&mut self, at: Point) -> Outcome<I> {
        if let Self::Dragging { pointer, .. } = self {
            *pointer = at;
            return Outcome::Nothing;
        }
        let Self::Pressed {
            item,
            at: origin,
            draggable,
            grab,
            ..
        } = self
        else {
            return Outcome::Nothing;
        };
        let Some(origin) = *origin else {
            *origin = Some(at);
            return Outcome::Nothing;
        };
        let (dx, dy) = (at.x - origin.x, at.y - origin.y);
        if !*draggable || dx.hypot(dy) < THRESHOLD {
            return Outcome::Nothing;
        }
        *self = Self::Dragging {
            item: item.clone(),
            target: None,
            seen: None,
            refused: None,
            pointer: at,
            grab: *grab,
        };
        Outcome::Started
    }

    /// The pointer is at `local` (inside `item`) while the button is down on
    /// it and no drag has started: the first such place is the grab offset
    /// of the ghost. Ignored otherwise.
    pub fn grab(&mut self, item: &I, local: Point) {
        if let Self::Pressed {
            item: pressed,
            grab,
            ..
        } = self
            && pressed == item
            && grab.is_none()
        {
            *grab = Some(local);
        }
    }

    /// The pointer's window position, while a drag runs.
    pub fn pointer(&self) -> Option<Point> {
        match self {
            Self::Dragging { pointer, .. } => Some(*pointer),
            Self::Idle | Self::Pressed { .. } => None,
        }
    }

    /// Where the pointer was inside the dragged item at the press.
    pub fn grab_offset(&self) -> Option<Point> {
        match self {
            Self::Dragging { grab, .. } => *grab,
            Self::Idle | Self::Pressed { .. } => None,
        }
    }

    /// The pointer is over `target` (or over no target) during a drag. The
    /// dragged item itself is no target.
    pub fn hover(&mut self, target: Option<Target<I>>) {
        if let Self::Dragging {
            item,
            target: current,
            refused,
            ..
        } = self
        {
            *current = target.filter(|t| t.item != *item);
            *refused = None;
        }
    }

    /// The pointer is over `target`, which the dragged item may not be dropped
    /// on: there is no drop target, and the place is remembered so it can be
    /// drawn as not allowed.
    pub fn refuse(&mut self, place: Target<I>) {
        if let Self::Dragging {
            item,
            target,
            refused,
            ..
        } = self
        {
            *target = None;
            *refused = Some(place).filter(|t| t.item != *item);
        }
    }

    /// The place the pointer is over where the dragged item may not go.
    pub fn refused(&self) -> Option<&Target<I>> {
        match self {
            Self::Dragging { refused, .. } => refused.as_ref(),
            Self::Idle | Self::Pressed { .. } => None,
        }
    }

    /// Whether `target` is the one evaluated last (so its work can be
    /// skipped); records it otherwise. Only meaningful during a drag; reset by
    /// [`Self::leave`] and [`Self::forget_seen`].
    pub fn seen(&mut self, target: &Target<I>) -> bool {
        let Self::Dragging { seen, .. } = self else {
            return false;
        };
        if seen.as_ref() == Some(target) {
            return true;
        }
        *seen = Some(target.clone());
        false
    }

    /// Forgets the target evaluated last (the layout changed under the drag).
    pub fn forget_seen(&mut self) {
        if let Self::Dragging { seen, .. } = self {
            *seen = None;
        }
    }

    /// The pointer left `item`.
    pub fn leave(&mut self, item: &I) {
        match self {
            Self::Pressed { left, .. } => *left = true,
            Self::Dragging {
                target,
                seen,
                refused,
                ..
            } => {
                if target.as_ref().is_some_and(|t| t.item == *item) {
                    *target = None;
                }
                if refused.as_ref().is_some_and(|t| t.item == *item) {
                    *refused = None;
                }
                if seen.as_ref().is_some_and(|t| t.item == *item) {
                    *seen = None;
                }
            }
            Self::Idle => {}
        }
    }

    /// The button came up over `item`: a click when it went down on the same
    /// item and the pointer has not left it. During a drag this does nothing;
    /// [`Self::end`] ends it.
    pub fn release_on(&mut self, item: &I) -> Outcome<I> {
        match self {
            Self::Pressed {
                item: pressed,
                left: false,
                ..
            } if pressed == item => {
                let click = Outcome::Click(pressed.clone());
                *self = Self::Idle;
                click
            }
            _ => Outcome::Nothing,
        }
    }

    /// The left button came up anywhere in the window.
    pub fn end(&mut self) -> Outcome<I> {
        match std::mem::take(self) {
            Self::Dragging {
                item,
                target: Some(target),
                ..
            } => Outcome::Drop { item, target },
            Self::Dragging { target: None, .. } => Outcome::Cancelled,
            Self::Idle | Self::Pressed { .. } => Outcome::Nothing,
        }
    }

    /// Esc, or the window lost focus.
    pub fn cancel(&mut self) -> Outcome<I> {
        match std::mem::take(self) {
            Self::Dragging { .. } => Outcome::Cancelled,
            Self::Idle | Self::Pressed { .. } => Outcome::Nothing,
        }
    }

    /// Forgets what `keep` refuses: a press or drag of an item that is no
    /// longer drawn is dropped, and so is a target that is gone.
    pub fn retain(&mut self, keep: impl Fn(&I) -> bool) {
        match self {
            Self::Idle => {}
            Self::Pressed { item, .. } => {
                if !keep(item) {
                    *self = Self::Idle;
                }
            }
            Self::Dragging {
                item,
                target,
                seen,
                refused,
                ..
            } => {
                if !keep(item) {
                    *self = Self::Idle;
                } else {
                    if target.as_ref().is_some_and(|t| !keep(&t.item)) {
                        *target = None;
                    }
                    if refused.as_ref().is_some_and(|t| !keep(&t.item)) {
                        *refused = None;
                    }
                    if seen.as_ref().is_some_and(|t| !keep(&t.item)) {
                        *seen = None;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type D = Drag<&'static str>;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn before(item: &'static str) -> Option<Target<&'static str>> {
        Some(Target {
            item,
            zone: Zone::Before,
        })
    }

    /// Presses `a` and moves 20 px away: a drag of `a`.
    fn dragging() -> D {
        let mut d = D::default();
        d.press("a", true);
        assert_eq!(d.moved(at(10.0, 10.0)), Outcome::Nothing);
        assert_eq!(d.moved(at(30.0, 10.0)), Outcome::Started);
        d
    }

    #[test]
    fn the_ghost_keeps_the_grab_offset_and_stays_in_the_window() {
        let tile = Size::new(200.0, 40.0);
        let window = Size::new(800.0, 600.0);
        let grab = at(20.0, 10.0);
        let table = [
            // (pointer, expected top left)
            (at(100.0, 50.0), at(80.0, 40.0)),
            (at(0.0, 0.0), at(0.0, 0.0)),
            (at(5.0, 700.0), at(0.0, 560.0)),
            (at(790.0, 590.0), at(600.0, 560.0)),
            (at(-30.0, -30.0), at(0.0, 0.0)),
            (at(400.0, 300.0), at(380.0, 290.0)),
        ];
        for (pointer, want) in table {
            assert_eq!(
                ghost_position(pointer, grab, tile, window),
                want,
                "{pointer:?}"
            );
        }
        // A tile larger than the window sits at the corner.
        assert_eq!(
            ghost_position(at(50.0, 50.0), grab, Size::new(900.0, 700.0), window),
            at(0.0, 0.0)
        );
        // Not a number: the corner, not a panic.
        let nan = ghost_position(at(f32::NAN, 5.0), grab, tile, window);
        assert!(nan.x.is_finite() && nan.y.is_finite());
    }

    #[test]
    fn the_pointer_follows_a_drag_and_is_not_kept_before_it() {
        let mut d = D::default();
        d.press("a", true);
        assert_eq!(d.pointer(), None);
        let _ = d.moved(at(10.0, 10.0));
        assert_eq!(
            d.moved(at(11.0, 10.0)),
            Outcome::Nothing,
            "under the threshold"
        );
        assert_eq!(d.pointer(), None, "a press is not a drag");
        assert_eq!(d.moved(at(30.0, 10.0)), Outcome::Started);
        assert_eq!(d.pointer(), Some(at(30.0, 10.0)));
        assert_eq!(d.moved(at(55.0, 42.0)), Outcome::Nothing);
        assert_eq!(d.pointer(), Some(at(55.0, 42.0)));
        let _ = d.cancel();
        assert_eq!(d.pointer(), None);
    }

    #[test]
    fn the_first_grab_of_the_pressed_item_becomes_the_offset_of_the_drag() {
        let mut d = D::default();
        d.press("a", true);
        d.grab(&"b", at(1.0, 1.0)); // another item: ignored
        d.grab(&"a", at(12.0, 9.0));
        d.grab(&"a", at(30.0, 20.0)); // only the first counts
        let _ = d.moved(at(10.0, 10.0));
        assert_eq!(d.moved(at(30.0, 10.0)), Outcome::Started);
        assert_eq!(d.grab_offset(), Some(at(12.0, 9.0)));
        // No grab seen: no offset.
        assert_eq!(dragging().grab_offset(), None);
        // A grab during the drag changes nothing.
        d.grab(&"a", at(0.0, 0.0));
        assert_eq!(d.grab_offset(), Some(at(12.0, 9.0)));
    }

    #[test]
    fn a_movement_under_4_px_followed_by_a_release_is_a_click() {
        let mut d = D::default();
        d.press("a", true);
        assert!(d.is_active() && !d.is_dragging());
        assert_eq!(d.moved(at(10.0, 10.0)), Outcome::Nothing);
        // 3 px along one axis, and 2 + 2 px diagonally (under 4 in all).
        assert_eq!(d.moved(at(13.0, 10.0)), Outcome::Nothing);
        assert_eq!(d.moved(at(12.0, 12.0)), Outcome::Nothing);
        assert!(!d.is_dragging());
        assert_eq!(d.release_on(&"a"), Outcome::Click("a"));
        assert!(!d.is_active());
        // The window's release that follows ends nothing.
        assert_eq!(d.end(), Outcome::Nothing);
    }

    #[test]
    fn four_px_or_more_starts_a_drag_and_it_is_no_click() {
        let mut d = D::default();
        d.press("a", true);
        assert_eq!(d.moved(at(10.0, 10.0)), Outcome::Nothing);
        assert_eq!(d.moved(at(13.9, 10.0)), Outcome::Nothing);
        assert_eq!(d.moved(at(14.0, 10.0)), Outcome::Started);
        assert!(d.is_dragging());
        assert_eq!(d.dragged(), Some(&"a"));
        // Back over the item and up: still no click.
        assert_eq!(d.release_on(&"a"), Outcome::Nothing);
        assert!(d.is_dragging(), "the window's release ends a drag");
        // A diagonal of 4 px or more counts as well.
        let mut d = D::default();
        d.press("a", true);
        let _ = d.moved(at(0.0, 0.0));
        assert_eq!(d.moved(at(3.0, 3.0)), Outcome::Started);
    }

    #[test]
    fn the_thirds_give_before_into_and_after() {
        assert_eq!(Zone::of(0.0, 48.0), Zone::Before);
        assert_eq!(Zone::of(15.9, 48.0), Zone::Before);
        assert_eq!(Zone::of(16.0, 48.0), Zone::Into);
        assert_eq!(Zone::of(31.9, 48.0), Zone::Into);
        assert_eq!(Zone::of(32.0, 48.0), Zone::After);
        assert_eq!(Zone::of(48.0, 48.0), Zone::After);
        // Outside the slot the nearest zone is used.
        assert_eq!(Zone::of(-5.0, 48.0), Zone::Before);
        assert_eq!(Zone::of(80.0, 48.0), Zone::After);
        // A slot with no height is never "into".
        assert_ne!(Zone::of(0.0, 0.0), Zone::Into);
    }

    #[test]
    fn a_release_with_no_target_esc_or_focus_loss_cancels() {
        // No target: the window's release cancels.
        let mut d = dragging();
        assert_eq!(d.end(), Outcome::Cancelled);
        assert!(!d.is_active());
        // A target that is left: no target.
        let mut d = dragging();
        d.hover(before("b"));
        d.leave(&"b");
        assert_eq!(d.end(), Outcome::Cancelled);
        // Esc, and the window losing focus, both go through `cancel`.
        let mut d = dragging();
        d.hover(before("b"));
        assert_eq!(d.cancel(), Outcome::Cancelled);
        assert!(!d.is_active());
        assert_eq!(
            d.end(),
            Outcome::Nothing,
            "the release after Esc drops nothing"
        );
        // Cancelling a plain press is not a cancelled drag.
        let mut d = D::default();
        d.press("a", true);
        assert_eq!(d.cancel(), Outcome::Nothing);
        assert!(!d.is_active());
    }

    #[test]
    fn a_release_on_a_target_drops_there() {
        let mut d = dragging();
        d.hover(before("b"));
        d.hover(Some(Target {
            item: "c",
            zone: Zone::Into,
        }));
        assert_eq!(
            d.end(),
            Outcome::Drop {
                item: "a",
                target: Target {
                    item: "c",
                    zone: Zone::Into
                }
            }
        );
        assert!(!d.is_active());
    }

    #[test]
    fn the_dragged_item_is_no_target() {
        let mut d = dragging();
        d.hover(before("a"));
        assert_eq!(d.target(), None);
        assert_eq!(d.end(), Outcome::Cancelled);
    }

    #[test]
    fn a_second_press_during_a_drag_is_ignored() {
        let mut d = dragging();
        d.hover(before("b"));
        d.press("c", true);
        assert_eq!(d.dragged(), Some(&"a"));
        assert_eq!(d.target(), before("b").as_ref());
        // A press while only pressed replaces the earlier press.
        let mut d = D::default();
        d.press("a", true);
        d.press("b", true);
        assert_eq!(d.release_on(&"b"), Outcome::Click("b"));
    }

    #[test]
    fn an_item_that_may_not_be_dragged_only_clicks() {
        let mut d = D::default();
        d.press("home", false);
        let _ = d.moved(at(0.0, 0.0));
        assert_eq!(d.moved(at(100.0, 100.0)), Outcome::Nothing);
        assert!(!d.is_dragging());
        // The pointer is no longer on the item: no click either, as for any
        // press the pointer left.
        d.leave(&"home");
        assert_eq!(d.release_on(&"home"), Outcome::Nothing);
    }

    #[test]
    fn leaving_the_item_ends_the_click_and_a_release_elsewhere_is_none() {
        let mut d = D::default();
        d.press("a", true);
        d.leave(&"a");
        assert_eq!(d.release_on(&"a"), Outcome::Nothing);
        let mut d = D::default();
        d.press("a", true);
        assert_eq!(d.release_on(&"b"), Outcome::Nothing);
        // A release with no press is nothing.
        assert_eq!(D::default().release_on(&"a"), Outcome::Nothing);
        // The window's release ends a press that never saw its own release.
        assert_eq!(d.end(), Outcome::Nothing);
        assert!(!d.is_active());
    }

    #[test]
    fn the_same_target_is_evaluated_once_until_the_pointer_leaves_or_things_change() {
        let mut d = dragging();
        let t = |item, zone| Target { item, zone };
        assert!(!d.seen(&t("b", Zone::Before)), "the first time");
        assert!(d.seen(&t("b", Zone::Before)), "the same again");
        assert!(!d.seen(&t("b", Zone::After)), "another zone");
        assert!(!d.seen(&t("c", Zone::After)), "another item");
        assert!(d.seen(&t("c", Zone::After)));
        d.leave(&"c");
        assert!(!d.seen(&t("c", Zone::After)), "re-entered after leaving");
        d.forget_seen();
        assert!(!d.seen(&t("c", Zone::After)), "after a change");
        // Not dragging: nothing is remembered.
        let mut idle = D::default();
        assert!(!idle.seen(&t("c", Zone::After)));
        assert!(!idle.seen(&t("c", Zone::After)));
    }

    #[test]
    fn items_that_vanish_end_the_press_or_the_drag() {
        let mut d = dragging();
        d.hover(before("b"));
        d.retain(|i| *i != "b");
        assert_eq!(d.target(), None);
        assert!(d.is_dragging());
        d.retain(|i| *i != "a");
        assert!(!d.is_active());
        let mut d = D::default();
        d.press("a", true);
        d.retain(|i| *i != "a");
        assert!(!d.is_active());
    }

    #[test]
    fn a_refused_place_is_no_target_and_a_release_there_cancels() {
        let mut d = dragging();
        d.hover(before("b"));
        d.refuse(Target {
            item: "c",
            zone: Zone::Into,
        });
        assert_eq!(d.target(), None, "a refused place is no target");
        assert_eq!(
            d.refused(),
            Some(&Target {
                item: "c",
                zone: Zone::Into
            })
        );
        assert_eq!(d.end(), Outcome::Cancelled, "nothing is dropped");
        assert_eq!(d.refused(), None);
        // A good target, leaving the place, or an item that vanishes clears it.
        let mut d = dragging();
        d.refuse(Target {
            item: "c",
            zone: Zone::Into,
        });
        d.hover(before("b"));
        assert_eq!(d.refused(), None, "a target replaces the refusal");
        d.refuse(Target {
            item: "c",
            zone: Zone::Into,
        });
        d.leave(&"c");
        assert_eq!(d.refused(), None, "leaving the place");
        d.refuse(Target {
            item: "c",
            zone: Zone::Into,
        });
        d.retain(|i| *i != "c");
        assert_eq!(d.refused(), None, "the place is gone");
        // Not dragging: nothing is kept.
        let mut idle = D::default();
        idle.refuse(Target {
            item: "c",
            zone: Zone::Into,
        });
        assert_eq!(idle.refused(), None);
    }
}
