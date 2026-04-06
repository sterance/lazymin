#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
unset NO_COLOR 2>/dev/null || true
if command -v magick >/dev/null 2>&1; then
  "$root/scripts/render-pwa-icons.sh"
fi
cd "$root/crates/lazymin-web"
trunk build --release
echo "built $root/dist (serve with: cd $root && trunk serve --release  or  npx serve dist)"
