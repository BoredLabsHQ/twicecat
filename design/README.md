# twicecat design

twicecat is a Matrix client written in Rust, built around audio: voice rooms, voice messages, calls. The name comes from the black cat that walks by twice — déjà vu, a glitch in the Matrix. For us that cat passing twice is also an **echo**: sound coming back. The whole identity is one idea: **a signal and its echo**.

Tagline: *A Matrix client that listens.*

This folder is the source of truth for anything visual. Read it before touching UI.

|File|What it is|
|-|-|
|`tokens.json`|All colours, spacing, radii, type and shadows ([DTCG](https://www.designtokens.org/) format). **Edit values here only.**|
|`generated/theme.rs`, `generated/tokens.css`|Generated from `tokens.json` by `scripts/gen\_tokens.py`. Never edit by hand.|
|`components/\*.md`|Guidelines per UI component.|
|`assets/`|Logos, icons, banners, emoji. See `assets/README.md` for which file goes where.|
|`fonts/`|The three typefaces (woff2, Latin subset) and their OFL licences.|
|`surfaces.md`|Every place twicecat appears outside its window, with sizes and files.|
|`scripts/check\_design.py`|CI guardrails: contrast floors and no colour literals in Rust code.|

## Principles

1. **Sound first.** What makes noise in the UI is audio: who's speaking, what's recording, what's playing. The `violet` accent is reserved for those states and the primary action.
2. **A signal, an echo.** A crisp element in `phosphor`/`ink`, its double in `violet`, offset and sliced into horizontal bands. That's the logo, the waveform, the banner pattern.
3. **Terminal, not cyberpunk.** A near-black `void` tinted violet, subtle scanlines, monospace for data. No green code rain, no saturated neon, no gradients.
4. **Calm.** One vivid colour on screen at a time, outside status signals.

## Voice and copy

* The name is always **`twicecat`**: one word, all lowercase, even at the start of a sentence. Never "TwiceCat", "Twice Cat" or "TWICECAT".
* UI and public presence (GitHub, website, socials) are in English.
* Sentence case everywhere ("Start voice room", not "Start Voice Room"). No period on buttons.
* Tone: dry, precise, one wink at a time, never two. ✓ "Mic check. You sound fine." ✗ "Woohoo! Your awesome mic is ready! 🎉"
* No emoji in interface text or documentation (icons come from the icon set). Users can use emoji freely in their messages; the twicecat emoji pack is for community rooms.
* Use Matrix vocabulary: "room" (never "channel"), "voice room", "voice message", "homeserver".
* Numbers and durations are set in `meta` (mono): `0:42`, `48 kHz`, `@sajida:boredlabs.co`.

## Colour

* Backgrounds, deepest to highest: `void` → `surface` → `surface-raised` → `surface-overlay`. Depth comes from these steps, not from shadows.
* Text: `ink` for content, `ink-muted` for secondary, `ink-faint` for tertiary (never below 12px).
* Accent: `violet` as a fill (primary button, unread badge, speaking ring, playhead) with `on-violet` on top. For accent **text** (links, mentions) use `violet-bright`, never `violet`.
* `phosphor` is the logo cat's colour. Not for UI text.
* Status: `live` (recording, muted, error), `ok` (connected, verified), `warn` (unverified, unstable). Always paired with a word or icon: they are never told apart by hue alone.
* Selection: `violet-wash` row background. No coloured left border.
* Outlines: `line` to separate, `line-strong` for controls (≥ 3:1).

## Typography

* **Chakra Petch** (`font.family.display`): titles, wordmark, banners. Its clipped corners echo the glitch slices. Never for paragraphs.
* **IBM Plex Sans** (`sans`): all UI text — `body`, `body-strong`, `small`.
* **JetBrains Mono** (`mono`): `code`, and `meta` for durations, timestamps, Matrix IDs, bitrates. Numbers that change during a call are always mono.
* One size per role from the `type` scale; don't invent in-between sizes.

## Space, radii, shapes

* 4px grid: `space.1` to `space.12`.
* Radii: `radius.xs` for anything bar-like (waveform, tags, slices), `sm` for controls, `md` for bubbles and tiles, `lg` for modals, `pill` for counters, play button and avatars.
* Brand pattern: **horizontal bars** offset by `space.1`/`space.2` steps. Use in banners, empty states and loading — never behind running text.
* Scanlines: `surface-raised` lines on `void`, every other line, on brand visuals only (icon, avatar, banners, splash).

## Motion and audio

* Whoever is speaking gets `glow-speaking`; its intensity follows the audio level, attack 60ms, release 240ms.
* The glitch (echo slices shifting) is a rare event: reconnecting, déjà vu (duplicate message), app launch. 200ms max, never looping, off under reduced-motion settings.
* Waveform: 3px vertical bars, `radius.xs`, 2px gap; played part `violet-bright`, the rest `line-strong`.

## Focus and accessibility

* Keyboard focus: `focus` outline, 2px solid, 2px offset, on every interactive element.
* Text holds 4.5:1 on the backgrounds named in its token description; control outlines and icons 3:1. `scripts/check\_design.py` enforces this.
* An audio state never relies on colour alone: crossed-out mic + the word "muted", dot + "REC".

## Logo

* The mark: a seated cat in `phosphor` (the signal) and its double in `violet`, offset right and sliced (the echo, the déjà vu). One clean cut through the body, three stray pixels on the right; nothing sticks out on the left.
* Clear space: one ear-height all around. Minimum size: 24px for the full mark; below that use `assets/icons/favicon.svg`.
* Don't: recolour outside the supplied variants, add a gradient or outline, separate the cat from its echo, put the echo on the left, place the colour mark on a light background (use `twicecat-lockup-on-light.svg`), redraw the mark.

## Iconography

* UI icons: 1.5px stroke, rounded joins, 20px grid, `ink-muted` at rest, `ink` on hover, `violet-bright` when active. Start from a free outline set (Lucide, Phosphor Regular); none is locked in yet.
* Tray icon: `assets/logos/twicecat-mark-mono-light.svg`, single ink.

## Using the tokens in code

Rust — pull the generated module into the UI crate (adjust the relative path):

```rust
#\[path = "../../../design/generated/theme.rs"]
pub mod theme;

// iced:  iced::Color::from\_rgba8(c.r, c.g, c.b, c.a as f32 / 255.0)
// egui:  egui::Color32::from\_rgba\_unmultiplied(c.r, c.g, c.b, c.a)
// Slint: slint::Color::from\_argb\_u8(c.a, c.r, c.g, c.b)
let c = theme::color::VIOLET;
```

Write those conversions once in a small `theme` adapter for the toolkit; everything else uses `theme::color::\*` etc. Load the fonts from `design/fonts/`.

Web — link `design/generated/tokens.css` and use `var(--violet)`, `var(--space-4)`, `class="body"`…

## Changing the design

1. Edit `tokens.json` (or a doc here).
2. Run `python3 design/scripts/gen\_tokens.py`.
3. Run `python3 design/scripts/check\_design.py`.
4. Commit `tokens.json` and `generated/` together.

