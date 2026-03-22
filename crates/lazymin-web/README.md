# lazymin-web (WASM)

Browser build of lazymin using [Trunk](https://trunkrs.dev/) and [xterm.js](https://xtermjs.org/).

## Prerequisites

- Rust target: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev/) and a `wasm-bindgen` matching the toolchain (Trunk installs or uses a compatible version)

## Build

From this directory:

```sh
trunk build --release
```

Output is written to the workspace `dist/` directory (see `Trunk.toml`).

From the repo root you can also run [`scripts/build-web.sh`](../../scripts/build-web.sh) — it invokes `trunk build --release` from this crate.

### Why Trunk (not a hand-written `import './lazymin_web.js'`)

Trunk emits **hashed** bundle names (e.g. `lazymin-web-<hash>.js`) and injects the wasm init script. The app reads `window.wasmBindings` (set by Trunk) for `run_game` / `on_terminal_data`. A static import of `lazymin_web.js` 404s after build and the browser reports MIME type `text/html` for the failed request.

### Chrome console: `integrity` / preload

Trunk may add SRI on preloads; Chrome can log that integrity is ignored for some preload types. It is harmless.

### xterm: `Terminal is not defined` in a module script

The xterm bundle sets `window.Terminal`. Inside `<script type="module">`, bare `Terminal` is not in scope; the page uses `window.Terminal` explicitly.

### `trunk` fails with `--no-color`

If your environment sets `NO_COLOR` to something other than `true`/`false`, unset it: `unset NO_COLOR` then run `trunk` again.

## Cloudflare Pages

1. Connect the repo and set the project root to **this crate** (`crates/lazymin-web`) or run the build from the repo root with a custom command.
2. **Build command:** `cd crates/lazymin-web && trunk build --release`
3. **Build output directory:** `dist` (relative to repo root: `dist` at the workspace root when `dist = "../../dist"`).
4. Add the custom hostname `terminal.smith-c.com` under **Custom domains** for the Pages project and complete DNS (CNAME or delegated) as prompted by Cloudflare.

If the build runs from the repo root, use:

- **Build command:** `trunk build --release --config crates/lazymin-web/Trunk.toml` (adjust if your Trunk version expects a different flag), or `cd crates/lazymin-web && trunk build --release`
- **Output directory:** `dist`

## Local preview

```sh
trunk serve --release
```

Open the URL Trunk prints (usually `http://127.0.0.1:8080`).
