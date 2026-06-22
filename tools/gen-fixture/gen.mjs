// Generates a real LevelDB database (via classic-level — native bindings to the
// C++ LevelDB) shaped like a FiveM KVS store, so the kvs-core compatibility test
// reads genuine on-disk format + Snappy-compressed tables.
//
// Output: kvs-core/tests/fixtures/sample-kvs/
import { ClassicLevel } from 'classic-level';
import { encode } from '@msgpack/msgpack';
import { rmSync, mkdirSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const outDir = fileURLToPath(
  new URL('../../kvs-core/tests/fixtures/sample-kvs/', import.meta.url),
);

rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });

const db = new ClassicLevel(outDir, {
  createIfMissing: true,
  errorIfExists: false,
  compression: true, // Snappy — what FiveM/LevelDB use by default
  // Tiny write buffer so even a small fixture flushes memtables to .ldb tables,
  // ensuring the Snappy SST path is present in the committed fixture.
  writeBufferSize: 16 * 1024,
  keyEncoding: 'utf8',
  valueEncoding: 'buffer',
});

const put = (k, v) => db.put(k, Buffer.from(encode(v)));

// Realistic FiveM-style entries: res:<resource>:<key> -> msgpack value.
// Mirrors SetResourceKvp (string), SetResourceKvpFloat (float), SetResourceKvpInt (int).
await put('res:my_hud:color', '#FF0000');
await put('res:my_hud:label', 'Speedometer');
await put('res:my_hud:scale', 1.25); // float
await put('res:my_hud:enabled_count', 42); // int
await put('res:garage:slot_1', 'adder');
await put('res:garage:slot_count', 7);
await put('rv:cache:my_hud', 12345); // resource-version metadata namespace

// JSON-as-string values (the common FiveM pattern: SetResourceKvp(json.encode(t))).
await put(
	'res:my_hud:layout',
	JSON.stringify({
		position: { x: 0.5, y: 0.85 },
		scale: 1.25,
		visible: true,
		widgets: ['speed', 'fuel', 'gear', 'rpm'],
		colors: { primary: '#ffffff', accent: '#888888' }
	})
);
await put(
	'res:garage:vehicles',
	JSON.stringify([
		{ model: 'adder', plate: 'FAST01', mods: { engine: 4, turbo: true } },
		{ model: 'zentorno', plate: 'GOFAST', mods: { engine: 3, turbo: false } }
	])
);

// Bulk, compressible entries spanning many write-buffer flushes, so the fixture
// contains Snappy-compressed .ldb tables (not just an uncompressed WAL).
for (let i = 0; i < 400; i++) {
  const padded = `PADDING_${'x'.repeat(48)}_${i}`;
  await put(`res:bulk_res:key_${String(i).padStart(4, '0')}`, padded);
}

// Best-effort compaction to merge memtable + L0 into tables.
try {
  await db.compactRange('', '￿');
} catch (e) {
  console.warn('compactRange skipped:', e.message);
}

await db.close();

const files = readdirSync(outDir).sort();
console.log(`Fixture written to ${outDir}`);
console.log('Files:', files.join(', '));
const tables = files.filter((f) => f.endsWith('.ldb') || f.endsWith('.sst'));
if (tables.length === 0) {
  console.error('WARNING: no .ldb/.sst tables produced — Snappy path will be untested.');
  process.exit(1);
}
