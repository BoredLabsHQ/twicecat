# Call tile

One participant in a voice room or call: avatar, name, audio state.

- **Caller provides:** display name, avatar (image or initial), state `speaking` / `muted` / silent, and the live audio level to drive the glow.
- Tile: `surface-raised`, radius `radius.md`, 168×128 by default; tiles are spaced `space.6` in the call grid.
- **speaking:** `glow-speaking` shadow (violet ring + glow) and the word "speaking" in `violet-bright`. Glow opacity follows the level (attack 60ms, release 240ms).
- **muted:** crossed-out mic icon + the word "muted" in `live`. Never colour alone.
- Name chip: `void` background, `body-strong` 13px, bottom-left, `space.2` inset.
- Avatar without image: initial in Chakra Petch 700 on `violet-deep`, text `ink`.
- Don't: pulse a silent tile; tint the whole tile.
