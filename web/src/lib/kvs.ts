// Bridge between the browser (File System Access API) and the wasm core.
//
// Data flow:
//   folder  --readFolder-->  RawFile[]  --new KvsDb-->  in-memory LevelDB
//   edits   --put/delete-->  KvsDb
//   save    --export_changes-->  write/remove files in the folder
//
// Nothing ever leaves the machine.

import init, { KvsDb } from '$lib/wasm/kvs_core.js';
import { zipSync } from 'fflate';
import type { Changes, EntryRow, GetResult, ValueKind } from './types';

let wasmReady: Promise<unknown> | null = null;
function ensureWasm(): Promise<unknown> {
	if (!wasmReady) wasmReady = init();
	return wasmReady;
}

// --- browser capability ----------------------------------------------------

/** True only on browsers that can both pick a directory and write back to it. */
export function isSupported(): boolean {
	if (typeof window === 'undefined') return false;
	const w = window as unknown as Record<string, unknown>;
	const hasPicker = 'showDirectoryPicker' in window;
	const fileHandle = w.FileSystemFileHandle as { prototype?: { createWritable?: unknown } } | undefined;
	const hasWritable = typeof fileHandle?.prototype?.createWritable === 'function';
	return hasPicker && hasWritable;
}

/** Brave intentionally disables the File System Access API behind a flag. */
export async function detectBrave(): Promise<boolean> {
	const nav = navigator as Navigator & { brave?: { isBrave?: () => Promise<boolean> } };
	try {
		return (await nav.brave?.isBrave?.()) === true;
	} catch {
		return false;
	}
}

// --- choosing a folder -----------------------------------------------------

function directoryPicker(): (o?: unknown) => Promise<FileSystemDirectoryHandle> {
	return (window as unknown as {
		showDirectoryPicker: (o?: unknown) => Promise<FileSystemDirectoryHandle>;
	}).showDirectoryPicker;
}

export async function pickDirectory(): Promise<FileSystemDirectoryHandle> {
	return directoryPicker()({ mode: 'readwrite', id: 'fivem-kvs' });
}

/** Pick a second folder to import from — read-only. */
export async function pickImportDirectory(): Promise<FileSystemDirectoryHandle> {
	return directoryPicker()({ mode: 'read', id: 'fivem-kvs-import' });
}

/** Extract a directory handle from a drop event's DataTransfer, if present. */
export async function directoryFromDrop(dt: DataTransfer): Promise<FileSystemDirectoryHandle | null> {
	// getAsFileSystemHandle() must be invoked synchronously while the drop event
	// is live, so kick them all off before awaiting.
	const pending: Promise<FileSystemHandle | null>[] = [];
	for (let i = 0; i < dt.items.length; i++) {
		const item = dt.items[i] as DataTransferItem & {
			getAsFileSystemHandle?: () => Promise<FileSystemHandle | null>;
		};
		if (item.kind === 'file' && typeof item.getAsFileSystemHandle === 'function') {
			pending.push(item.getAsFileSystemHandle());
		}
	}
	for (const handle of await Promise.all(pending)) {
		if (handle && handle.kind === 'directory') return handle as FileSystemDirectoryHandle;
	}
	return null;
}

// --- permissions -----------------------------------------------------------

type PermHandle = {
	queryPermission?: (o: { mode: string }) => Promise<PermissionState>;
	requestPermission?: (o: { mode: string }) => Promise<PermissionState>;
};

export async function ensureReadWrite(dir: FileSystemDirectoryHandle): Promise<boolean> {
	const h = dir as unknown as PermHandle;
	const opts = { mode: 'readwrite' };
	if ((await h.queryPermission?.(opts)) === 'granted') return true;
	return (await h.requestPermission?.(opts)) === 'granted';
}

/** Read-only permission — enough for an import source we never write to. */
export async function ensureRead(dir: FileSystemDirectoryHandle): Promise<boolean> {
	const h = dir as unknown as PermHandle;
	const opts = { mode: 'read' };
	if ((await h.queryPermission?.(opts)) === 'granted') return true;
	return (await h.requestPermission?.(opts)) === 'granted';
}

// --- reading the folder ----------------------------------------------------

export interface RawFile {
	name: string;
	bytes: Uint8Array;
}

