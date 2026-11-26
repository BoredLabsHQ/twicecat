# Room item

A row of the room list: avatar, name, preview, time, unread count.

- **Caller provides:** room name, avatar (image, initial or `#`), last-event preview, timestamp, unread count, whether a voice room is live.
- Name `body-strong` `ink`, preview `small` `ink-muted`, time `meta`; everything truncated to one line.
- Active room: background `violet-wash`. Hover: `surface-overlay`. No coloured left bar.
- Unread: pill `violet` + `on-violet`, mono 12px, `radius.pill`. Mention: same pill prefixed with "@".
- Live voice room: `violet-bright` dot + "voice · N in room" in `meta`.
- Padding `space.2` × `space.3`, radius `radius.sm`, focus ring inset 2px.
- Don't: bold the name for unread in addition to the pill; show more than one pill.
