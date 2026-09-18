# KVS Explorer

A browser-based explorer and editor for the FiveM KVS store. The KVS is a [LevelDB]
database, so this app opens it with a Rust core compiled to WebAssembly and a SvelteKit UI
on top. No server, no upload step. The bytes never leave your machine.

## The data format

FiveM's resource Key-Value Store (the `SetResourceKvp*` / `GetResourceKvp*` natives) is a
LevelDB database. On the client it lives at `%appdata%\CitizenFX\kvs` (FiveM mounts it at
the virtual path `fxd:/kvs/`, or `fxd:/kvs_cl2/` on CL2). The folder is a normal LevelDB
directory: `CURRENT`, `MANIFEST-*`, `*.ldb` tables, `*.log` (the WAL), `LOG`, `LOCK`.

A few things worth knowing about what's inside:

- Keys are namespaced. Resource entries are stored as `res:<resource>:<key>`, and
  resource version metadata as `rv:<...>`. Grouping by resource is a prefix split.
- Values are msgpack. The native packs the Lua value directly: `SetResourceKvp` gives you
  a string, `SetResourceKvpInt` an int, `SetResourceKvpFloat` a 32-bit float. Resources
  that store tables usually `json.encode` them into a string value first.
- Blocks are compressed with Snappy, LevelDB's default.

## Architecture

```text
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

Built with `crate-type = ["rlib", "cdylib"]`, split into two layers:

`src/lib.rs` is the platform-agnostic core, which native `cargo test` also exercises:

- `open_from_files(&[DbFile]) -> (DB, SharedEnv)` writes the folder's bytes into a
  `rusty_leveldb::MemEnv` at `db/<name>` and runs `DB::open` over it with
  `create_if_missing = false`. `SharedEnv` is an `Rc<Box<dyn Env>>` shared between our
  file I/O and the DB, so leveldb's mutations are visible when we read the files back.
- `for_each_entry` iterates via the `LdbIterator` `advance()`/`current()` protocol.
- `parse_key` splits the `res:`/`rv:` namespace. `decode_value` and `encode_value` wrap
  `rmpv`, and `value_type`/`value_preview`/`looks_like_json` summarize values for the UI.
- `snapshot_files(&SharedEnv)` reads every file currently in the MemEnv (skipping `LOCK`).

`src/wasm.rs` (gated on `#[cfg(target_arch = "wasm32")]`) is the `wasm-bindgen` boundary.
A `KvsDb` object holds the open `DB`, the `SharedEnv`, and a `HashMap<name, content-hash>`
of the files as originally loaded, which is what diffing compares against later.
Marshalling is manual `js_sys` (`Uint8Array` in and out, object shaping) plus
`serde-wasm-bindgen` for the lightweight row list.

The `KvsDb` surface:

| method | purpose |
| --- | --- |
| `new KvsDb(files)` | open from `[{name, bytes}]` |
| `entries()` | lightweight rows `{rawKey, namespace, resource, key, valueType, preview, ...}` |
| `get(rawKey)` | full decode `{type, value, hex, byteLen}` |
| `get_raw(rawKey)` | exact stored bytes (verbatim copy / import) |
| `put(rawKey, value, kind)` | `kind` is one of `string`, `int`, `float`, `bool`, `raw`; re-encodes msgpack (`raw` stores bytes verbatim) |
| `delete(rawKey)` | tombstone |
| `export_changes()` | diff MemEnv vs load snapshot, returns `{changed:[{name,bytes}], deleted:[name]}` |
| `mark_saved()` | reset the diff baseline after a successful disk write |

### Write model

Edits never hand-craft SST or log files. `put` and `delete` go through `rusty-leveldb`,
which appends to the WAL and may compact, so what you end up with is a valid new file set
inside the MemEnv. On save, `export_changes()` content-hashes the current MemEnv files
against the load-time snapshot and returns only the deltas. `kvs.ts` then writes changed
files via `FileSystemFileHandle.createWritable()` and calls `removeEntry()` for deletions.
Because leveldb itself generated the files, the on-disk DB stays consistent. Before the
first write, the app downloads a backup zip of the original bytes, built with `fflate`.

### `web` (SvelteKit)

A pure SPA: `ssr = false` and `adapter-static` with an `index.html` fallback, so it builds
to plain static files. The **Try the demo** button opens the fixture in
`web/static/sample/` through the same code path as a real folder, minus the directory
handle, which leaves the session read-only. `lib/kvs.ts` is the only place that touches the File System Access
API: `getAsFileSystemHandle()` for drag and drop, `showDirectoryPicker({mode})` for the
button, `dir.entries()` to read bytes, and `createWritable()`/`removeEntry()` to write
back. The app loads the wasm module (built with `--target web`) lazily, virtualizes the key
list, and only fully decodes a value when you select it.

