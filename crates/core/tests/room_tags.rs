//! Favourite and low priority room tags against the local homeserver
//! (`scripts/stack.sh up -d --wait && scripts/create-test-users.sh`).
//!
//! Alice runs the core under test and makes her own room; a second SDK
//! client of the same account reads the tags back from the server. Nothing
//! here prints room names.

mod common;

use std::sync::Arc;

use common::{
    DEFAULT_TIMEOUT, TestEnv, assert_local_homeserver, create_room, login_and_run, raw_login,
    start_core, unique_room_name,
};
use matrix_sdk::ruma::OwnedRoomId;
use matrix_sdk::ruma::events::tag::TagName;
use twicecat_core::CoreHandle;
use twicecat_core::api::{
    ActionKind, Command, CoreError, CoreEvent, ListDiff, RoomId, RoomSummary, RoomTagView,
};
use twicecat_core::secrets::InMemorySecretStore;

/// Applies core events to `rooms` until `done` holds, or returns the
/// `ActionFailed` that came first.
async fn pump(
    core: &mut CoreHandle,
    rooms: &mut Vec<RoomSummary>,
    done: impl Fn(&[RoomSummary]) -> bool,
) -> Option<(ActionKind, RoomId, CoreError)> {
    let wait = async {
        while !done(rooms) {
            match core.events.recv().await {
                Some(CoreEvent::RoomListChanged(diffs)) => {
                    for diff in diffs {
                        ListDiff::apply(&diff, rooms);
                    }
                }
                Some(CoreEvent::ActionFailed {
                    room_id,
                    action,
                    error,
                }) => return Some((action, room_id, error)),
                Some(_) => {}
                None => panic!("event channel closed"),
            }
        }
        None
    };
    tokio::time::timeout(DEFAULT_TIMEOUT, wait)
        .await
        .expect("timed out waiting for the room list")
}

fn flags(rooms: &[RoomSummary], id: &OwnedRoomId) -> Option<(bool, bool)> {
    rooms
        .iter()
        .find(|r| r.id.as_str() == id.as_str())
        .map(|r| (r.is_favourite, r.is_low_priority))
}

async fn server_tags(client: &matrix_sdk::Client, id: &OwnedRoomId) -> (bool, bool) {
    let room = client
        .get_room(id)
        .expect("the second client knows the room");
    let tags = room
        .tags()
        .await
        .expect("read the tags")
        .unwrap_or_default();
    (
        tags.contains_key(&TagName::Favorite),
        tags.contains_key(&TagName::LowPriority),
    )
}

#[tokio::test]
#[ignore = "needs the local homeserver"]
async fn tags_are_set_switched_and_removed() {
    let env = TestEnv::load();
    assert_local_homeserver(env.homeserver_url());
    let raw = raw_login(1).await;
    let id = create_room(&raw, &unique_room_name(), None).await;
    raw.sync_once(matrix_sdk::config::SyncSettings::default())
        .await
        .expect("the second client syncs");

    let dir = tempfile::tempdir().unwrap();
    let mut core = start_core(dir.path(), Arc::new(InMemorySecretStore::new()));
    login_and_run(&mut core, &env, 1).await;
    let mut rooms = Vec::new();
    let room_id = RoomId::from(id.as_str());
    assert!(
        pump(&mut core, &mut rooms, |r| flags(r, &id).is_some())
            .await
            .is_none()
    );
    assert_eq!(flags(&rooms, &id), Some((false, false)));

    for (tag, want) in [
        (Some(RoomTagView::Favourite), (true, false)),
        (Some(RoomTagView::LowPriority), (false, true)),
        (None, (false, false)),
    ] {
        core.commands
            .send(Command::SetRoomTag {
                room_id: room_id.clone(),
                tag,
            })
            .await
            .unwrap();
        let failed = pump(&mut core, &mut rooms, |r| flags(r, &id) == Some(want)).await;
        assert!(failed.is_none(), "the tag change failed");
        // A second client of the same account sees what the server holds.
        raw.sync_once(matrix_sdk::config::SyncSettings::default())
            .await
            .expect("the second client syncs");
        assert_eq!(server_tags(&raw, &id).await, want);
    }
}

#[tokio::test]
#[ignore = "needs the local homeserver"]
async fn tagging_an_unknown_room_fails_without_a_panic() {
    let env = TestEnv::load();
    assert_local_homeserver(env.homeserver_url());
    let dir = tempfile::tempdir().unwrap();
    let mut core = start_core(dir.path(), Arc::new(InMemorySecretStore::new()));
    login_and_run(&mut core, &env, 1).await;
    let nowhere = RoomId::from("!nowhere:localhost");
    core.commands
        .send(Command::SetRoomTag {
            room_id: nowhere.clone(),
            tag: Some(RoomTagView::Favourite),
        })
        .await
        .unwrap();
    let mut rooms = Vec::new();
    let failed = pump(&mut core, &mut rooms, |_| false).await;
    assert!(matches!(
        failed,
        Some((ActionKind::RoomTag, room, _)) if room == nowhere
    ));
}
