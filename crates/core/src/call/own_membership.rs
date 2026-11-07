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

use std::future::Future;
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
use tokio::sync::{Mutex, Notify, mpsc};
use tokio::time::Instant;

use super::media::MediaNotice;
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
    try_schedule_delayed_leave(client, room, state_key)
        .await
        .map_err(|_| CoreError::Call("could not schedule the call leave".into()))
}

/// [`schedule_delayed_leave`], with the error as the server sent it.
async fn try_schedule_delayed_leave(
    client: &Client,
    room: &Room,
    state_key: &str,
) -> Result<Option<String>, matrix_sdk::HttpError> {
    let empty: Raw<_> = Raw::new(&json!({}))
        .expect("an empty object serializes")
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
        Err(e) => Err(e),
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
    send_membership_raw(room, state_key, content)
        .await
        .map_err(|e| match e.client_api_error_kind() {
            // The room's power levels reserve call memberships (Element
            // creates rooms with `org.matrix.msc3401.call.member` at 0).
            Some(ErrorKind::Forbidden) => {
                CoreError::Call("you are not allowed to join calls in this room".into())
            }
            _ => CoreError::Call("could not publish the call membership".into()),
        })
}

/// [`send_membership`], with the error as the server sent it.
async fn send_membership_raw(
    room: &Room,
    state_key: &str,
    content: Value,
) -> Result<String, matrix_sdk::Error> {
    room.send_state_event_raw(CALL_MEMBER_EVENT, state_key, content)
        .await
        .map(|r| r.event_id.to_string())
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

/// Gives up on getting our membership back once it has been gone this long
/// (more than one `DELAYED_LEAVE` plus a sync round trip).
pub(super) const REJOIN_GIVE_UP: Duration = Duration::from_secs(30);

/// Every try to join again may take at least this long, even past the end
/// of the window.
const MIN_ATTEMPT: Duration = Duration::from_secs(3);

/// How long a restart of the delayed leave may take (it is sent every
/// `DELAYED_LEAVE_RESTART`, and a stuck one must not block the loop).
const RESTART_LIMIT: Duration = Duration::from_secs(10);

/// Between two tries to join again.
const REJOIN_RETRY: Duration = Duration::from_millis(500);

/// How a request about our membership failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    /// The server refuses us (no or unknown token, forbidden): trying again
    /// does not help.
    Fatal,
    /// Anything else (network, server trouble): may pass.
    Retry,
}

/// What to do after a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    Retry,
    GiveUp,
}

/// Decides, after each failed try to join again, whether to keep trying.
#[derive(Debug, Clone, Copy)]
struct RejoinPolicy {
    /// When we found our membership gone.
    gone_at: Instant,
    give_up_after: Duration,
}

impl RejoinPolicy {
    /// When a try started at `now` must be over: the SDK retries server
    /// errors and rate limits for minutes per request, so the end of the
    /// window has to be enforced here. A timeout counts as [`Failure::Retry`].
    fn attempt_deadline(&self, now: Instant) -> Instant {
        (self.gone_at + self.give_up_after).max(now + MIN_ATTEMPT)
    }

    fn after_failure(&self, failure: Failure, now: Instant) -> Next {
        match failure {
            Failure::Fatal => Next::GiveUp,
            Failure::Retry if now.saturating_duration_since(self.gone_at) >= self.give_up_after => {
                Next::GiveUp
            }
            Failure::Retry => Next::Retry,
        }
    }
}

/// Runs a try, giving up on it at `deadline` (a failure to retry).
async fn within<T>(
    deadline: Instant,
    attempt: impl Future<Output = Result<T, Failure>>,
) -> Result<T, Failure> {
    tokio::time::timeout_at(deadline, attempt)
        .await
        .unwrap_or(Err(Failure::Retry))
}

