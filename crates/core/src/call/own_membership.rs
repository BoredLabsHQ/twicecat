//! Our membership in the room state (Element Call's compatibility mode,
//! see `membership`): the delayed empty membership that leaves for us if we
//! crash (MSC4140), restarted while we are alive; the membership itself,
//! refreshed before it expires; and joining again from scratch when it is
//! gone.
//!
//! Every update of the delayed leave goes through [`update_delayed_leave`],
//! which sends the access token. ruma declares the current MSC4140 update
//! request (`unstable_v2`) as `NoAccessToken`, so the SDK leaves the
//! `Authorization` header out, and newer Synapse answers that with
//! `401 M_MISSING_TOKEN`: the delayed leave is never restarted and fires
//! while we are still in the call.

use std::sync::Arc;
use std::time::Duration;

use matrix_sdk::ruma::api::client::delayed_events::update_delayed_event::UpdateAction;
use matrix_sdk::ruma::api::client::delayed_events::update_delayed_event::{
    unstable_v1 as update_delayed_v1, unstable_v2 as update_delayed,
};
use matrix_sdk::ruma::api::client::delayed_events::{DelayParameters, delayed_state_event};
use matrix_sdk::ruma::api::error::ErrorKind;
use matrix_sdk::ruma::events::StateEventType;
use matrix_sdk::ruma::serde::Raw;
use matrix_sdk::{Client, Room};
use serde_json::{Value, json};
use tokio::sync::{Mutex, Notify};

use super::membership::{
    self, CALL_MEMBER_EVENT, DELAYED_LEAVE, DELAYED_LEAVE_RESTART, MEMBERSHIP_EXPIRES,
};
use super::now_ms;
use crate::api::{CallIntent, CoreError};

/// Refresh our membership this long before it would expire.
const REFRESH_MARGIN: Duration = Duration::from_secs(10 * 60);

/// Schedules the empty membership that fires if we stop restarting it.
/// `Ok(None)` when the server has no delayed events (MSC4140): the call
/// still works, but a crash leaves a stale membership until it expires.
pub(super) async fn schedule_delayed_leave(
    client: &Client,
    room: &Room,
    state_key: &str,
) -> Result<Option<String>, CoreError> {
    let empty: Raw<_> = Raw::new(&json!({}))
        .map_err(|_| CoreError::Internal("empty membership".into()))?
        .cast_unchecked();
    let request = delayed_state_event::unstable::Request::new_raw(
        room.room_id().to_owned(),
        state_key.to_owned(),
        StateEventType::from(CALL_MEMBER_EVENT),
        DelayParameters::Timeout {
            timeout: DELAYED_LEAVE,
        },
        empty,
    );
    match client.send(request).await {
        Ok(reply) => Ok(Some(reply.delay_id)),
        Err(e)
            if matches!(
                e.client_api_error_kind(),
                Some(ErrorKind::Unrecognized | ErrorKind::NotFound)
            ) =>
        {
            tracing::warn!("homeserver has no delayed events; crash-safe leave disabled");
            Ok(None)
        }
        Err(_) => Err(CoreError::Call("could not schedule the call leave".into())),
    }
}

/// Updates the delayed leave (restart, cancel, send), with the access token.
/// The request type has none (`NoAccessToken`), so `force_auth` adds it. A
/// server that does not know the `{action}` path gets the older request,
/// which has the action in the body (as the SDK's widget driver sends it).
pub(super) async fn update_delayed_leave(
    client: &Client,
    delay_id: String,
    action: UpdateAction,
) -> Result<(), matrix_sdk::HttpError> {
    let config = client.request_config().force_auth();
    let current = client
        .send(update_delayed::Request::new(
            delay_id.clone(),
            action.clone(),
        ))
        .with_request_config(config)
        .await;
    match current {
        Ok(_) => Ok(()),
        Err(e) if matches!(e.client_api_error_kind(), Some(ErrorKind::Unrecognized)) => client
            .send(update_delayed_v1::Request::new(delay_id, action))
            .await
            .map(|_| ()),
        Err(e) => Err(e),
    }
}

