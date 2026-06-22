//! Portable core for reading/writing a FiveM KVS store.
//!
//! A FiveM "KVS folder" is a [LevelDB] database (FiveM opens it at the virtual
//! path `fxd:/kvs/`, which maps to `%appdata%/CitizenFX/kvs` on disk). Keys are
//! namespaced:
//!   * `res:<resource>:<key>` — a resource KVP entry (`SetResourceKvp*`)
//!   * `rv:<...>`             — resource-version metadata
//! Values are msgpack-encoded, so their type (string / int / float) is
//! self-describing.
//!
//! This module is deliberately platform-agnostic: it operates purely on a
//! `MemEnv` populated from raw file bytes, so the exact same code path runs in
//! a native `cargo test` and inside the browser (wasm), where the bytes come
//! from the File System Access API instead of disk.
//!
//! [LevelDB]: https://github.com/google/leveldb

use std::io::{Read, Write};
use std::path::Path;
use std::rc::Rc;

use rusty_leveldb::env::Env;
use rusty_leveldb::{LdbIterator, MemEnv, Options, DB};

#[cfg(target_arch = "wasm32")]
mod wasm;

/// Mount point for the in-memory database inside the `MemEnv`.
pub const DB_DIR: &str = "db";

/// Shared in-memory environment backing the database. Cloning the `Rc` shares
/// the same underlying file map between our own file I/O and the open `DB`, so
/// edits the DB makes are visible when we snapshot the files back out.
pub type SharedEnv = Rc<Box<dyn Env>>;

/// One LevelDB file (the on-disk representation of a single file in the folder).
#[derive(Clone, Debug)]
pub struct DbFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Build an in-memory LevelDB from the raw files of a KVS folder and open it.
///
/// `files` is the content of the dragged folder: each LevelDB file
/// (`CURRENT`, `MANIFEST-*`, `*.ldb`, `*.log`, `LOG`, ...) as `(name, bytes)`.
/// Returns the open [`DB`] plus the shared [`MemEnv`], so the caller can later
/// snapshot the (possibly mutated) file set to write back to disk.
pub fn open_from_files(files: &[DbFile]) -> Result<(DB, SharedEnv), String> {
    let env: SharedEnv = Rc::new(Box::new(MemEnv::new()));
    let _ = env.mkdir(Path::new(DB_DIR));

    for f in files {
        // Use Path::join to match LevelDB's own internal path construction
        // (separator differs per platform); MemEnv keys files by these paths.
        let path = Path::new(DB_DIR).join(&f.name);
        let mut w = env
            .open_writable_file(&path)
            .map_err(|e| format!("create {}: {e:?}", f.name))?;
        w.write_all(&f.bytes)
            .map_err(|e| format!("write {}: {e:?}", f.name))?;
        w.flush().map_err(|e| format!("flush {}: {e:?}", f.name))?;
    }

    let mut opt = Options::default();
    opt.env = env.clone();
    opt.create_if_missing = false;
    let db = DB::open(DB_DIR, opt).map_err(|e| format!("open db: {e:?}"))?;
    Ok((db, env))
}

/// Read every file currently in the in-memory database directory.
///
/// This reflects LevelDB's own file set after any `put`/`delete`+`flush`, so
/// diffing this against the originally-loaded files yields exactly the files to
/// write back to (or remove from) disk. The `LOCK` file is never returned.
pub fn snapshot_files(env: &SharedEnv) -> Result<Vec<DbFile>, String> {
    let mut out = Vec::new();
    let children = env
        .children(Path::new(DB_DIR))
        .map_err(|e| format!("list db dir: {e:?}"))?;

    for child in children {
        let name = child
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| child.to_string_lossy().into_owned());
        if name == "LOCK" {
            continue;
        }
        let path = Path::new(DB_DIR).join(&name);
        let mut r = env
            .open_sequential_file(&path)
            .map_err(|e| format!("read {name}: {e:?}"))?;
        let mut bytes = Vec::new();
        r.read_to_end(&mut bytes)
            .map_err(|e| format!("read {name}: {e:?}"))?;
        out.push(DbFile { name, bytes });
    }
    Ok(out)
}

/// Visit every key/value pair in the database, in sorted key order.
///
/// `rusty-leveldb`'s `DBIterator` is not a std `Iterator`; it uses the
/// `LdbIterator` `seek_to_first`/`current`/`advance` protocol, wrapped here.
pub fn for_each_entry(db: &mut DB, mut f: impl FnMut(&[u8], &[u8])) -> Result<(), String> {
    // rusty-leveldb iterators start *before* the first element: advance() must
    // be called before each read and returns false once exhausted.
    let mut it = db.new_iter().map_err(|e| format!("new_iter: {e:?}"))?;
    while it.advance() {
        if let Some((k, v)) = it.current() {
            f(&k, &v);
        }
    }
    Ok(())
}

/// The FiveM namespace a raw LevelDB key belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyNamespace {
    /// `res:<resource>:<key>` — a resource KVP entry.
    Resource,
    /// `rv:<...>` — resource-version metadata.
    ResourceVersion,
    /// Anything that doesn't match a known prefix.
    Other,
}

