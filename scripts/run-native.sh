#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
# tui needs a tty (-it). audio inside docker is often limited or host-specific.
exec docker compose run --rm -it dev cargo run -p lazymin-native "$@"