/// Sends our membership; returns its event ID.
pub(super) async fn send_membership(
    room: &Room,
    state_key: &str,
    content: Value,
) -> Result<String, CoreError> {
    room.send_state_event_raw(CALL_MEMBER_EVENT, state_key, content)
        .await
        .map(|r| r.event_id.to_string())
        .map_err(|e| match e.client_api_error_kind() {
            // The room's power levels reserve call memberships (Element
            // creates rooms with `org.matrix.msc3401.call.member` at 0).
            Some(ErrorKind::Forbidden) => {
                CoreError::Call("you are not allowed to join calls in this room".into())
            }
            _ => CoreError::Call("could not publish the call membership".into()),
        })
}

/// Leaves: fires the delayed leave at once, or (without delayed events, or
/// if that failed) sends the empty membership.
pub(super) async fn remove(room: &Room, state_key: &str, delay_id: Option<String>) {
    let fired = match delay_id {
        Some(id) => update_delayed_leave(&room.client(), id, UpdateAction::Send)
            .await
            .is_ok(),
        None => false,
    };
    if !fired && send_membership(room, state_key, json!({})).await.is_err() {
        tracing::warn!("could not remove the call membership; it expires by itself");
    }
}

/// What [`keep_membership`] needs.
pub(super) struct OwnMembership {
    pub(super) client: Client,
    pub(super) room: Room,
    pub(super) state_key: String,
    pub(super) user_id: String,
    pub(super) device_id: String,
    pub(super) service_url: String,
    /// A voice or a video call, kept on every refresh.
    pub(super) intent: CallIntent,
    pub(super) delay_id: Arc<Mutex<Option<String>>>,
    /// Woken by `watch_participants` when our membership is gone from the
    /// room state.
    pub(super) gone: Arc<Notify>,
}

impl OwnMembership {
    /// Restarts the delayed leave so it never fires while we are alive. If
    /// the server lost it (restart, cancelled) or already sent it, schedules
    /// a new one.
    async fn restart_delayed_leave(&self) {
        let mut guard = self.delay_id.lock().await;
        let Some(id) = guard.clone() else {
            // The homeserver has no delayed events.
            return;
        };
        match update_delayed_leave(&self.client, id, UpdateAction::Restart).await {
            Ok(_) => {}
            Err(e) if matches!(e.client_api_error_kind(), Some(ErrorKind::NotFound)) => {
                if let Ok(Some(new)) =
                    schedule_delayed_leave(&self.client, &self.room, &self.state_key).await
                {
                    *guard = Some(new);
                }
            }
            // Network trouble: try again next tick; the server sends the
            // leave if we stay away longer than the delay.
            Err(_) => {}
        }
    }

    /// Sends our membership, valid for `validity` from `created_ts` (`None`:
    /// from now, a fresh membership).
    async fn publish(&self, created_ts: Option<u64>, validity: Duration) -> Result<(), CoreError> {
        let content = membership::join_content(
            &self.user_id,
            &self.device_id,
            &self.service_url,
            self.room.room_id().as_str(),
            created_ts,
            validity,
            self.intent,
        );
        send_membership(&self.room, &self.state_key, content)
            .await
            .map(|_| ())
    }

    /// Joins again from scratch, as at the start (js-sdk
    /// `MembershipManager`, "missing own membership"): a new delayed leave,
    /// the old one cancelled first (left alone, it would fire and remove the
    /// new membership), then a fresh membership.
    async fn rejoin(&self) -> Result<(), CoreError> {
        let mut guard = self.delay_id.lock().await;
        if let Some(old) = guard.clone() {
            match update_delayed_leave(&self.client, old, UpdateAction::Cancel).await {
                Ok(_) => {}
                Err(e) if matches!(e.client_api_error_kind(), Some(ErrorKind::NotFound)) => {}
                Err(_) => return Err(CoreError::Call("could not cancel the call leave".into())),
            }
            *guard = None;
        }
        *guard = schedule_delayed_leave(&self.client, &self.room, &self.state_key).await?;
        drop(guard);
        self.publish(None, MEMBERSHIP_EXPIRES).await
    }
}