/// A raw key split into its FiveM namespace components.
#[derive(Debug, Clone)]
pub struct ParsedKey {
    pub namespace: KeyNamespace,
    pub resource: Option<String>,
    pub key: String,
}

/// Split a raw LevelDB key into its FiveM namespace components.
pub fn parse_key(raw: &[u8]) -> ParsedKey {
    let s = String::from_utf8_lossy(raw);
    if let Some(rest) = s.strip_prefix("res:") {
        if let Some((resource, key)) = rest.split_once(':') {
            return ParsedKey {
                namespace: KeyNamespace::Resource,
                resource: Some(resource.to_string()),
                key: key.to_string(),
            };
        }
    }
    if let Some(rest) = s.strip_prefix("rv:") {
        return ParsedKey {
            namespace: KeyNamespace::ResourceVersion,
            resource: None,
            key: rest.to_string(),
        };
    }
    ParsedKey {
        namespace: KeyNamespace::Other,
        resource: None,
        key: s.into_owned(),
    }
}

/// Decode a msgpack-encoded KVS value into a dynamic value.
pub fn decode_value(bytes: &[u8]) -> Option<rmpv::Value> {
    rmpv::decode::read_value(&mut &bytes[..]).ok()
}

/// Encode a dynamic value back to msgpack bytes (the storage format).
pub fn encode_value(v: &rmpv::Value) -> Vec<u8> {
    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, v).expect("msgpack encode to Vec never fails");
    buf
}

/// A short, human-facing type label for a decoded value.
pub fn value_type(v: &rmpv::Value) -> &'static str {
    match v {
        rmpv::Value::Nil => "nil",
        rmpv::Value::Boolean(_) => "bool",
        rmpv::Value::Integer(_) => "int",
        rmpv::Value::F32(_) | rmpv::Value::F64(_) => "float",
        rmpv::Value::String(_) => "string",
        rmpv::Value::Binary(_) => "binary",
        rmpv::Value::Array(_) => "array",
        rmpv::Value::Map(_) => "map",
        rmpv::Value::Ext(_, _) => "ext",
    }
}

/// A short preview string for list display, truncated to `max` characters.
pub fn value_preview(v: &rmpv::Value, max: usize) -> String {
    let s = match v {
        rmpv::Value::String(s) => s.as_str().unwrap_or("<non-utf8>").to_owned(),
        rmpv::Value::Integer(i) => i
            .as_i64()
            .map(|n| n.to_string())
            .or_else(|| i.as_u64().map(|n| n.to_string()))
            .unwrap_or_default(),
        rmpv::Value::F32(f) => f.to_string(),
        rmpv::Value::F64(f) => f.to_string(),
        rmpv::Value::Boolean(b) => b.to_string(),
        rmpv::Value::Nil => "nil".to_owned(),
        rmpv::Value::Binary(b) => format!("<{} bytes>", b.len()),
        rmpv::Value::Array(a) => format!("[{} items]", a.len()),
        rmpv::Value::Map(m) => format!("{{{} entries}}", m.len()),
        rmpv::Value::Ext(t, b) => format!("<ext {t}, {} bytes>", b.len()),
    };
    truncate_chars(&s, max)
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let head: String = s.chars().take(max).collect();
        format!("{head}…")
    } else {
        s.to_owned()
    }
}

/// Cheap heuristic: does this string look like a JSON object or array? Used to
/// flag JSON-bearing string values in the list without a full parse.
pub fn looks_like_json(s: &str) -> bool {
    let t = s.trim();
    (t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_namespaced_keys() {
        let pk = parse_key(b"res:my_res:foo:bar");
        assert_eq!(pk.namespace, KeyNamespace::Resource);
        assert_eq!(pk.resource.as_deref(), Some("my_res"));
        assert_eq!(pk.key, "foo:bar"); // keys may contain colons

        let pk = parse_key(b"rv:cache:thing");
        assert_eq!(pk.namespace, KeyNamespace::ResourceVersion);
        assert_eq!(pk.key, "cache:thing");

        let pk = parse_key(b"weird");
        assert_eq!(pk.namespace, KeyNamespace::Other);
    }

    #[test]
    fn types_and_previews() {
        assert_eq!(value_type(&rmpv::Value::from("hi")), "string");
        assert_eq!(value_type(&rmpv::Value::from(42i64)), "int");
        assert_eq!(value_type(&rmpv::Value::F32(1.5)), "float");
        assert_eq!(value_preview(&rmpv::Value::from("hello world"), 5), "hello…");
        assert_eq!(value_preview(&rmpv::Value::from(7i64), 10), "7");
    }

    #[test]
    fn encode_decode_roundtrips() {
        for v in [
            rmpv::Value::from("text"),
            rmpv::Value::from(123i64),
            rmpv::Value::F32(2.5),
        ] {
            let bytes = encode_value(&v);
            assert_eq!(decode_value(&bytes), Some(v));
        }
    }
}
