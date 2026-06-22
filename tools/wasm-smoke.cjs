// Runtime smoke test of the actual wasm bindings (the same KvsDb code the
// browser runs), exercised in Node against the LevelDB fixture. Verifies the
// JS<->wasm marshalling: entries() shape, get() decoding, put/delete, and
// export_changes() round-trip.
//
//   wasm-pack build kvs-core --dev --target nodejs --out-dir target/wasm-node --out-name kvs_core
//   node tools/wasm-smoke.cjs
const fs = require('fs');
const path = require('path');
const { KvsDb } = require('../kvs-core/target/wasm-node/kvs_core.js');

const fixtureDir = path.join(__dirname, '../kvs-core/tests/fixtures/sample-kvs');

function loadFiles() {
	return fs
		.readdirSync(fixtureDir)
		.filter((n) => n !== 'LOCK' && fs.statSync(path.join(fixtureDir, n)).isFile())
		.map((name) => ({ name, bytes: new Uint8Array(fs.readFileSync(path.join(fixtureDir, name))) }));
}

let pass = 0;
let fail = 0;
function check(cond, msg) {
	if (cond) pass++;
	else {
		fail++;
		console.error('  FAIL:', msg);
	}
}

const files = loadFiles();
const db = new KvsDb(files);

// --- read ---
const entries = db.entries();
check(Array.isArray(entries) && entries.length >= 7, `entries() length = ${entries?.length}`);
const colorRow = entries.find((e) => e.rawKey === 'res:my_hud:color');
check(!!colorRow, 'found res:my_hud:color row');
check(colorRow?.resource === 'my_hud' && colorRow?.namespace === 'res', 'row namespace/resource parsed');
check(colorRow?.valueType === 'string', `row valueType = ${colorRow?.valueType}`);

const color = db.get('res:my_hud:color');
check(color?.type === 'string' && color?.value === '#FF0000', `get string = ${JSON.stringify(color?.value)}`);
const count = db.get('res:my_hud:enabled_count');
check(count?.type === 'int' && count?.value === 42, `get int = ${JSON.stringify(count?.value)}`);
const scale = db.get('res:my_hud:scale');
check(scale?.type === 'float' && Math.abs(scale.value - 1.25) < 1e-6, `get float = ${JSON.stringify(scale?.value)}`);
check(typeof color?.hex === 'string' && color.hex.length > 0, 'get() returns hex');

// --- edit / add / delete ---
db.put('res:my_hud:color', '#123456', 'string');
db.put('res:new_res:added', 7, 'int');
db.put('res:my_hud:scale', 2.5, 'float');
db.delete('res:garage:slot_1');

const changes = db.export_changes();
check(Array.isArray(changes?.changed) && changes.changed.length > 0, `export changed count = ${changes?.changed?.length}`);
check(changes.changed.every((c) => c.bytes instanceof Uint8Array && typeof c.name === 'string'), 'changed entries are {name, Uint8Array}');

// --- reload from the exported file set (simulates writing to disk + reopening) ---
const map = new Map(files.map((f) => [f.name, f.bytes]));
for (const c of changes.changed) map.set(c.name, c.bytes);
for (const d of changes.deleted) map.delete(d);
const merged = [...map.entries()].map(([name, bytes]) => ({ name, bytes }));

const db2 = new KvsDb(merged);
check(db2.get('res:my_hud:color')?.value === '#123456', 'reopened: edited string persisted');
check(db2.get('res:new_res:added')?.value === 7, 'reopened: added int persisted');
check(Math.abs(db2.get('res:my_hud:scale')?.value - 2.5) < 1e-6, 'reopened: edited float persisted');
check(db2.get('res:garage:slot_1') === null, 'reopened: deleted key is gone');

// --- import mechanism: get_raw (verbatim bytes) + put 'raw' ---
const srcDb = new KvsDb(files); // pristine source
const rawColor = srcDb.get_raw('res:my_hud:color');
check(rawColor instanceof Uint8Array && rawColor.length > 0, 'get_raw returns bytes');
check(srcDb.get_raw('res:nope:missing') === null, 'get_raw on a missing key returns null');
const destDb = new KvsDb(files);
destDb.put('res:imported:thing', rawColor, 'raw');
check(destDb.get('res:imported:thing')?.value === '#FF0000', 'put raw imports a value verbatim');

console.log(`\nwasm smoke: ${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
