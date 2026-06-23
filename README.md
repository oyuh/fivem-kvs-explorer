# KVS Explorer

A client-side explorer/editor for the **FiveM KVS** store. The KVS is a [LevelDB] database;
this app reads and writes it entirely in the browser via a Rust core compiled to
WebAssembly, with a SvelteKit front-end. No server, no upload — the bytes never leave the
machine.

---

## The data format

FiveM's resource Key-Value Store (`SetResourceKvp*` / `GetResourceKvp*`) is a **LevelDB**
database. On the client it lives at `%appdata%\CitizenFX\kvs` (FiveM mounts it at the
virtual path `fxd:/kvs/`, or `fxd:/kvs_cl2/` on CL2). The folder is a standard LevelDB
directory: `CURRENT`, `MANIFEST-*`, `*.ldb` tables, `*.log` (WAL), `LOG`, `LOCK`.

- **Key namespacing** — resource entries are stored as `res:<resource>:<key>`; resource
  version metadata as `rv:<…>`. Grouping by resource is a prefix split.
- **Value encoding** — values are **msgpack** (the native packs the Lua value:
  `SetResourceKvp` → string, `SetResourceKvpInt` → int, `SetResourceKvpFloat` → 32-bit
  float). Resources that store tables typically `json.encode` them into a string value.
- **Compression** — LevelDB's default Snappy block compression.

## Architecture

```
┌──────────────────────────── Browser (Chromium) ───────────────────────────┐
│  SvelteKit SPA (Svelte 5 runes, adapter-static)                            │
│    components/ ── ResourceTree │ EntryList (virtualized) │ ValueDetail     │
│                   JsonEditor │ ImportDialog │ AddEntryDialog               │
│        │ FileSystemDirectoryHandle (File System Access API)                │
│        ▼                                                                    │
│  lib/kvs.ts ── reads file bytes ⇄ marshals to/from wasm ⇄ writes back      │
│        │ [{ name, bytes: Uint8Array }]        { changed, deleted }         │
│        ▼                                              ▲                     │
│  kvs-core (Rust → wasm, wasm-bindgen)                 │                     │
│    MemEnv (in-memory Env) ← dragged files             │                     │
│    rusty-leveldb DB::open over MemEnv                   │                    │
│    rmpv msgpack decode/encode · res:/rv: key parsing                       │
│    put/delete → leveldb rewrites files in MemEnv → diff vs load snapshot   │
└────────────────────────────────────────────────────────────────────────────┘
```

### `kvs-core` (Rust)

`crate-type = ["rlib", "cdylib"]`. Two layers:

- **`src/lib.rs`** — platform-agnostic core, also exercised by native `cargo test`:
  - `open_from_files(&[DbFile]) -> (DB, SharedEnv)` writes the folder's bytes into a
    `rusty_leveldb::MemEnv` at `db/<name>` and `DB::open`s over it (`create_if_missing =
    false`). `SharedEnv = Rc<Box<dyn Env>>` is shared between our file I/O and the DB so
    leveldb's mutations are visible when we read the files back.
  - `for_each_entry` iterates via the `LdbIterator` `advance()`/`current()` protocol.
  - `parse_key` splits the `res:`/`rv:` namespace; `decode_value`/`encode_value` wrap
    `rmpv`; `value_type`/`value_preview`/`looks_like_json` summarize values.
  - `snapshot_files(&SharedEnv)` reads every file currently in the MemEnv (skipping `LOCK`).
- **`src/wasm.rs`** (`#[cfg(target_arch = "wasm32")]`) — the `wasm-bindgen` boundary. A
  `KvsDb` object holds the open `DB`, the `SharedEnv`, and a `HashMap<name, content-hash>`
  of the files as originally loaded (for diffing). Marshalling is manual `js_sys`
  (`Uint8Array` in/out, object shaping) plus `serde-wasm-bindgen` for the lightweight row
  list.

`KvsDb` surface:

| method | purpose |
| --- | --- |
| `new KvsDb(files)` | open from `[{name, bytes}]` |
| `entries()` | lightweight rows `{rawKey, namespace, resource, key, valueType, preview, …}` |
| `get(rawKey)` | full decode `{type, value, hex, byteLen}` |
| `get_raw(rawKey)` | exact stored bytes (verbatim copy / import) |
| `put(rawKey, value, kind)` | `kind ∈ {string,int,float,bool,raw}`; re-encodes msgpack (`raw` = bytes verbatim) |
| `delete(rawKey)` | tombstone |
| `export_changes()` | diff MemEnv vs load snapshot → `{changed:[{name,bytes}], deleted:[name]}` |
| `mark_saved()` | reset the diff baseline after a successful disk write |

### Write model

Edits never hand-craft SST/log files. `put`/`delete` go through `rusty-leveldb`, which
appends to the WAL and may compact, producing a **valid new file set inside the MemEnv**.
On save, `export_changes()` content-hashes the current MemEnv files against the load-time
snapshot and returns only the deltas; `kvs.ts` writes changed files via
`FileSystemFileHandle.createWritable()` and `removeEntry()`s deletions. Because leveldb
generated the files, the on-disk DB stays consistent. A backup zip (`fflate`) of the
original bytes is downloaded before the first write.

