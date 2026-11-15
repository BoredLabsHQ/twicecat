//! Reordering the rail by dragging (sidebar rework plan, decisions S1, S3
//! and S4).
//!
//! The rail is drawn with `sweeten` columns: one for the top-level items and
//! one inside each open folder. A sweeten column reports only indices (which
//! child was picked, and which index it was dropped on). This module turns
//! those into what the rest of the app already speaks: a [`RailKey`] and a
//! place before or after another key, which `rail_drop::plan` turns into the
//! same `Move up` and `Move down` operations the options card produces.
//!
//! Nothing here knows about a window or logs anything; `Debug` of every type
//! hides room IDs because [`RailKey`]'s does.

use sweeten::widget::drag::DragEvent;

use crate::drag::{Target, Zone};
use crate::spaces::{RailItem, RailKey};

/// Which sweeten column of the rail an event comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailColumn {
    /// The top-level spaces and folders.
    Top,
    /// The spaces inside the open folder with this ID.
    Folder(u16),
}

/// What a sweeten column reports, in the app's own type (sweeten's derives
/// only `Debug` and `Clone`, and `SpacesMessage` needs `PartialEq`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReorderEvent {
    /// The item at `index` was picked up.
    Picked {
        /// Its index in the column.
        index: usize,
    },
    /// The item at `index` was dropped on the place of the item at
    /// `target_index`.
    Dropped {
        /// Where it was.
        index: usize,
        /// Where it was dropped.
        target_index: usize,
    },
    /// The drag ended with no drop (the pointer left, or its position was
    /// lost).
    Canceled {
        /// The item that was dragged.
        index: usize,
    },
}

impl From<DragEvent> for ReorderEvent {
    fn from(event: DragEvent) -> Self {
        match event {
            DragEvent::Picked { index } => Self::Picked { index },
            DragEvent::Dropped {
                index,
                target_index,
            } => Self::Dropped {
                index,
                target_index,
            },
            DragEvent::Canceled { index } => Self::Canceled { index },
        }
    }
}

/// A drag that runs: the column, and the keys it held when the item was
/// picked, so a drop can be refused when the rail changed in between.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailReorder {
    /// The column the item was picked in.
    pub column: RailColumn,
    /// The keys of its children, in order.
    pub keys: Vec<RailKey>,
}

/// Where a drop puts the dragged item, next to another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// Right before this key.
    Before(RailKey),
    /// Right after this key.
    After(RailKey),
}

impl Place {
    /// The target `rail_drop::plan` takes.
    pub fn target(self) -> Target<RailKey> {
        match self {
            Self::Before(item) => Target {
                item,
                zone: Zone::Before,
            },
            Self::After(item) => Target {
                item,
                zone: Zone::After,
            },
        }
    }
}

/// The place a drop of the item at `index` on the place of the item at
/// `target_index` stands for (sweeten removes the item and inserts it at the
/// target's index): after the key there when it goes down, before it when it
/// goes up. `None` for the same index or an index out of range.
pub fn target(keys: &[RailKey], index: usize, target_index: usize) -> Option<Place> {
    let target = keys.get(target_index)?;
    keys.get(index)?;
    match target_index.cmp(&index) {
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(Place::After(target.clone())),
        std::cmp::Ordering::Less => Some(Place::Before(target.clone())),
    }
}

