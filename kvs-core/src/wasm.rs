//! wasm-bindgen bindings: the `KvsDb` object the browser drives.
//!
//! Only compiled for `wasm32` (gated in `lib.rs`). All LevelDB/msgpack logic
//! lives in the portable parent module; this file is just the JS interop:
//! marshalling `Uint8Array`s in and out and shaping JS objects.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use rusty_leveldb::DB;

use crate::{
    decode_value, encode_value, for_each_entry, looks_like_json, open_from_files, parse_key,
    snapshot_files, value_preview, value_type, DbFile, KeyNamespace, SharedEnv,
};

/// Install a panic hook so Rust panics surface as readable console errors.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

fn js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn hash_bytes(b: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    b.hash(&mut h);
    h.finish()
}

fn to_uint8(b: &[u8]) -> js_sys::Uint8Array {
    let arr = js_sys::Uint8Array::new_with_length(b.len() as u32);
    arr.copy_from(b);
    arr
}

fn set(obj: &js_sys::Object, key: &str, val: &JsValue) -> Result<(), JsValue> {
    js_sys::Reflect::set(obj, &JsValue::from_str(key), val).map(|_| ())
}

fn to_hex(b: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(b.len() * 2);
    for byte in b {
        let _ = write!(s, "{byte:02x}");
    }
    s
}

fn ns_str(ns: &KeyNamespace) -> &'static str {
    match ns {
        KeyNamespace::Resource => "res",
        KeyNamespace::ResourceVersion => "rv",
        KeyNamespace::Other => "other",
    }
}

/// Best-effort conversion of a decoded msgpack value to a native JS value.
fn value_to_js(v: &rmpv::Value) -> JsValue {
    match v {
        rmpv::Value::Nil => JsValue::NULL,
        rmpv::Value::Boolean(b) => JsValue::from_bool(*b),
        rmpv::Value::Integer(i) => i
            .as_i64()
            .map(|n| JsValue::from_f64(n as f64))
            .or_else(|| i.as_u64().map(|n| JsValue::from_f64(n as f64)))
            .unwrap_or(JsValue::NULL),
        rmpv::Value::F32(f) => JsValue::from_f64(*f as f64),
        rmpv::Value::F64(f) => JsValue::from_f64(*f),
        rmpv::Value::String(s) => s.as_str().map(JsValue::from_str).unwrap_or(JsValue::NULL),
        rmpv::Value::Binary(b) => to_uint8(b).into(),
        rmpv::Value::Array(a) => {
            let arr = js_sys::Array::new();
            for item in a {
                arr.push(&value_to_js(item));
            }
            arr.into()
        }
        rmpv::Value::Map(m) => {
            let obj = js_sys::Object::new();
            for (k, val) in m {
                let key = match k {
                    rmpv::Value::String(s) => s.as_str().unwrap_or_default().to_owned(),
                    other => format!("{other}"),
                };
                let _ = set(&obj, &key, &value_to_js(val));
            }
            obj.into()
        }
        rmpv::Value::Ext(_, b) => to_uint8(b).into(),
    }
}

/// One row in the explorer list (lightweight; full value fetched on demand).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EntryRow {
    raw_key: String,
    namespace: &'static str,
    resource: Option<String>,
    key: String,
    value_type: &'static str,
    preview: String,
    /// Whether the value decoded as msgpack (false ⇒ shown as raw bytes).
    decodable: bool,
    byte_len: usize,
}

/// An open FiveM KVS database, backed by an in-memory LevelDB.
#[wasm_bindgen]
pub struct KvsDb {
    db: DB,
    env: SharedEnv,
    /// name -> content hash of the files as originally loaded, for diffing.
    original: HashMap<String, u64>,
}

#[wasm_bindgen]
impl KvsDb {
    /// Open a database from the raw folder files.
    ///
    /// `files` is a JS array of `{ name: string, bytes: Uint8Array }`.
    #[wasm_bindgen(constructor)]
    pub fn open(files: JsValue) -> Result<KvsDb, JsValue> {
        let arr: js_sys::Array = files.dyn_into().map_err(|_| js_err("files must be an array"))?;
        let mut dbfiles = Vec::with_capacity(arr.length() as usize);
        for item in arr.iter() {
            let name = js_sys::Reflect::get(&item, &JsValue::from_str("name"))?
                .as_string()
                .ok_or_else(|| js_err("file.name must be a string"))?;
            let bytes_val = js_sys::Reflect::get(&item, &JsValue::from_str("bytes"))?;
            let u8: js_sys::Uint8Array = bytes_val
                .dyn_into()
                .map_err(|_| js_err("file.bytes must be a Uint8Array"))?;
            dbfiles.push(DbFile {
                name,
                bytes: u8.to_vec(),
            });
        }

        let original = dbfiles
            .iter()
            .map(|f| (f.name.clone(), hash_bytes(&f.bytes)))
            .collect();
        let (db, env) = open_from_files(&dbfiles).map_err(js_err)?;
        Ok(KvsDb { db, env, original })
    }

