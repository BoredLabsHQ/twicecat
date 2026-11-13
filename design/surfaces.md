# Surfaces

Every place twicecat exists outside its own window, and the file to use. Default background: `void`.

| Where | Surface | Size / format | File |
| --- | --- | --- | --- |
| App | Linux `.desktop` icon (hicolor scalable) | SVG | `assets/icons/app-icon.svg` |
| App | hicolor 512, Flathub | PNG 512 | `assets/icons/app-icon-512.png` |
| App | macOS `.icns`, Windows `.ico` | derive 1024 → 16 at packaging | `assets/icons/app-icon.svg` |
| App | Tray, notifications | mono SVG | `assets/logos/twicecat-mark-mono-light.svg` |
| App | About / splash | SVG | `assets/logos/twicecat-lockup.svg` |
| App | `twicecat --version` | text | `twicecat` in ANSI 256 colour 141 (≈ `violet-bright`) |
| Web | Favicon | SVG + PNG 32/48 | `assets/icons/favicon.*` |
| Web | Apple touch icon | PNG 180 | `assets/icons/apple-touch-icon-180.png` |
| Web | Open Graph link preview | PNG 1200×630 | `assets/banners/og-image-1200x630.png` |
| Web | Docs logo (mdBook, docs.rs `html_logo_url`) | SVG | `assets/logos/twicecat-mark.svg` |
| Forge | README banner | PNG 1600×400 | `assets/banners/readme-banner-1600x400.png` |
| Forge | Repository social preview | PNG 1280×640 | `assets/banners/social-preview-1280x640.png` |
| Forge | Org / repo avatar | PNG 512 | `assets/icons/avatar-512.png` |
| Forge | crates.io | no image: README + `homepage` | — |
| Matrix | Space and official room avatars | PNG 512, round crop | `assets/icons/avatar-512.png` |
| Matrix | Custom emoji pack | PNG 128 | `assets/emoji/emoji-*.png` |
| Socials | Mastodon / Bluesky / X header | PNG 1500×500 | `assets/banners/header-1500x500.png` |
| Socials | Avatar | PNG 512 | `assets/icons/avatar-512.png` |

Still to produce: app screenshots (AppStream, website, README) once the UI exists, framed on `void` at 16:10; multi-size `.icns` / `.ico`; a UI icon set.
