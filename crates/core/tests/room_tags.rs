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
    ActionKind, Command, CoreError, CoreEvent, ListDiff, RoomId, RoomListQuery, RoomListTab,
    RoomSummary, RoomTagView,
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

/// The ids of `mine` that `rooms` lists, sorted.
fn listed(rooms: &[RoomSummary], mine: &[&OwnedRoomId]) -> Vec<String> {
    let mut found: Vec<String> = mine
        .iter()
        .filter(|id| flags(rooms, id).is_some())
        .map(|id| id.to_string())
        .collect();
    found.sort();
    found
}

fn sorted(ids: &[&OwnedRoomId]) -> Vec<String> {
    let mut v: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    v.sort();
    v
}

#[tokio::test]
#[ignore = "needs the local homeserver"]
async fn each_new_tab_lists_exactly_its_rooms() {
    use matrix_sdk::ruma::events::Mentions;
    use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;

    let env = TestEnv::load();
    assert_local_homeserver(env.homeserver_url());
    let alice = raw_login(1).await;
    let bob = raw_login(2).await;
    let bob_id = env.user_id(2);

    let favourite = create_room(&alice, &unique_room_name(), None).await;
    let low = create_room(&alice, &unique_room_name(), None).await;
    let read = create_room(&alice, &unique_room_name(), None).await;
    let marked = create_room(&alice, &unique_room_name(), None).await;
    let mentioned = create_room(&alice, &unique_room_name(), Some(&bob_id)).await;
    let invited = create_room(&bob, &unique_room_name(), Some(&env.user_id(1))).await;

    // Bob joins his invitation to the mention room and mentions alice.
    bob.sync_once(matrix_sdk::config::SyncSettings::default())
        .await
        .expect("bob syncs");
    bob.get_room(&mentioned)
        .expect("bob knows the invitation")
        .join()
        .await
        .expect("bob joins");
    let content =
        RoomMessageEventContent::text_plain("ping").add_mentions(Mentions::with_user_ids([env
            .user_id(1)
            .try_into()
            .unwrap()]));
    common::send_content(&bob, &mentioned, &content).await;

    // The marked room has no message at all: only the unread mark.
    alice
        .sync_once(matrix_sdk::config::SyncSettings::default())
        .await
        .expect("alice syncs");
    alice
        .get_room(&marked)
        .expect("alice knows her room")
        .set_unread_flag(true)
        .await
        .expect("mark unread");

    let dir = tempfile::tempdir().unwrap();
    let mut core = start_core(dir.path(), Arc::new(InMemorySecretStore::new()));
    login_and_run(&mut core, &env, 1).await;
    let mut rooms = Vec::new();
    let all = [&favourite, &low, &read, &marked, &mentioned, &invited];
    assert!(
        pump(&mut core, &mut rooms, |r| listed(r, &all).len() == 6)
            .await
            .is_none()
    );
    for (id, tag) in [
        (&favourite, RoomTagView::Favourite),
        (&low, RoomTagView::LowPriority),
    ] {
        core.commands
            .send(Command::SetRoomTag {
                room_id: RoomId::from(id.as_str()),
                tag: Some(tag),
            })
            .await
            .unwrap();
    }
    let tagged = |r: &[RoomSummary]| {
        flags(r, &favourite) == Some((true, false)) && flags(r, &low) == Some((false, true))
    };
    assert!(pump(&mut core, &mut rooms, tagged).await.is_none());

    let cases: [(RoomListTab, Vec<&OwnedRoomId>); 5] = [
        (RoomListTab::Favourites, vec![&favourite]),
        (RoomListTab::LowPriority, vec![&low]),
        (RoomListTab::Invites, vec![&invited]),
        (RoomListTab::Mentions, vec![&mentioned, &marked]),
        (RoomListTab::Unreads, vec![&mentioned, &marked]),
    ];
    for (tab, expected) in cases {
        core.commands
            .send(Command::SetRoomListQuery(RoomListQuery {
                tab,
                ..RoomListQuery::default()
            }))
            .await
            .unwrap();
        // The new filter arrives as a `Reset`; wait for the expected set.
        let want = sorted(&expected);
        let failed = pump(&mut core, &mut rooms, |r| listed(r, &all) == want).await;
        assert!(failed.is_none(), "a command failed");
    }
}
