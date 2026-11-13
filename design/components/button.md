# Button

The action button, in four variants: primary, secondary, ghost, danger.

- **Caller provides:** a label in sentence case with no period, an optional 16px stroke icon before the label, the action.
- **primary**: fill `violet`, label `on-violet`. One per view: join a call, send, sign in. Hover: `violet-bright`.
- **secondary**: fill `surface-raised`, 1px `line-strong` outline, label `ink`. Everyday actions. Hover: `surface-overlay`.
- **ghost**: no fill, label `ink-muted` (hover `ink` on `surface-overlay`). Toolbar actions.
- **danger**: fill `surface-raised`, outline and label `live`. Leave call, delete. The label names the action ("Leave call"), never "OK".
- **disabled**: 45% opacity, no hover.
- Height 36px, horizontal padding `space.4`, radius `radius.sm`, label `body-strong` at 14px. Focus: `focus` 2px, offset 2px.
- Don't: two primary buttons side by side; a violet button for a destructive action.