/// Keeps our membership in the room state: restarts the delayed leave,
/// re-publishes the membership with a longer validity before it expires,
/// and joins again from scratch when it is gone (the server sent our leave
/// during a stall longer than the delay, or it expired).
pub(super) async fn keep_membership(own: OwnMembership, joined_ms: u64) {
    let refresh_after =
        |validity: Duration| tokio::time::Instant::now() + validity.saturating_sub(REFRESH_MARGIN);
    let mut ticker = tokio::time::interval(DELAYED_LEAVE_RESTART);
    ticker.tick().await;
    let mut created_ts = joined_ms;
    let mut validity = MEMBERSHIP_EXPIRES;
    let mut refresh_at = refresh_after(validity);
    let mut rejoin = false;
    loop {
        tokio::select! {
            _ = ticker.tick() => own.restart_delayed_leave().await,
            () = tokio::time::sleep_until(refresh_at) => {
                validity += MEMBERSHIP_EXPIRES;
                // Counted from the first join, so the next refresh comes
                // a whole validity later.
                refresh_at = refresh_after(MEMBERSHIP_EXPIRES);
                if own.publish(Some(created_ts), validity).await.is_err() {
                    tracing::warn!("could not refresh the call membership");
                }
            }
            () = own.gone.notified() => rejoin = true,
        }
        if rejoin {
            let started = now_ms();
            match own.rejoin().await {
                Ok(()) => {
                    rejoin = false;
                    created_ts = started;
                    validity = MEMBERSHIP_EXPIRES;
                    refresh_at = refresh_after(validity);
                }
                // Tried again on the next tick.
                Err(_) => tracing::warn!("could not join the call again; retrying"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;

    use matrix_sdk::authentication::matrix::MatrixSession;
    use matrix_sdk::ruma::{owned_device_id, owned_user_id};
    use matrix_sdk::{SessionMeta, SessionTokens};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    const TOKEN: &str = "sentinel-access-token-x";
    const V2: &str = "/_matrix/client/unstable/org.matrix.msc4140/delayed_events/d1";

    /// One request the fake homeserver saw.
    #[derive(Debug, Clone)]
    struct Seen {
        method: String,
        path: String,
        authorization: Option<String>,
        body: String,
    }

    /// A homeserver on localhost: answers `/versions`, records every
    /// delayed-event request, and answers those with the scripted `(status, body)` in
    /// order (then `200 {}`).
    struct Fake {
        url: String,
        seen: Arc<StdMutex<Vec<Seen>>>,
    }

    impl Fake {
        async fn start(script: Vec<(u16, &'static str)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
            let url = format!("http://{}", listener.local_addr().expect("addr"));
            let seen = Arc::new(StdMutex::new(Vec::new()));
            let script = Arc::new(StdMutex::new(
                script
                    .into_iter()
                    .collect::<std::collections::VecDeque<_>>(),
            ));
            let log = seen.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((mut socket, _)) = listener.accept().await else {
                        return;
                    };
                    let (log, script) = (log.clone(), script.clone());
                    tokio::spawn(async move {
                        let Some(request) = read_request(&mut socket).await else {
                            return;
                        };
                        let (status, body) = if request.path.ends_with("/versions") {
                            (200, r#"{"versions":["v1.11"]}"#)
                        } else if !request.path.contains("/delayed_events/") {
                            // The client's own background requests.
                            (404, r#"{"errcode":"M_NOT_FOUND","error":"x"}"#)
                        } else {
                            log.lock().expect("log").push(request);
                            script
                                .lock()
                                .expect("script")
                                .pop_front()
                                .unwrap_or((200, "{}"))
                        };
                        let head = format!(
                            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = socket.write_all(head.as_bytes()).await;
                    });
                }
            });
            Self { url, seen }
        }

        fn seen(&self) -> Vec<Seen> {
            self.seen.lock().expect("log").clone()
        }

        /// A client logged in with [`TOKEN`].
        async fn client(&self) -> Client {
            let client = Client::builder()
                .homeserver_url(&self.url)
                .build()
                .await
                .expect("client");
            client
                .restore_session(MatrixSession {
                    meta: SessionMeta {
                        user_id: owned_user_id!("@alice:example.org"),
                        device_id: owned_device_id!("DEVICEID"),
                    },
                    tokens: SessionTokens {
                        access_token: TOKEN.to_owned(),
                        refresh_token: None,
                    },
                })
                .await
                .expect("restore");
            client
        }
    }

    async fn read_request(socket: &mut tokio::net::TcpStream) -> Option<Seen> {
        let mut data = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = socket.read(&mut chunk).await.ok()?;
            if n == 0 {
                return None;
            }
            data.extend_from_slice(&chunk[..n]);
            let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") else {
                continue;
            };
            let head = String::from_utf8_lossy(&data[..end]).into_owned();
            let length = header(&head, "content-length")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            while data.len() < end + 4 + length {
                let n = socket.read(&mut chunk).await.ok()?;
                if n == 0 {
                    return None;
                }
                data.extend_from_slice(&chunk[..n]);
            }
            let mut first = head.lines().next()?.split(' ');
            return Some(Seen {
                method: first.next()?.to_owned(),
                path: first.next()?.to_owned(),
                authorization: header(&head, "authorization"),
                body: String::from_utf8_lossy(&data[end + 4..end + 4 + length]).into_owned(),
            });
        }
    }

    fn header(head: &str, name: &str) -> Option<String> {
        head.lines().skip(1).find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case(name).then(|| v.trim().to_owned())
        })
    }

    #[tokio::test]
    async fn every_update_carries_the_access_token() {
        for (action, word) in [
            (UpdateAction::Restart, "restart"),
            (UpdateAction::Cancel, "cancel"),
            (UpdateAction::Send, "send"),
        ] {
            let fake = Fake::start(vec![]).await;
            let client = fake.client().await;
            update_delayed_leave(&client, "d1".to_owned(), action)
                .await
                .expect("accepted");
            let seen = fake.seen();
            assert_eq!(seen.len(), 1, "{word}: {seen:?}");
            assert_eq!(seen[0].method, "POST");
            assert_eq!(seen[0].path, format!("{V2}/{word}"));
            assert_eq!(
                seen[0].authorization.as_deref(),
                Some("Bearer sentinel-access-token-x")
            );
        }
    }

    #[tokio::test]
    async fn a_server_without_the_action_path_gets_the_older_request() {
        let fake = Fake::start(vec![(404, r#"{"errcode":"M_UNRECOGNIZED","error":"x"}"#)]).await;
        let client = fake.client().await;
        update_delayed_leave(&client, "d1".to_owned(), UpdateAction::Restart)
            .await
            .expect("the older path accepts");
        let seen = fake.seen();
        assert_eq!(seen.len(), 2, "{seen:?}");
        assert_eq!(seen[1].method, "POST");
        assert_eq!(
            seen[1].path,
            "/_matrix/client/unstable/org.matrix.msc4140/delayed_events/d1"
        );
        assert_eq!(
            seen[1].authorization.as_deref(),
            Some("Bearer sentinel-access-token-x")
        );
        let body: Value = serde_json::from_str(&seen[1].body).expect("json body");
        assert_eq!(body, json!({"action": "restart"}));
    }

    #[tokio::test]
    async fn an_unknown_delay_id_is_not_retried_on_the_older_path() {
        let fake = Fake::start(vec![(404, r#"{"errcode":"M_NOT_FOUND","error":"x"}"#)]).await;
        let client = fake.client().await;
        let err = update_delayed_leave(&client, "d1".to_owned(), UpdateAction::Restart)
            .await
            .expect_err("not found");
        assert!(matches!(
            err.client_api_error_kind(),
            Some(ErrorKind::NotFound)
        ));
        assert_eq!(fake.seen().len(), 1);
    }
}
