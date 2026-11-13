# Voice message

A voice message in the timeline: play button, waveform, duration.

- **Caller provides:** playing/paused state, progress 0–1, waveform samples (~44 bars, from `org.matrix.msc1767.audio` `waveform` when present), duration.
- Bubble: `surface-raised`, radius `radius.md`, padding `space.3`, max width 360px.
- Play button: 36px `violet` disc, icon `on-violet`. Accessible label "Play" / "Pause".
- Waveform: 3px bars, `radius.xs`, 2px gap; played part `violet-bright`, the rest `line-strong`.
- Duration in `meta`, no wrapping: `0:18 / 0:42` while playing, the total otherwise.
- Don't: animate the waveform in a loop; set the duration in a proportional font (digits jump).
