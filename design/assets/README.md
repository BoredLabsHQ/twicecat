# Assets

All marks are vector SVGs with outlined text (no font needed); PNGs are renders of the SVG of the same name. Never redraw, recolour or re-export the mark from memory — use these files.

## logos/

| File | Use |
| --- | --- |
| `twicecat-mark.svg` | Colour mark, transparent. On `void`, `surface`, `surface-raised`. Docs logo (mdBook, docs.rs). |
| `twicecat-mark-512.png` | PNG of the colour mark, transparent. |
| `twicecat-mark-mono-light.svg` | Single ink `phosphor` #f3eeff: dark backgrounds, tray, one-colour print. |
| `twicecat-mark-mono-dark.svg` | Single ink `void` #0a0812: light backgrounds. |
| `twicecat-wordmark.svg` | "twice" `phosphor` + "cat" `violet-bright`, Chakra Petch 700 outlined. |
| `twicecat-lockup.svg`, `-1024.png` | Mark + wordmark for dark backgrounds: about screen, splash. |
| `twicecat-lockup-on-light.svg` | Mark + wordmark for light backgrounds (`void` + `violet-deep`). |

## icons/

| File | Use |
| --- | --- |
| `app-icon.svg` | App icon (rounded square, scanlines, glow). Linux hicolor `scalable/apps`; source for `.icns` / `.ico`. |
| `app-icon-512.png` | hicolor `512x512/apps`, Flathub / AppStream. |
| `apple-touch-icon-180.png` | Website apple-touch-icon. |
| `favicon.svg`, `favicon-32.png`, `favicon-48.png` | Simplified mark (5 thick slices, no cut), readable at 16px. Use below 24px. |
| `avatar.svg`, `avatar-512.png` | Square on `void`; the mark fits the inscribed circle, so it survives round crops (Matrix, GitHub, Mastodon). |

## banners/

`void`, scanlines, glowing mark on the left, wordmark, tagline, `matrix · voice first · written in rust` in mono, violet rule at the bottom. Keep anything new inside the central 80% of the height (Mastodon and X overlay the avatar bottom-left).

| File | Use |
| --- | --- |
| `social-preview-1280x640` | GitHub / Forgejo repository social preview. |
| `og-image-1200x630` | Open Graph for the website and docs. |
| `header-1500x500` | Mastodon / Bluesky / X header. |
| `readme-banner-1600x400` | Top of the repository README. |

## emoji/

Matrix custom emoji pack (MSC2545, `im.ponies.room_emotes`), 128×128 PNG with SVG sources.

| Shortcode | Meaning |
| --- | --- |
| `:twicecat:` | The simplified mark. |
| `:dejavu:` | Only the echo — "saw that one go by already" (duplicate message, question already asked). |
| `:speaking:` | Cat + three violet waves — "taking the floor". |
| `:listening:` | Cat with violet headphones — "listening". |
