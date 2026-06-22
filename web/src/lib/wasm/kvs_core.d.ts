/* tslint:disable */
/* eslint-disable */

/**
 * An open FiveM KVS database, backed by an in-memory LevelDB.
 */
export class KvsDb {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Delete a key.
     */
    delete(raw_key: string): void;
    /**
     * All entries as lightweight rows for the explorer list.
     */
    entries(): any;
    /**
     * Diff the current (mutated) file set against what was originally loaded.
     * Returns `{ changed: [{name, bytes}], deleted: [name] }` — exactly the
     * files to write to / remove from the folder on disk.
     */
    export_changes(): any;
    /**
     * Fetch and fully decode a single value:
     * `{ type, value, hex, byteLen } | null`.
     */
    get(raw_key: string): any;
    /**
     * Raw stored value bytes for a key (for verbatim copy / import), or null.
     */
    get_raw(raw_key: string): any;
    /**
     * Reset the diff baseline to the current state. Call after the changes from
     * `export_changes` have been successfully written to disk, so the next save
     * diffs against what's now on disk.
     */
    mark_saved(): void;
    /**
     * Open a database from the raw folder files.
     *
     * `files` is a JS array of `{ name: string, bytes: Uint8Array }`.
     */
    constructor(files: any);
    /**
     * Insert or update a value. `kind` is `string` | `int` | `float` | `bool`
     * | `raw`; for `raw`, `value` is a `Uint8Array` stored verbatim.
     */
    put(raw_key: string, value: any, kind: string): void;
}

/**
 * Install a panic hook so Rust panics surface as readable console errors.
 */
export function start(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_kvsdb_free: (a: number, b: number) => void;
    readonly kvsdb_delete: (a: number, b: number, c: number) => [number, number];
    readonly kvsdb_entries: (a: number) => [number, number, number];
    readonly kvsdb_export_changes: (a: number) => [number, number, number];
    readonly kvsdb_get: (a: number, b: number, c: number) => [number, number, number];
    readonly kvsdb_get_raw: (a: number, b: number, c: number) => [number, number, number];
    readonly kvsdb_mark_saved: (a: number) => [number, number];
    readonly kvsdb_open: (a: any) => [number, number, number];
    readonly kvsdb_put: (a: number, b: number, c: number, d: any, e: number, f: number) => [number, number];
    readonly start: () => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