### `web` (SvelteKit)

Pure SPA: `ssr = false`, `adapter-static` with an `index.html` fallback — builds to static
files. `lib/kvs.ts` is the only place that touches the File System Access API:
`getAsFileSystemHandle()` (drag) / `showDirectoryPicker({mode})` (button), `dir.entries()`
to read bytes, `createWritable()`/`removeEntry()` to write back. The wasm module
(`--target web`) is initialized lazily. The UI virtualizes the key list and lazily fully
decodes a value only on selection.

## Browser constraints (hard limits, not bugs)

- **File System Access API ⇒ Chromium only.** Firefox/Safari don't implement it. **Brave**
  ships it disabled behind `brave://flags/#file-system-access-api`.
- **Chromium blocklists AppData.** `%APPDATA%` and `%LOCALAPPDATA%` (and *all* descendants)
  are `kBlockAllChildren` in Chromium's File System Access blocklist, so the kvs folder
  **cannot be opened in place**. The home directory is `kDontBlockChildren`, so copy the
  folder to e.g. the Desktop, edit, and copy it back. The app surfaces this in the UI.
- **Secure context** required (`localhost` or HTTPS).

## Requirements

- A Chromium browser (Chrome / Edge / Brave-with-flag / Opera).
- [Bun] (package manager + runner).
- Rust **stable ≥ 1.87** (rusty-leveldb 4.x uses `is_multiple_of`), the
  `wasm32-unknown-unknown` target, and [wasm-pack] — only needed to rebuild the wasm core.

## Commands (from the repo root)

| Command | Action |
| --- | --- |
| `bun run setup` | install web dependencies |
| `bun run dev` | build the wasm core (debug) + start Vite |
| `bun run build` | build the release wasm + static site → `web/build/` |
| `bun run check` | `svelte-check` |
| `bun run test` | `cargo test` (native) + the wasm-binding smoke test |
| `bun run wasm` / `wasm:dev` | rebuild just the wasm core into `web/src/lib/wasm` |
| `bun run fixture` | regenerate the LevelDB test/sample fixture |

```bash
bun run setup && bun run dev   # then open the localhost URL in Chrome/Edge
```

## Testing

- **`kvs-core/tests/compat.rs`** — the cross-implementation gate. The fixture is a real
  LevelDB produced by **C++ LevelDB** (via Node `classic-level`), so it exercises the
  genuine on-disk format and Snappy-compressed `.ldb` tables. Asserts: pure-Rust
  `rusty-leveldb` reads it, `res:`/`rv:` parsing, msgpack type round-trips, and a
  put/add/delete → `snapshot_files` → reopen round-trip.
- **`tools/wasm-smoke.cjs`** — runs the actual `KvsDb` bindings (nodejs-target wasm, same
  code the browser runs) against the fixture: `entries()` shape, typed `get()`, `put`,
  `delete`, `export_changes()` reload round-trip, and the `get_raw` → `put 'raw'` import
  path.
- **`tools/gen-fixture/`** — seeds the fixture with `classic-level` + `@msgpack/msgpack`
  (string/int/float plus JSON-as-string values, sized to force Snappy table flushes).

## Deploy (Vercel)

Vercel's build image has no Rust toolchain, so the compiled wasm core is **committed** at
`web/src/lib/wasm/`; Vercel builds only the Rust-free SvelteKit app. [vercel.json](vercel.json)
sets `installCommand`/`buildCommand`/`outputDirectory`.

1. Import the repo at [vercel.com/new](https://vercel.com/new).
2. Leave **Root Directory** `./` and **Framework Preset** Other.
3. Deploy. Every push to `master` auto-deploys; the dev-only sample data is stripped from
   production builds.

CI ([.github/workflows/deploy.yml](.github/workflows/deploy.yml)) rebuilds the wasm and runs
`bun run build` + `bun run check` + `cargo test` on every push/PR.

> **After changing `kvs-core`**, rebuild and commit the wasm so Vercel ships it:
> `bun run wasm && git add web/src/lib/wasm && git commit`.

## Project layout

```
kvs-core/                 Rust → wasm
  src/lib.rs              portable core (native cargo test)
  src/wasm.rs             wasm-bindgen KvsDb
  tests/compat.rs         C++-LevelDB compatibility + round-trip tests
web/
  src/lib/kvs.ts          File System Access ⇄ wasm bridge
  src/lib/json.ts         no-dep JSON tokenizer/highlighter/validator
  src/lib/components/     ResourceTree, EntryList, ValueDetail, JsonEditor,
                          ImportDialog, AddEntryDialog, Splitter, Icon, …
  src/lib/wasm/           committed wasm-pack output (rebuilt by `bun run wasm`)
  src/routes/+page.svelte app shell + state
tools/                    fixture generator + wasm smoke test
vercel.json               Vercel build config (Rust-free)
```

[LevelDB]: https://github.com/google/leveldb
[Bun]: https://bun.sh
[wasm-pack]: https://rustwasm.github.io/wasm-pack/
