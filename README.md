# KVS Explorer

Browse and edit your **FiveM KVS** (key-value store) right in the browser — drag the
folder in, explore every key grouped by resource, edit values, and save back to disk.
**Nothing is ever uploaded**; all the work happens locally.

A FiveM "kvs folder" (`%appdata%\CitizenFX\kvs`) is actually a [LevelDB] database with
msgpack-encoded values. So under the hood this is a LevelDB reader/writer compiled to
**Rust → WebAssembly**, with a **SvelteKit** UI on top. The heavy lifting (parsing tables,
Snappy decompression, msgpack) runs in wasm; the UI stays snappy via list virtualization.

## Requirements

- **A Chromium browser** — Chrome or Edge work out of the box. Brave ships the
  [File System Access API] disabled; enable it at `brave://flags/#file-system-access-api`,
  relaunch, and reload. Firefox/Safari aren't supported (no File System Access API).
- **[Bun]** — package manager + script runner.
- **Rust** (stable) + **[wasm-pack]** — only needed to build the wasm core.
  - `rustup target add wasm32-unknown-unknown`
  - `cargo install wasm-pack`

## Quick start

```bash
bun run setup     # install the web app's dependencies
bun run dev       # build the wasm core (dev) + start the dev server
```

Open the printed `localhost` URL in Chrome/Edge. Browsers block access to `AppData`, so
**copy your `kvs` folder to your Desktop first**, then drag that copy in (or click
“Explore sample data” to try bundled demo data). When done, copy the edited folder back
into `%appdata%\CitizenFX`.

> 💡 **Close FiveM first** so it isn't writing to the folder while you edit. A backup zip
> is offered automatically before your first save, and your original stays untouched until
> you copy the edited folder back.

## Commands (run from the repo root)

| Command | What it does |
| --- | --- |
| `bun run setup` | Install the web app dependencies. |
| `bun run dev` | Build the wasm core (debug) and start the Vite dev server. |
| `bun run build` | Build the wasm core (release) and the static site → `web/build/`. |
| `bun run preview` | Serve the production build locally. |
| `bun run check` | Type-check the SvelteKit app (`svelte-check`). |
| `bun run test` | Run the Rust tests **and** the wasm-binding smoke test. |
| `bun run wasm` / `wasm:dev` | (Re)build just the wasm core into `web/src/lib/wasm`. |
| `bun run fixture` | Regenerate the sample/test LevelDB fixture. |

The output of `bun run build` in `web/build/` is plain static files — host it anywhere
(Vercel, Cloudflare Pages, GitHub Pages, Netlify, …). No server, no special headers required.
The dev-only **sample data** is stripped from production builds.

## Deploy (Vercel via GitHub Actions)

Because the app compiles Rust → wasm, the build runs in **GitHub Actions** (which has the
Rust toolchain) and the prebuilt static output is shipped to **Vercel**. The workflow lives
in [.github/workflows/deploy.yml](.github/workflows/deploy.yml): on every push to `master`
it installs Rust + wasm-pack + Bun, runs `bun run build`, type-checks, runs the tests, and
deploys.

One-time setup:

1. Create a Vercel project (Framework preset: **Other** — it's ignored, we deploy prebuilt).
   Locally: `npm i -g vercel && vercel link` — this writes `.vercel/project.json` with your
   org and project IDs.
2. Add three **GitHub repository secrets** (Settings → Secrets and variables → Actions):
   - `VERCEL_TOKEN` — from Vercel → Account Settings → Tokens
   - `VERCEL_ORG_ID` — from `.vercel/project.json`
   - `VERCEL_PROJECT_ID` — from `.vercel/project.json`

Until those secrets exist, the build/test steps still run on every push; the deploy step is
skipped automatically. (Don't also enable Vercel's own Git integration — Vercel's build
image has no Rust toolchain, so let Actions do the building.)

## Project layout

```
kvs-explorer/
├─ kvs-core/        Rust crate → wasm (LevelDB I/O, key parsing, msgpack)
│  ├─ src/lib.rs    portable core (also runs in native `cargo test`)
│  ├─ src/wasm.rs   wasm-bindgen `KvsDb` object the browser drives
│  └─ tests/        compatibility + write round-trip tests
├─ web/             SvelteKit app (Svelte 5, adapter-static SPA)
│  └─ src/lib/      kvs.ts (FSA ⇄ wasm bridge) + components
├─ tools/           fixture generator + wasm smoke test
└─ package.json     root command hub (this is what you run)
```

## How saving works (and why it's safe)

Edits are made against an in-memory copy of the LevelDB. On **Save to disk**, LevelDB
itself produces the new files; we diff them against what was loaded and write only the
changed files back through the File System Access API (removing any LevelDB compacted
away). We never hand-craft database files, and a backup zip is taken before the first
write.

[LevelDB]: https://github.com/google/leveldb
[File System Access API]: https://developer.mozilla.org/en-US/docs/Web/API/File_System_API
[Bun]: https://bun.sh
[wasm-pack]: https://rustwasm.github.io/wasm-pack/
