#!/usr/bin/env bash
# Renders Twicecat's icon from its SVG (packaging/icons/io.github.ga_lep.Twicecat.svg)
# into what the packages install: a freedesktop hicolor tree
# (packaging/icons/hicolor/<N>x<N>/apps/<app id>.png and scalable/apps/<app id>.svg)
# and a Windows .ico. The outputs are committed; run this after changing the SVG.
# The macOS .icns is made from the PNGs when the DMG is packaged.
#
# Needs rsvg-convert (librsvg) or Inkscape, and ImageMagick 7 (magick) or 6 (convert).
#
#   scripts/make-icons.sh
set -euo pipefail
cd "$(dirname "$0")/.."

app_id=io.github.ga_lep.Twicecat
svg=packaging/icons/$app_id.svg
out=packaging/icons
if command -v rsvg-convert >/dev/null; then
    render() { rsvg-convert -w "$1" -h "$1" "$svg" -o "$2"; }
elif command -v inkscape >/dev/null; then
    render() { inkscape "$svg" -w "$1" -h "$1" -o "$2" >/dev/null 2>&1; }
else
    echo "error: neither rsvg-convert nor inkscape is installed" >&2; exit 1
fi
if command -v magick >/dev/null; then
    imagemagick=(magick)
elif command -v convert >/dev/null; then
    imagemagick=(convert)
else
    echo "error: ImageMagick is not installed" >&2; exit 1
fi

rm -rf "$out/hicolor"
mkdir -p "$out/hicolor/scalable/apps"
cp "$svg" "$out/hicolor/scalable/apps/$app_id.svg"
for size in 16 24 32 48 64 128 256 512; do
    dir="$out/hicolor/${size}x${size}/apps"
    mkdir -p "$dir"
    render "$size" "$dir/$app_id.png"
done

# Windows: one .ico with the sizes Explorer and the taskbar ask for.
ico_sizes=()
for size in 16 24 32 48 64 256; do
    ico_sizes+=("$out/hicolor/${size}x${size}/apps/$app_id.png")
done
"${imagemagick[@]}" "${ico_sizes[@]}" "$out/$app_id.ico"

echo "rendered $out/hicolor and $out/$app_id.ico from $svg" >&2