    /// All entries as lightweight rows for the explorer list.
    pub fn entries(&mut self) -> Result<JsValue, JsValue> {
        let mut rows: Vec<EntryRow> = Vec::new();
        for_each_entry(&mut self.db, |k, v| {
            let pk = parse_key(k);
            let decoded = decode_value(v);
            rows.push(EntryRow {
                raw_key: String::from_utf8_lossy(k).into_owned(),
                namespace: ns_str(&pk.namespace),
                resource: pk.resource,
                key: pk.key,
                value_type: match decoded.as_ref() {
                    Some(rmpv::Value::String(s)) if s.as_str().is_some_and(looks_like_json) => "json",
                    Some(d) => value_type(d),
                    None => "bytes",
                },
                preview: decoded
                    .as_ref()
                    .map(|d| value_preview(d, 80))
                    .unwrap_or_else(|| format!("<{} bytes>", v.len())),
                decodable: decoded.is_some(),
                byte_len: v.len(),
            });
        })
        .map_err(js_err)?;

        serde_wasm_bindgen::to_value(&rows).map_err(js_err)
    }

    /// Fetch and fully decode a single value:
    /// `{ type, value, hex, byteLen } | null`.
    pub fn get(&mut self, raw_key: String) -> Result<JsValue, JsValue> {
        let Some(bytes) = self.db.get(raw_key.as_bytes()) else {
            return Ok(JsValue::NULL);
        };
        let decoded = decode_value(&bytes);
        let obj = js_sys::Object::new();
        set(
            &obj,
            "type",
            &JsValue::from_str(decoded.as_ref().map(value_type).unwrap_or("bytes")),
        )?;
        set(
            &obj,
            "value",
            &decoded.as_ref().map(value_to_js).unwrap_or(JsValue::NULL),
        )?;
        set(&obj, "hex", &JsValue::from_str(&to_hex(&bytes)))?;
        set(&obj, "byteLen", &JsValue::from_f64(bytes.len() as f64))?;
        Ok(obj.into())
    }

    /// Raw stored value bytes for a key (for verbatim copy / import), or null.
    pub fn get_raw(&mut self, raw_key: String) -> Result<JsValue, JsValue> {
        match self.db.get(raw_key.as_bytes()) {
            Some(bytes) => Ok(to_uint8(&bytes).into()),
            None => Ok(JsValue::NULL),
        }
    }

    /// Insert or update a value. `kind` is `string` | `int` | `float` | `bool`
    /// | `raw`; for `raw`, `value` is a `Uint8Array` stored verbatim.
    pub fn put(&mut self, raw_key: String, value: JsValue, kind: String) -> Result<(), JsValue> {
        let bytes = match kind.as_str() {
            "string" => encode_value(&rmpv::Value::from(
                value.as_string().ok_or_else(|| js_err("expected a string"))?,
            )),
            "int" => {
                let n = value.as_f64().ok_or_else(|| js_err("expected a number"))?;
                encode_value(&rmpv::Value::from(n as i64))
            }
            // FiveM stores KvpFloat as a 32-bit float.
            "float" => {
                let n = value.as_f64().ok_or_else(|| js_err("expected a number"))?;
                encode_value(&rmpv::Value::F32(n as f32))
            }
            "bool" => encode_value(&rmpv::Value::Boolean(
                value.as_bool().ok_or_else(|| js_err("expected a bool"))?,
            )),
            "raw" => {
                let u8: js_sys::Uint8Array = value
                    .dyn_into()
                    .map_err(|_| js_err("raw value must be a Uint8Array"))?;
                u8.to_vec()
            }
            other => return Err(js_err(format!("unknown value kind: {other}"))),
        };
        self.db.put(raw_key.as_bytes(), &bytes).map_err(js_err)?;
        self.db.flush().map_err(js_err)?;
        Ok(())
    }

    /// Delete a key.
    pub fn delete(&mut self, raw_key: String) -> Result<(), JsValue> {
        self.db.delete(raw_key.as_bytes()).map_err(js_err)?;
        self.db.flush().map_err(js_err)?;
        Ok(())
    }

    /// Diff the current (mutated) file set against what was originally loaded.
    /// Returns `{ changed: [{name, bytes}], deleted: [name] }` — exactly the
    /// files to write to / remove from the folder on disk.
    pub fn export_changes(&mut self) -> Result<JsValue, JsValue> {
        self.db.flush().map_err(js_err)?;
        let snap = snapshot_files(&self.env).map_err(js_err)?;

        let changed = js_sys::Array::new();
        let mut seen = HashSet::new();
        for f in &snap {
            seen.insert(f.name.clone());
            let unchanged = self
                .original
                .get(&f.name)
                .is_some_and(|h| *h == hash_bytes(&f.bytes));
            if !unchanged {
                let o = js_sys::Object::new();
                set(&o, "name", &JsValue::from_str(&f.name))?;
                set(&o, "bytes", &to_uint8(&f.bytes).into())?;
                changed.push(&o);
            }
        }

        let deleted = js_sys::Array::new();
        for name in self.original.keys() {
            if !seen.contains(name) {
                deleted.push(&JsValue::from_str(name));
            }
        }

        let result = js_sys::Object::new();
        set(&result, "changed", &changed.into())?;
        set(&result, "deleted", &deleted.into())?;
        Ok(result.into())
    }

    /// Reset the diff baseline to the current state. Call after the changes from
    /// `export_changes` have been successfully written to disk, so the next save
    /// diffs against what's now on disk.
    pub fn mark_saved(&mut self) -> Result<(), JsValue> {
        self.db.flush().map_err(js_err)?;
        let snap = snapshot_files(&self.env).map_err(js_err)?;
        self.original = snap
            .iter()
            .map(|f| (f.name.clone(), hash_bytes(&f.bytes)))
            .collect();
        Ok(())
    }
}
