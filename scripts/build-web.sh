#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
unset NO_COLOR 2>/dev/null || true
cd "$root/crates/lazymin-web"
trunk build --release
echo "built $root/dist (serve with: cd $root && trunk serve --release  or  npx serve dist)"
