# Cloudflare Pages (terminal.smith-c.com)

## Prerequisites

- `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev/): `cargo install trunk`

## Build

From the repository root:

```sh
cd crates/lazymin-web && trunk build --release
```

Artifacts are emitted to the workspace `dist/` directory (see `crates/lazymin-web/Trunk.toml`).

## Pages project

1. In the Cloudflare dashboard: **Workers & Pages** > **Create** > **Pages** > **Connect to Git** (or upload `dist/` manually).
2. Configure the build:
   - **Root directory** (if offered): `crates/lazymin-web` *or* keep the repo root and use the commands below.
   - **Build command:** `cd crates/lazymin-web && trunk build --release`
   - **Build output directory:** `dist` (path relative to the repository root when `dist = "../../dist"` in `Trunk.toml`).
3. **Custom domains:** add `terminal.smith-c.com` and follow Cloudflare DNS instructions (typically a CNAME to `*.pages.dev`).

## Manual deploy (wrangler)

With [Wrangler](https://developers.cloudflare.com/workers/wrangler/) authenticated:

```sh
cd crates/lazymin-web && trunk build --release
npx wrangler pages deploy ../../dist --project-name=<your-project>
```

Replace `<your-project>` with your Pages project name.
