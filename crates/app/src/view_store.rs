//! Where the selected space is remembered (sidebar rework, Task 11d).
//!
//! The app reopens on the space the user had selected: only that (not the
//! open room, the filter chip or the scroll), one small file per account in
//! the data root, never sent to the homeserver. The rest of the app reaches
//! it only through [`ViewStore`]; only this module (and `App::booter`, which
//! builds the disk store) names the core's `DataPaths` functions for it, and no
//! screen does. Nothing here logs,
//! and no `Debug` shows an ID.

use std::fmt;

use iced::Task;
use twicecat_core::api::{RoomId, UserId};
use twicecat_core::paths::DataPaths;

use crate::blocking;

/// The answer to a load: the space remembered for `account`, with the account
/// it was asked for (a late answer for another account is ignored). `Debug`
/// hides both.
#[derive(Clone, PartialEq, Eq)]
pub struct ViewAnswer {
    /// The account the load was for.
    pub account: UserId,
    /// The remembered space; `None` for Home or when nothing usable is kept.
    pub space: Option<RoomId>,
}

impl fmt::Debug for ViewAnswer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewAnswer")
            .field("account", &"<redacted>")
            .field("space", &self.space.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

/// Where the selected space is kept, one per account.
pub trait ViewStore: Send {
    /// The space remembered for `account`.
    fn load(&self, account: &UserId) -> Task<ViewAnswer>;
    /// Remembers `space` (`None` for Home) for `account`.
    fn save(&self, account: &UserId, space: Option<RoomId>) -> Task<()>;
    /// Forgets `account`'s space (an explicit logout).
    fn remove(&self, account: &UserId) -> Task<()>;
}

/// A store that does nothing; `App::new` installs it so tests never touch the
/// disk. It remembers nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoViewStore;

impl ViewStore for NoViewStore {
    fn load(&self, _account: &UserId) -> Task<ViewAnswer> {
        Task::none()
    }

    fn save(&self, _account: &UserId, _space: Option<RoomId>) -> Task<()> {
        Task::none()
    }

    fn remove(&self, _account: &UserId) -> Task<()> {
        Task::none()
    }
}

/// The view files in the data root. The disk work runs on [`blocking`]
/// threads; a write is small and whole, and the newest one that runs last
/// wins, which is the order the UI asked for in practice (one thread pair
/// at most, writes of the same account are rare).
#[derive(Clone)]
pub struct DiskView {
    paths: DataPaths,
}

impl fmt::Debug for DiskView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiskView").finish_non_exhaustive()
    }
}

impl DiskView {
    /// A store for the view files under `paths`.
    pub fn new(paths: DataPaths) -> Self {
        Self { paths }
    }
}

impl ViewStore for DiskView {
    fn load(&self, account: &UserId) -> Task<ViewAnswer> {
        let (paths, account) = (self.paths.clone(), account.clone());
        Task::future(async move {
            let asked = account.clone();
            let space = blocking::run(move || paths.read_view_space(&account))
                .await
                .flatten();
            ViewAnswer {
                account: asked,
                space,
            }
        })
    }

    fn save(&self, account: &UserId, space: Option<RoomId>) -> Task<()> {
        let (paths, account) = (self.paths.clone(), account.clone());
        Task::future(async move {
            // Best effort: a view that cannot be kept is only not remembered.
            let _ = blocking::run(move || paths.write_view_space(&account, space.as_ref())).await;
        })
    }

    fn remove(&self, account: &UserId) -> Task<()> {
        let (paths, account) = (self.paths.clone(), account.clone());
        Task::future(async move {
            let _ = blocking::run(move || paths.remove_view(&account)).await;
        })
    }
}

/// A fake store that records every call and touches no disk.
#[cfg(test)]
pub use fake::{Call, MemoryView};

#[cfg(test)]
mod fake {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use super::*;

    /// One call to a [`MemoryView`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Call {
        /// `load(account)`.
        Load(UserId),
        /// `save(account, space)`.
        Save(UserId, Option<RoomId>),
        /// `remove(account)`.
        Remove(UserId),
    }

    /// Keeps the spaces in memory per account, shared by its clones (a second
    /// `App` with a clone of the store is "the next start"), and records
    /// every call.
    #[derive(Debug, Clone, Default)]
    pub struct MemoryView {
        /// Every call so far, in order.
        pub calls: Arc<Mutex<Vec<Call>>>,
        /// What is remembered, per account.
        pub kept: Arc<Mutex<HashMap<UserId, Option<RoomId>>>>,
    }

    impl MemoryView {
        /// The calls made so far.
        pub fn calls(&self) -> Vec<Call> {
            self.calls.lock().expect("test mutex").clone()
        }

        /// How many saves were made.
        pub fn saves(&self) -> usize {
            self.calls()
                .iter()
                .filter(|c| matches!(c, Call::Save(..)))
                .count()
        }

        /// Pre-sets what `account` remembers.
        pub fn keep(&self, account: &str, space: Option<&str>) {
            self.kept
                .lock()
                .expect("test mutex")
                .insert(UserId::from(account), space.map(RoomId::from));
        }

        /// What `account` remembers.
        pub fn kept_for(&self, account: &str) -> Option<RoomId> {
            self.kept
                .lock()
                .expect("test mutex")
                .get(&UserId::from(account))
                .cloned()
                .flatten()
        }
    }

    impl ViewStore for MemoryView {
        fn load(&self, account: &UserId) -> Task<ViewAnswer> {
            self.calls
                .lock()
                .expect("test mutex")
                .push(Call::Load(account.clone()));
            let space = self
                .kept
                .lock()
                .expect("test mutex")
                .get(account)
                .cloned()
                .flatten();
            Task::done(ViewAnswer {
                account: account.clone(),
                space,
            })
        }

        fn save(&self, account: &UserId, space: Option<RoomId>) -> Task<()> {
            self.calls
                .lock()
                .expect("test mutex")
                .push(Call::Save(account.clone(), space.clone()));
            self.kept
                .lock()
                .expect("test mutex")
                .insert(account.clone(), space);
            Task::done(())
        }

        fn remove(&self, account: &UserId) -> Task<()> {
            self.calls
                .lock()
                .expect("test mutex")
                .push(Call::Remove(account.clone()));
            self.kept.lock().expect("test mutex").remove(account);
            Task::done(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_answer_hides_the_account_and_the_space_in_debug() {
        let answer = ViewAnswer {
            account: UserId::from("@secret:x"),
            space: Some(RoomId::from("!secretroom:x")),
        };
        let text = format!("{answer:?}");
        assert!(!text.contains("secret"), "{text}");
        assert!(text.contains("redacted"));
    }

    #[test]
    fn only_the_view_store_and_main_name_the_view_files_or_the_disk_store() {
        for (path, text) in crate::source_scan::sources() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let code = crate::source_scan::mask(&crate::source_scan::production_part(&text));
            if name != "view_store.rs" {
                for word in [
                    "read_view_space",
                    "write_view_space",
                    "remove_view",
                    "view_file",
                ] {
                    assert!(!code.contains(word), "{} names {word}", path.display());
                }
            }
            // `App::booter` builds the disk store from the data paths it is given.
            if name != "view_store.rs" && name != "main.rs" && name != "state.rs" {
                assert!(!code.contains("DiskView"), "{} names it", path.display());
            }
        }
    }
}