/// Classifies an error kind sent by the server (`None`: no Matrix error).
fn failure_of(kind: Option<&ErrorKind>) -> Failure {
    match kind {
        Some(ErrorKind::MissingToken | ErrorKind::UnknownToken(_) | ErrorKind::Forbidden) => {
            Failure::Fatal
        }
        _ => Failure::Retry,
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
    /// Where we report that our membership is gone, back, or lost for good.
    pub(super) notices: mpsc::Sender<MediaNotice>,
    /// How long to try to join again before giving up.
    pub(super) give_up_after: Duration,
}

impl OwnMembership {
    /// Restarts the delayed leave so it never fires while we are alive. If
    /// the server lost it (restart, cancelled) or already sent it, schedules
    /// a new one. `Err` is the failure that stopped it: [`Failure::Fatal`]
    /// means the server will send the leave and we could not join again.
    async fn restart_delayed_leave(&self) -> Result<(), Failure> {
        let mut guard = self.delay_id.lock().await;
        let Some(id) = guard.clone() else {
            // The homeserver has no delayed events.
            return Ok(());
        };
        match update_delayed_leave(&self.client, id, UpdateAction::Restart).await {
            Ok(_) => Ok(()),
            Err(e) if matches!(e.client_api_error_kind(), Some(ErrorKind::NotFound)) => {
                match try_schedule_delayed_leave(&self.client, &self.room, &self.state_key).await {
                    Ok(Some(new)) => {
                        *guard = Some(new);
                        Ok(())
                    }
                    Ok(None) => Ok(()),
                    Err(e) => Err(failure_of(e.client_api_error_kind())),
                }
            }
            // Network trouble: try again next tick; the server sends the
            // leave if we stay away longer than the delay.
            Err(e) => Err(failure_of(e.client_api_error_kind())),
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

    /// [`Self::publish`], given up after `limit` (a failure to retry).
    async fn refresh(
        &self,
        created_ts: u64,
        validity: Duration,
        limit: Duration,
    ) -> Result<(), Failure> {
        within(Instant::now() + limit, async {
            self.publish(Some(created_ts), validity)
                .await
                .map_err(|_| Failure::Retry)
        })
        .await
    }

    /// Joins again from scratch, as at the start (js-sdk
    /// `MembershipManager`, "missing own membership"): a new delayed leave,
    /// the old one cancelled first (left alone, it would fire and remove the
    /// new membership), then a fresh membership.
    async fn rejoin(&self) -> Result<(), Failure> {
        let mut guard = self.delay_id.lock().await;
        if let Some(old) = guard.clone() {
            match update_delayed_leave(&self.client, old, UpdateAction::Cancel).await {
                Ok(_) => {}
                Err(e) if matches!(e.client_api_error_kind(), Some(ErrorKind::NotFound)) => {}
                Err(e) => return Err(failure_of(e.client_api_error_kind())),
            }
            *guard = None;
        }
        *guard = try_schedule_delayed_leave(&self.client, &self.room, &self.state_key)
            .await
            .map_err(|e| failure_of(e.client_api_error_kind()))?;
        drop(guard);
        let content = membership::join_content(
            &self.user_id,
            &self.device_id,
            &self.service_url,
            self.room.room_id().as_str(),
            None,
            MEMBERSHIP_EXPIRES,
            self.intent,
        );
        send_membership_raw(&self.room, &self.state_key, content)
            .await
            .map(|_| ())
            .map_err(|e| failure_of(e.client_api_error_kind()))
    }
}

/// Keeps our membership in the room state: restarts the delayed leave,
/// re-publishes the membership with a longer validity before it expires,
/// and joins again from scratch when it is gone (the server sent our leave
/// during a stall longer than the delay, or it expired).
///
/// While it is gone the engine holds the microphone, camera and screen share (`MembershipGone`, then
/// `MembershipBack`). If it cannot come back (see [`RejoinPolicy`]), or the
/// server refuses to keep the delayed leave away, it sends `MembershipLost`
/// and returns: the call ends.
pub(super) async fn keep_membership(own: OwnMembership, joined_ms: u64) {
    let refresh_after =
        |validity: Duration| Instant::now() + validity.saturating_sub(REFRESH_MARGIN);
    let mut ticker = tokio::time::interval(DELAYED_LEAVE_RESTART);
    ticker.tick().await;
    let mut created_ts = joined_ms;
    let mut validity = MEMBERSHIP_EXPIRES;
    let mut refresh_at = refresh_after(validity);
    // Set while our membership is gone.
    let mut gone: Option<RejoinPolicy> = None;
    let mut retry_at = Instant::now();
    loop {
        let rejoining = gone.is_some();
        tokio::select! {
            _ = ticker.tick() => {
                let deadline = Instant::now() + RESTART_LIMIT;
                if within(deadline, own.restart_delayed_leave()).await == Err(Failure::Fatal) {
                    tracing::warn!("the homeserver refuses to keep the call leave away");
                    let _ = own.notices.send(MediaNotice::MembershipLost { refused: true }).await;
                    return;
                }
            }
            () = tokio::time::sleep_until(refresh_at) => {
                validity += MEMBERSHIP_EXPIRES;
                // Counted from the first join, so the next refresh comes
                // a whole validity later.
                refresh_at = refresh_after(MEMBERSHIP_EXPIRES);
                // Bounded: a stuck refresh must not delay the rest of the loop
                // (the SDK retries server errors for minutes).
                if own.refresh(created_ts, validity, RESTART_LIMIT).await.is_err() {
                    tracing::warn!("could not refresh the call membership");
                }
            }
            () = own.gone.notified() => {
                if gone.is_none() {
                    gone = Some(RejoinPolicy {
                        gone_at: Instant::now(),
                        give_up_after: own.give_up_after,
                    });
                    retry_at = Instant::now();
                    let _ = own.notices.send(MediaNotice::MembershipGone).await;
                }
            }
            () = tokio::time::sleep_until(retry_at), if rejoining => {}
        }
        let Some(policy) = gone else {
            continue;
        };
        if Instant::now() < retry_at {
            continue;
        }
        let started = now_ms();
        match within(policy.attempt_deadline(Instant::now()), own.rejoin()).await {
            Ok(()) => {
                gone = None;
                created_ts = started;
                validity = MEMBERSHIP_EXPIRES;
                refresh_at = refresh_after(validity);
                let _ = own.notices.send(MediaNotice::MembershipBack).await;
            }
            Err(failure) => match policy.after_failure(failure, Instant::now()) {
                Next::Retry => {
                    tracing::warn!(?failure, "could not join the call again; retrying");
                    retry_at = Instant::now() + REJOIN_RETRY;
                }
                Next::GiveUp => {
                    tracing::warn!(?failure, "could not join the call again; giving up");
                    let refused = failure == Failure::Fatal;
                    let _ = own
                        .notices
                        .send(MediaNotice::MembershipLost { refused })
                        .await;
                    return;
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;

    use matrix_sdk::authentication::matrix::MatrixSession;
    use matrix_sdk::ruma::{owned_device_id, owned_room_id, owned_user_id};
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

        /// Like [`Fake::start`], but `answer` replies to every request except
        /// `/versions`, and all of them are recorded.
        async fn start_with(
            answer: impl Fn(&Seen) -> (u16, String) + Send + Sync + 'static,
        ) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
            let url = format!("http://{}", listener.local_addr().expect("addr"));
            let seen = Arc::new(StdMutex::new(Vec::new()));
            let (log, answer) = (seen.clone(), Arc::new(answer));
            tokio::spawn(async move {
                loop {
                    let Ok((mut socket, _)) = listener.accept().await else {
                        return;
                    };
                    let (log, answer) = (log.clone(), answer.clone());
                    tokio::spawn(async move {
                        let Some(request) = read_request(&mut socket).await else {
                            return;
                        };
                        let (status, body) = if request.path.ends_with("/versions") {
                            (200, r#"{"versions":["v1.11"]}"#.to_owned())
                        } else {
                            let reply = answer(&request);
                            log.lock().expect("log").push(request);
                            reply
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

        /// Like [`Fake::client`], but with the SDK's default request config,
        /// which retries server errors for a long time.
        async fn client_with_retries(&self) -> Client {
            let client = Client::builder()
                .homeserver_url(&self.url)
                .build()
                .await
                .expect("client");
            self.log_in(&client).await;
            client
        }

        async fn log_in(&self, client: &Client) {
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
        }

        fn seen(&self) -> Vec<Seen> {
            self.seen.lock().expect("log").clone()
        }

        /// A client logged in with [`TOKEN`].
        async fn client(&self) -> Client {
            let client = Client::builder()
                .homeserver_url(&self.url)
                // The SDK retries server errors for a long time; the
                // policy under test is ours.
                .request_config(matrix_sdk::config::RequestConfig::new().disable_retry())
                .build()
                .await
                .expect("client");
            self.log_in(&client).await;
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

    #[test]
    fn errors_a_retry_cannot_fix_give_up_at_once() {
        let policy = RejoinPolicy {
            gone_at: Instant::now(),
            give_up_after: Duration::from_secs(30),
        };
        assert_eq!(
            policy.after_failure(Failure::Fatal, policy.gone_at),
            Next::GiveUp
        );
    }

    #[test]
    fn other_failures_retry_until_the_window_has_passed() {
        let gone_at = Instant::now();
        let policy = RejoinPolicy {
            gone_at,
            give_up_after: Duration::from_secs(30),
        };
        let at = |s| gone_at + Duration::from_secs(s);
        assert_eq!(policy.after_failure(Failure::Retry, at(0)), Next::Retry);
        assert_eq!(policy.after_failure(Failure::Retry, at(29)), Next::Retry);
        assert_eq!(policy.after_failure(Failure::Retry, at(30)), Next::GiveUp);
        assert_eq!(policy.after_failure(Failure::Retry, at(90)), Next::GiveUp);
    }

    #[test]
    fn only_token_and_permission_errors_are_fatal() {
        assert_eq!(failure_of(Some(&ErrorKind::MissingToken)), Failure::Fatal);
        assert_eq!(failure_of(Some(&ErrorKind::Forbidden)), Failure::Fatal);
        assert_eq!(
            failure_of(Some(&ErrorKind::UnknownToken(Default::default()))),
            Failure::Fatal
        );
        assert_eq!(failure_of(Some(&ErrorKind::NotFound)), Failure::Retry);
        assert_eq!(failure_of(None), Failure::Retry);
    }

    const SYNC: &str = r#"{"next_batch":"s1","rooms":{"join":{"!r:example.org":{}}}}"#;

    /// Answers a request as a homeserver that refuses everything with
    /// `refuse` (if any) and accepts otherwise.
    fn answer(request: &Seen, refuse: Option<(u16, &'static str)>) -> (u16, String) {
        if request.path.contains("/sync") {
            return (200, SYNC.to_owned());
        }
        if let Some((status, body)) = refuse {
            return (status, body.to_owned());
        }
        if request.path.contains("/delayed_events/") {
            (200, "{}".to_owned())
        } else if request.path.contains("delay") {
            (200, r#"{"delay_id":"new"}"#.to_owned())
        } else {
            (200, r#"{"event_id":"$e"}"#.to_owned())
        }
    }

    const MISSING_TOKEN: (u16, &str) = (401, r#"{"errcode":"M_MISSING_TOKEN","error":"x"}"#);
    const SERVER_ERROR: (u16, &str) = (500, r#"{"errcode":"M_UNKNOWN","error":"x"}"#);

    struct Keeper {
        fake: Fake,
        gone: Arc<Notify>,
        notices: mpsc::Receiver<MediaNotice>,
        task: tokio::task::JoinHandle<()>,
    }

    /// Runs `keep_membership` against a homeserver that refuses with
    /// `refuse`.
    async fn keeper(refuse: Option<(u16, &'static str)>, give_up_after: Duration) -> Keeper {
        keeper_with(refuse, give_up_after, false).await
    }

    /// [`keeper`], with the SDK's own retries of server errors on or off.
    async fn keeper_with(
        refuse: Option<(u16, &'static str)>,
        give_up_after: Duration,
        sdk_retries: bool,
    ) -> Keeper {
        let (own, fake, gone, notices) = own_with(refuse, give_up_after, sdk_retries).await;
        let task = tokio::spawn(keep_membership(own, now_ms()));
        Keeper {
            fake,
            gone,
            notices,
            task,
        }
    }

    async fn own_with(
        refuse: Option<(u16, &'static str)>,
        give_up_after: Duration,
        sdk_retries: bool,
    ) -> (
        OwnMembership,
        Fake,
        Arc<Notify>,
        mpsc::Receiver<MediaNotice>,
    ) {
        // The setup (login, first sync) is answered well; the refusals start
        // once the keeper runs, so the SDK's retries only hit its requests.
        let armed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let on = armed.clone();
        let fake = Fake::start_with(move |r| {
            let refuse = refuse.filter(|_| on.load(std::sync::atomic::Ordering::SeqCst));
            answer(r, refuse)
        })
        .await;
        let client = if sdk_retries {
            fake.client_with_retries().await
        } else {
            fake.client().await
        };
        client
            .sync_once(matrix_sdk::config::SyncSettings::default())
            .await
            .expect("sync");
        let room = client
            .get_room(&owned_room_id!("!r:example.org"))
            .expect("room");
        armed.store(true, std::sync::atomic::Ordering::SeqCst);
        let gone = Arc::new(Notify::new());
        let (notices_tx, notices) = mpsc::channel(8);
        let own = OwnMembership {
            client,
            room,
            state_key: "_@alice:example.org_DEVICEID".to_owned(),
            user_id: "@alice:example.org".to_owned(),
            device_id: "DEVICEID".to_owned(),
            service_url: "https://sfu.example.org".to_owned(),
            intent: CallIntent::Audio,
            delay_id: Arc::new(Mutex::new(Some("d1".to_owned()))),
            gone: gone.clone(),
            notices: notices_tx,
            give_up_after,
        };
        (own, fake, gone, notices)
    }

    async fn next(notices: &mut mpsc::Receiver<MediaNotice>) -> MediaNotice {
        tokio::time::timeout(Duration::from_secs(5), notices.recv())
            .await
            .expect("a notice in time")
            .expect("channel open")
    }

    #[tokio::test]
    async fn a_refused_token_ends_the_rejoin_at_once() {
        let mut k = keeper(Some(MISSING_TOKEN), Duration::from_secs(1)).await;
        k.gone.notify_one();
        let started = Instant::now();
        assert_eq!(next(&mut k.notices).await, MediaNotice::MembershipGone);
        assert_eq!(
            next(&mut k.notices).await,
            MediaNotice::MembershipLost { refused: true }
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        tokio::time::timeout(Duration::from_secs(2), k.task)
            .await
            .expect("the task finishes")
            .expect("no panic");
    }

    #[tokio::test]
    async fn server_errors_are_retried_until_the_window_has_passed() {
        let mut k = keeper(Some(SERVER_ERROR), Duration::from_secs(1)).await;
        k.gone.notify_one();
        let started = Instant::now();
        assert_eq!(next(&mut k.notices).await, MediaNotice::MembershipGone);
        assert_eq!(
            next(&mut k.notices).await,
            MediaNotice::MembershipLost { refused: false }
        );
        assert!(started.elapsed() >= Duration::from_secs(1));
        let attempts = k
            .fake
            .seen()
            .iter()
            .filter(|r| r.path.ends_with("/cancel"))
            .count();
        assert!(attempts > 1, "{attempts} attempts");
        tokio::time::timeout(Duration::from_secs(2), k.task)
            .await
            .expect("the task finishes")
            .expect("no panic");
    }

    #[tokio::test]
    async fn the_window_holds_with_the_sdk_retrying_server_errors() {
        // The SDK's default config retries a 500 for minutes per request;
        // the window must still end the rejoin.
        let mut k = keeper_with(Some(SERVER_ERROR), Duration::from_secs(1), true).await;
        k.gone.notify_one();
        let started = Instant::now();
        assert_eq!(next(&mut k.notices).await, MediaNotice::MembershipGone);
        let lost = tokio::time::timeout(Duration::from_secs(12), k.notices.recv())
            .await
            .expect("lost in time");
        assert_eq!(lost, Some(MediaNotice::MembershipLost { refused: false }));
        // About 4 s expected; the margin keeps a loaded machine from
        // flaking, while without the per-attempt deadline the SDK retries
        // for minutes (and the 12 s wait above fails).
        assert!(started.elapsed() < Duration::from_secs(10));
        tokio::time::timeout(Duration::from_secs(2), k.task)
            .await
            .expect("the task finishes")
            .expect("no panic");
    }

    #[tokio::test]
    async fn a_refresh_the_sdk_keeps_retrying_is_given_up_at_its_limit() {
        let (own, _fake, _gone, _notices) =
            own_with(Some(SERVER_ERROR), Duration::from_secs(30), true).await;
        let started = Instant::now();
        let done = tokio::time::timeout(
            Duration::from_secs(8),
            own.refresh(0, MEMBERSHIP_EXPIRES, Duration::from_secs(1)),
        )
        .await
        .expect("the refresh ends at its limit");
        assert_eq!(done, Err(Failure::Retry));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn an_attempt_gets_the_rest_of_the_window_but_at_least_a_minimum() {
        let gone_at = Instant::now();
        let policy = RejoinPolicy {
            gone_at,
            give_up_after: Duration::from_secs(30),
        };
        let at = |s| gone_at + Duration::from_secs(s);
        assert_eq!(policy.attempt_deadline(at(0)), at(30));
        assert_eq!(policy.attempt_deadline(at(10)), at(30));
        assert_eq!(policy.attempt_deadline(at(29)), at(29) + MIN_ATTEMPT);
        assert_eq!(policy.attempt_deadline(at(60)), at(60) + MIN_ATTEMPT);
    }

    #[tokio::test]
    async fn a_membership_that_comes_back_is_reported_and_kept() {
        let mut k = keeper(None, Duration::from_secs(1)).await;
        k.gone.notify_one();
        assert_eq!(next(&mut k.notices).await, MediaNotice::MembershipGone);
        assert_eq!(next(&mut k.notices).await, MediaNotice::MembershipBack);
        tokio::time::sleep(Duration::from_millis(1500)).await;
        assert!(k.notices.try_recv().is_err());
        assert!(!k.task.is_finished());
        k.task.abort();
    }

    #[tokio::test]
    async fn a_refused_restart_ends_the_call() {
        let mut k = keeper(Some(MISSING_TOKEN), Duration::from_secs(30)).await;
        let lost = tokio::time::timeout(Duration::from_secs(8), k.notices.recv())
            .await
            .expect("a notice in time");
        assert_eq!(lost, Some(MediaNotice::MembershipLost { refused: true }));
        tokio::time::timeout(Duration::from_secs(2), k.task)
            .await
            .expect("the task finishes")
            .expect("no panic");
    }
}
