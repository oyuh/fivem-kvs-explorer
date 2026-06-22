// Shapes returned by the wasm `KvsDb` (see kvs-core/src/wasm.rs).

export type Namespace = 'res' | 'rv' | 'other';

export interface EntryRow {
	/** The full LevelDB key, e.g. `res:my_hud:color`. */
	rawKey: string;
	namespace: Namespace;
	/** Resource name for `res:` keys, else null. */
	resource: string | null;
	/** The key portion after the namespace/resource. */
	key: string;
	/** `string` | `int` | `float` | `bool` | `array` | `map` | `bytes` | ... */
	valueType: string;
	/** Short, truncated preview for the list. */
	preview: string;
	/** Whether the value decoded as msgpack (false ⇒ shown as raw bytes). */
	decodable: boolean;
	byteLen: number;
}

export interface GetResult {
	type: string;
	/** Best-effort native JS value (string/number/bool/array/object). */
	value: unknown;
	/** Lowercase hex of the raw stored bytes. */
	hex: string;
	byteLen: number;
}

/** The msgpack type a value is (re-)encoded as when saved. */
export type ValueKind = 'string' | 'int' | 'float' | 'bool' | 'raw';

export interface Changes {
	changed: { name: string; bytes: Uint8Array }[];
	deleted: string[];
}

/** A sidebar grouping: a resource, a namespace bucket, or the "all" pseudo-group. */
export interface Group {
	id: string;
	label: string;
	count: number;
	type: 'all' | 'res' | 'rv' | 'other';
}