/// The keys of the children of `column`, in the order they are drawn: the
/// items of the rail, or the spaces of an open folder. `None` for a folder
/// that is not drawn open.
pub fn column_keys(items: &[RailItem<'_>], column: RailColumn) -> Option<Vec<RailKey>> {
    match column {
        RailColumn::Top => Some(
            items
                .iter()
                .map(|item| match item {
                    RailItem::Space(space) => RailKey::Space(space.id.clone()),
                    RailItem::Folder(folder) => RailKey::Folder(folder.id),
                })
                .collect(),
        ),
        RailColumn::Folder(id) => items.iter().find_map(|item| match item {
            RailItem::Folder(folder) if folder.id == id && folder.open => Some(
                folder
                    .spaces
                    .iter()
                    .map(|space| RailKey::Space(space.id.clone()))
                    .collect(),
            ),
            _ => None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use twicecat_core::api::{FolderLayout, RailEntry, RoomId, SidebarLayout, UserId};

    use super::*;
    use crate::rail_drop;
    use crate::sidebar::Op;
    use crate::sidebar_menu::{Entry, Target as MenuTarget, op_for};

    fn rid(id: &str) -> RoomId {
        RoomId::from(format!("!{id}:x"))
    }

    fn sp(id: &str) -> RailKey {
        RailKey::Space(rid(id))
    }

    fn keys(ids: &[&str]) -> Vec<RailKey> {
        ids.iter().map(|id| sp(id)).collect()
    }

    fn card(key: &RailKey) -> MenuTarget {
        match key {
            RailKey::Space(id) => MenuTarget::Space(id.clone()),
            RailKey::Folder(id) => MenuTarget::Folder(*id),
            RailKey::Home => unreachable!("Home has no card"),
        }
    }

    #[test]
    fn equal_indices_and_out_of_range_indices_give_nothing() {
        let k = keys(&["a", "b", "c", "d"]);
        for i in 0..k.len() {
            assert_eq!(target(&k, i, i), None, "{i}");
        }
        assert_eq!(target(&k, 4, 0), None);
        assert_eq!(target(&k, 0, 4), None);
        assert_eq!(target(&k, 9, 9), None);
        assert_eq!(target(&[], 0, 0), None);
    }

    #[test]
    fn a_larger_target_is_after_that_key_and_a_smaller_one_before_it() {
        let k = keys(&["a", "b", "c", "d"]);
        assert_eq!(target(&k, 0, 3), Some(Place::After(k[3].clone())));
        assert_eq!(target(&k, 3, 0), Some(Place::Before(k[0].clone())));
        assert_eq!(target(&k, 1, 2), Some(Place::After(k[2].clone())));
        assert_eq!(target(&k, 2, 1), Some(Place::Before(k[1].clone())));
    }

    /// The layout of a rail, and the spaces the core reports.
    fn layout(rail: Vec<RailEntry>) -> SidebarLayout {
        let mut layout = SidebarLayout::new(UserId::from("@me:x"));
        layout.rail = rail;
        layout
    }

    /// For every ordered pair of the column's items, the drop works out to
    /// the card's Move up or Move down, as many times as the items are apart.
    fn assert_matches_the_card(layout: &SidebarLayout, known: &[RoomId], column: &[RailKey]) {
        for from in 0..column.len() {
            for to in 0..column.len() {
                let place = target(column, from, to);
                let item = &column[from];
                let ops = place.map_or_else(Vec::new, |place| {
                    rail_drop::plan(layout, known, item, &place.target())
                });
                let entry = if to > from {
                    Entry::MoveDown
                } else {
                    Entry::MoveUp
                };
                let buttons: Vec<Op> = (0..from.abs_diff(to))
                    .map(|_| op_for(&card(item), entry).expect("the card offers it"))
                    .collect();
                assert_eq!(ops, buttons, "{from} to {to}");
            }
        }
    }

    #[test]
    fn on_a_four_item_rail_with_a_folder_every_drop_is_the_cards_moves() {
        let layout = layout(vec![
            RailEntry::Space(rid("a")),
            RailEntry::Folder(FolderLayout {
                id: 1,
                name: None,
                open: false,
                spaces: vec![rid("x"), rid("y")],
            }),
            RailEntry::Space(rid("b")),
            RailEntry::Space(rid("c")),
        ]);
        let known: Vec<RoomId> = ["a", "x", "y", "b", "c"].map(rid).to_vec();
        let column = vec![sp("a"), RailKey::Folder(1), sp("b"), sp("c")];
        assert_matches_the_card(&layout, &known, &column);
    }

    #[test]
    fn inside_a_three_space_folder_every_drop_is_the_cards_moves() {
        let layout = layout(vec![
            RailEntry::Space(rid("a")),
            RailEntry::Folder(FolderLayout {
                id: 1,
                name: None,
                open: true,
                spaces: vec![rid("x"), rid("y"), rid("z")],
            }),
        ]);
        let known: Vec<RoomId> = ["a", "x", "y", "z"].map(rid).to_vec();
        assert_matches_the_card(&layout, &known, &keys(&["x", "y", "z"]));
    }

    #[test]
    fn a_stored_space_the_core_does_not_report_does_not_shift_the_drop() {
        // The column holds drawn items only, so a ghost between them is not
        // an index: dropping a on c still takes the card's one step over b.
        let layout = layout(vec![
            RailEntry::Space(rid("a")),
            RailEntry::Space(rid("ghost")),
            RailEntry::Space(rid("b")),
        ]);
        let known: Vec<RoomId> = ["a", "b"].map(rid).to_vec();
        assert_matches_the_card(&layout, &known, &keys(&["a", "b"]));
    }

    #[test]
    fn the_keys_of_a_column_are_its_drawn_children() {
        use crate::spaces::SpacesModel;
        use twicecat_core::api::SpaceView;

        let space = |id: &str| SpaceView {
            id: rid(id),
            name: id.to_owned(),
            avatar: None,
            level: 0,
            children: 0,
            rooms: Vec::new(),
        };
        let mut model = SpacesModel::default();
        model.set(["a", "x", "y", "b"].map(space).to_vec());
        let layout = layout(vec![
            RailEntry::Space(rid("a")),
            RailEntry::Folder(FolderLayout {
                id: 3,
                name: None,
                open: true,
                spaces: vec![rid("x"), rid("y")],
            }),
            RailEntry::Folder(FolderLayout {
                id: 4,
                name: None,
                open: false,
                spaces: vec![rid("b"), rid("gone")],
            }),
        ]);
        let items = model.rail_items(Some(&layout));
        assert_eq!(
            column_keys(&items, RailColumn::Top),
            Some(vec![sp("a"), RailKey::Folder(3), RailKey::Folder(4)])
        );
        assert_eq!(
            column_keys(&items, RailColumn::Folder(3)),
            Some(keys(&["x", "y"]))
        );
        assert_eq!(column_keys(&items, RailColumn::Folder(4)), None, "closed");
        assert_eq!(column_keys(&items, RailColumn::Folder(9)), None, "unknown");
    }

    #[test]
    fn a_sweeten_event_becomes_ours_and_its_debug_shows_no_id() {
        let picked: ReorderEvent = DragEvent::Picked { index: 2 }.into();
        assert_eq!(picked, ReorderEvent::Picked { index: 2 });
        let dropped: ReorderEvent = DragEvent::Dropped {
            index: 0,
            target_index: 3,
        }
        .into();
        assert_eq!(
            dropped,
            ReorderEvent::Dropped {
                index: 0,
                target_index: 3
            }
        );
        let canceled: ReorderEvent = DragEvent::Canceled { index: 1 }.into();
        assert_eq!(canceled, ReorderEvent::Canceled { index: 1 });
        let reorder = RailReorder {
            column: RailColumn::Top,
            keys: keys(&["secret"]),
        };
        assert!(!format!("{reorder:?}").contains("secret"));
    }
}
