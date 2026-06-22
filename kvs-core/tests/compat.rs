//! Phase 0 compatibility spike.
//!
//! Proves that `rusty-leveldb` (pure Rust) can read a LevelDB database produced
//! by the *C++* implementation — via `classic-level`, which is a native binding
//! to real LevelDB, so the fixture exercises the genuine on-disk format and
//! Snappy block compression. This is the gate for the whole project: if this
//! passes, the browser (wasm) read path is sound.
//!
//! Regenerate the fixture with:
//!   npm --prefix tools/gen-fixture install
//!   node tools/gen-fixture/gen.mjs

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use kvs_core::{
    decode_value, for_each_entry, open_from_files, parse_key, snapshot_files, DbFile, KeyNamespace,
};

fn load_dir(dir: &Path) -> Vec<DbFile> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            out.push(DbFile {
                name: entry.file_name().to_string_lossy().into_owned(),
                bytes: fs::read(entry.path()).unwrap(),
            });
        }
    }
    out
}

#[test]
fn reads_fivem_style_leveldb() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample-kvs");
    assert!(
        dir.exists(),
        "fixture missing — run `npm --prefix tools/gen-fixture install && node tools/gen-fixture/gen.mjs`"
    );

    let files = load_dir(&dir);
    assert!(
        files.iter().any(|f| f.name == "CURRENT"),
        "not a leveldb dir (no CURRENT): {:?}",
        files.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    let (mut db, env) = open_from_files(&files).expect("open leveldb from raw files");

    // Iterate the whole DB; resource grouping via key parsing must work.
    let mut count = 0usize;
    let mut resources = BTreeSet::new();
    for_each_entry(&mut db, |k, _v| {
        count += 1;
        let pk = parse_key(k);
        if pk.namespace == KeyNamespace::Resource {
            resources.insert(pk.resource.unwrap());
        }
    })
    .expect("iterate db");
    assert!(count >= 7, "expected many entries, got {count}");
    assert!(
        resources.contains("my_hud"),
        "expected a 'my_hud' resource, got {resources:?}"
    );

    // msgpack decode must preserve types exactly (string / float / int).
    let color = db.get(b"res:my_hud:color").expect("color present");
    assert_eq!(decode_value(&color).unwrap().as_str(), Some("#FF0000"));

    let scale = db.get(b"res:my_hud:scale").expect("scale present");
    assert_eq!(decode_value(&scale).unwrap().as_f64(), Some(1.25));

    let n = db.get(b"res:my_hud:enabled_count").expect("count present");
    assert_eq!(decode_value(&n).unwrap().as_i64(), Some(42));

    // The bulk entries force memtable flushes -> Snappy-compressed .ldb tables,
    // so reading them above already exercised the Snappy decode path. Assert a
    // table actually exists, otherwise the Snappy path went untested.
    let snap = snapshot_files(&env).expect("snapshot files");
    assert!(
        snap.iter()
            .any(|f| f.name.ends_with(".ldb") || f.name.ends_with(".sst")),
        "no .ldb/.sst table in fixture — Snappy path not exercised; files: {:?}",
        snap.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

fn msgpack(v: rmpv::Value) -> Vec<u8> {
    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &v).expect("encode msgpack");
    buf
}

/// De-risks the write-back path: edit an existing value, add a brand-new
/// resource/key, and delete a key; then snapshot the mutated files (exactly
/// what we'd write to disk) and reopen them to confirm the DB is still valid and
/// the changes persisted.
#[test]
fn write_add_delete_roundtrip() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample-kvs");
    let files = load_dir(&dir);
    let (mut db, env) = open_from_files(&files).expect("open leveldb");

    db.put(b"res:my_hud:color", &msgpack(rmpv::Value::from("#00FF00")))
        .expect("edit existing");
    db.put(b"res:new_res:greeting", &msgpack(rmpv::Value::from("hello")))
        .expect("add new key");
    db.delete(b"res:garage:slot_1").expect("delete key");
    db.flush().expect("flush");

    // The mutated file set is exactly what gets written back to the folder.
    let changed = snapshot_files(&env).expect("snapshot");
    assert!(changed.iter().any(|f| f.name == "CURRENT"));

    // Reopen from the snapshot — simulates writing to disk and reloading.
    let (mut db2, _env2) = open_from_files(&changed).expect("reopen mutated db");

    let color = db2.get(b"res:my_hud:color").expect("edited value present");
    assert_eq!(decode_value(&color).unwrap().as_str(), Some("#00FF00"));

    let greeting = db2.get(b"res:new_res:greeting").expect("new value present");
    assert_eq!(decode_value(&greeting).unwrap().as_str(), Some("hello"));

    assert!(
        db2.get(b"res:garage:slot_1").is_none(),
        "deleted key is still present after reopen"
    );

    let mut resources = BTreeSet::new();
    for_each_entry(&mut db2, |k, _v| {
        let pk = parse_key(k);
        if pk.namespace == KeyNamespace::Resource {
            resources.insert(pk.resource.unwrap());
        }
    })
    .expect("iterate reopened db");
    assert!(resources.contains("new_res"), "new resource not grouped");
}
