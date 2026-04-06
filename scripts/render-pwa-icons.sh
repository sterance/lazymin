#!/usr/bin/env bash
set -euo pipefail
web="$(cd "$(dirname "$0")/../crates/lazymin-web" && pwd)"
dir="$web/icons"
if ! command -v magick >/dev/null 2>&1; then
  echo "magick (ImageMagick) is required" >&2
  exit 1
fi
cd "$dir"
# solid backgrounds for maskable-style icons; stroke matches each svg theme
magick -background '#111111' -density 256 icon-dark-mode.svg -resize 192x192 icon-192-dark.png
magick -background '#111111' -density 256 icon-dark-mode.svg -resize 512x512 icon-512-dark.png
magick -background '#ffffff' -density 256 icon-light-mode.svg -resize 192x192 icon-192-light.png
magick -background '#ffffff' -density 256 icon-light-mode.svg -resize 512x512 icon-512-light.png
# classic .ico for legacy clients and default /favicon.ico fetch (single file; dark theme art)
magick -background '#111111' -density 256 icon-dark-mode.svg -resize 256x256 -define icon:auto-resize=16,32,48 "$web/favicon.ico"