export async function readFolder(dir: FileSystemDirectoryHandle): Promise<RawFile[]> {
	const files: RawFile[] = [];
	const entries = (dir as unknown as {
		entries: () => AsyncIterable<[string, FileSystemHandle]>;
	}).entries();
	for await (const [name, handle] of entries) {
		if (handle.kind !== 'file') continue; // a LevelDB folder is flat
		const file = await (handle as FileSystemFileHandle).getFile();
		files.push({ name, bytes: new Uint8Array(await file.arrayBuffer()) });
	}
	return files;
}

/** Heuristic: does this folder look like a LevelDB database? */
export function looksLikeLevelDb(files: RawFile[]): boolean {
	return files.some((f) => f.name === 'CURRENT');
}

// --- a live editing session ------------------------------------------------

export interface KvsSession {
	db: KvsDb;
	/** null for the bundled read-only sample data. */
	dir: FileSystemDirectoryHandle | null;
	original: RawFile[];
	backedUp: boolean;
}

export async function openSession(
	dir: FileSystemDirectoryHandle | null,
	files: RawFile[]
): Promise<KvsSession> {
	await ensureWasm();
	const db = new KvsDb(files);
	return { db, dir, original: files, backedUp: false };
}

/** Load the bundled sample KVS (read-only) from static/sample. */
export async function loadSample(base = ''): Promise<RawFile[]> {
	const manifest: string[] = await (await fetch(`${base}/sample/manifest.json`)).json();
	const files: RawFile[] = [];
	for (const name of manifest) {
		const res = await fetch(`${base}/sample/${name}`);
		files.push({ name, bytes: new Uint8Array(await res.arrayBuffer()) });
	}
	return files;
}

export function listEntries(s: KvsSession): EntryRow[] {
	return s.db.entries() as EntryRow[];
}

export function getValue(s: KvsSession, rawKey: string): GetResult | null {
	return s.db.get(rawKey) as GetResult | null;
}

export function putValue(s: KvsSession, rawKey: string, value: unknown, kind: ValueKind): void {
	s.db.put(rawKey, value, kind);
}

export function deleteValue(s: KvsSession, rawKey: string): void {
	s.db.delete(rawKey);
}

/** Exact stored bytes for a key (used to copy values verbatim during import). */
export function getRawValue(s: KvsSession, rawKey: string): Uint8Array | null {
	return s.db.get_raw(rawKey) as Uint8Array | null;
}

/** Store value bytes verbatim (no re-encoding) — the import path. */
export function putRawValue(s: KvsSession, rawKey: string, bytes: Uint8Array): void {
	s.db.put(rawKey, bytes, 'raw');
}

/** Open a second folder read-only (an import source); never written to disk. */
export async function openImportSource(files: RawFile[]): Promise<KvsSession> {
	return openSession(null, files);
}

// --- saving ----------------------------------------------------------------

/** Zip the folder exactly as loaded and download it as a safety backup. */
export function downloadBackup(s: KvsSession): void {
	const entries: Record<string, Uint8Array> = {};
	for (const f of s.original) entries[f.name] = f.bytes;
	const zipped = zipSync(entries, { level: 6 });
	triggerDownload(zipped, `kvs-backup-${timestamp()}.zip`);
	s.backedUp = true;
}

export interface SaveResult {
	written: number;
	deleted: number;
}

export async function saveToDisk(s: KvsSession): Promise<SaveResult> {
	if (!s.dir) throw new Error('This is read-only sample data — open your own folder to save changes.');
	const changes = s.db.export_changes() as Changes;
	for (const { name, bytes } of changes.changed) {
		const fh = await s.dir.getFileHandle(name, { create: true });
		const writable = await fh.createWritable();
		// Wrap in a Blob to sidestep TS's ArrayBufferLike/SharedArrayBuffer narrowing.
		await writable.write(new Blob([bytes as BlobPart]));
		await writable.close();
	}
	for (const name of changes.deleted) {
		try {
			await s.dir.removeEntry(name);
		} catch {
			/* already gone */
		}
	}
	s.db.mark_saved();
	return { written: changes.changed.length, deleted: changes.deleted.length };
}

// --- helpers ---------------------------------------------------------------

function triggerDownload(bytes: Uint8Array, filename: string): void {
	const blob = new Blob([bytes as BlobPart], { type: 'application/zip' });
	const url = URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.href = url;
	a.download = filename;
	document.body.appendChild(a);
	a.click();
	a.remove();
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function timestamp(): string {
	return new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
}