## Browser constraints

The three limits below come from the browser, not from this app:

- The File System Access API means Chromium only. Firefox and Safari don't implement it,
  and Brave ships it disabled behind `brave://flags/#file-system-access-api`.
- Chromium blocklists AppData. `%APPDATA%` and `%LOCALAPPDATA%` (and all of their
  descendants) are `kBlockAllChildren` in Chromium's File System Access blocklist, so you
  can't open the kvs folder in place. The home directory is `kDontBlockChildren`, so the
  workflow is to copy the folder somewhere like the Desktop, edit it, then copy it back.
  The start screen lists those steps.
- The page has to run in a secure context (`localhost` or HTTPS).

## Requirements

- A Chromium browser (Chrome, Edge, Brave with the flag, Opera).
- [Bun] as the package manager and script runner.
- To rebuild the wasm core only: Rust stable 1.87 or newer (rusty-leveldb 4.x uses
  `is_multiple_of`), the `wasm32-unknown-unknown` target, and [wasm-pack].

## Commands (from the repo root)

| Command | Action |
| --- | --- |
| `bun run setup` | install web dependencies |
| `bun run dev` | build the wasm core (debug) + start Vite |
| `bun run build` | build the release wasm + static site into `web/build/` |
| `bun run check` | `svelte-check` |
| `bun run test` | `cargo test` (native) + the wasm-binding smoke test |
| `bun run wasm` / `wasm:dev` | rebuild only the wasm core into `web/src/lib/wasm` |
| `bun run fixture` | regenerate the LevelDB test/sample fixture |

```bash
bun run setup && bun run dev   # then open the localhost URL in Chrome/Edge
```

## Testing

- `kvs-core/tests/compat.rs` is the cross-implementation gate. The fixture is a real
  LevelDB produced by C++ LevelDB (via Node's `classic-level`), so it exercises the
  genuine on-disk format and Snappy-compressed `.ldb` tables. It asserts that pure-Rust
  `rusty-leveldb` can read it, that `res:`/`rv:` parsing works, that msgpack types
  round-trip, and that a put/add/delete followed by `snapshot_files` and a reopen also
  round-trips.
- `tools/wasm-smoke.cjs` runs the actual `KvsDb` bindings (nodejs-target wasm, same code
  the browser runs) against the fixture: the `entries()` shape, typed `get()`, `put`,
  `delete`, the `export_changes()` reload round-trip, and the `get_raw` into `put 'raw'`
  import path.
- `tools/gen-fixture/` seeds the fixture with `classic-level` + `@msgpack/msgpack`
  (string/int/float plus JSON-as-string values, sized to force Snappy table flushes).

## Deploy (Vercel)

Vercel's build image has no Rust toolchain, so the repo commits the compiled wasm core at
`web/src/lib/wasm/` and Vercel builds only the Rust-free SvelteKit app. [vercel.json](vercel.json)
sets `installCommand`, `buildCommand`, and `outputDirectory`.

1. Import the repo at [vercel.com/new](https://vercel.com/new).
2. Leave Root Directory as `./` and Framework Preset as Other.
3. Deploy. Every push to `master` auto-deploys.

CI ([.github/workflows/deploy.yml](.github/workflows/deploy.yml)) rebuilds the wasm and runs
`bun run build` + `bun run check` + `cargo test` on every push and PR.

> Heads up: after changing `kvs-core`, rebuild and commit the wasm so Vercel ships it:
> `bun run wasm && git add web/src/lib/wasm && git commit`.

## Project layout

```text
kvs-core/                 Rust → wasm
  src/lib.rs              portable core (native cargo test)
  src/wasm.rs             wasm-bindgen KvsDb
  tests/compat.rs         C++-LevelDB compatibility + round-trip tests
web/
  static/sample/          demo LevelDB served over HTTP (the Try the demo button)
  src/lib/kvs.ts          File System Access ⇄ wasm bridge
  src/lib/json.ts         no-dep JSON tokenizer/highlighter/validator
  src/lib/components/     ResourceTree, EntryList, ValueDetail, JsonEditor,
                          ImportDialog, AddEntryDialog, Splitter, Icon, ...
  src/lib/wasm/           committed wasm-pack output (rebuilt by `bun run wasm`)
  src/routes/+page.svelte app shell + state
tools/                    fixture generator + wasm smoke test
vercel.json               Vercel build config (Rust-free)
```

[LevelDB]: https://github.com/google/leveldb
[Bun]: https://bun.sh
[wasm-pack]: https://rustwasm.github.io/wasm-pack/
