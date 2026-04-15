use std::mem;

use hash_map::{CollisionStrategy, HashAlgorithm, HashMap};
use js_sys::Array;
use wasm_bindgen::prelude::*;

fn to_js_error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}

fn parse_strategy(strategy: &str) -> Result<CollisionStrategy, JsValue> {
    match strategy {
        "chaining" => Ok(CollisionStrategy::Chaining),
        "open_addressing" | "open-addressing" | "open" => Ok(CollisionStrategy::OpenAddressing),
        other => Err(to_js_error(format!("unknown collision strategy: {other}"))),
    }
}

fn parse_hash_algorithm(hash_fn: &str) -> Result<HashAlgorithm, JsValue> {
    match hash_fn {
        "siphash" | "siphash_2_4" => Ok(HashAlgorithm::SipHash24),
        "fnv1a" | "fnv1a_32" => Ok(HashAlgorithm::Fnv1a32),
        "murmur3" | "murmur3_32" => Ok(HashAlgorithm::Murmur3_32),
        "djb2" => Ok(HashAlgorithm::Djb2),
        other => Err(to_js_error(format!("unknown hash function: {other}"))),
    }
}

fn entry_array(key: String, value: String) -> Array {
    let entry = Array::new();
    entry.push(&JsValue::from_str(&key));
    entry.push(&JsValue::from_str(&value));
    entry
}

fn entries_array(entries: impl IntoIterator<Item = (String, String)>) -> Array {
    let array = Array::new();
    for (key, value) in entries {
        array.push(&entry_array(key, value));
    }
    array
}

#[wasm_bindgen]
pub struct WasmHashMap {
    inner: HashMap<String, String>,
}

#[wasm_bindgen]
impl WasmHashMap {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: HashMap::default(),
        }
    }

    #[wasm_bindgen(js_name = "withOptions")]
    pub fn with_options(capacity: usize, strategy: &str, hash_fn: &str) -> Result<Self, JsValue> {
        Ok(Self {
            inner: HashMap::with_hash_fn(
                capacity,
                parse_strategy(strategy)?,
                parse_hash_algorithm(hash_fn)?,
            ),
        })
    }

    pub fn set(&mut self, key: &str, value: &str) {
        let inner = mem::take(&mut self.inner);
        self.inner = inner.set(key.to_string(), value.to_string());
    }

    #[wasm_bindgen(js_name = "delete")]
    pub fn delete(&mut self, key: &str) -> bool {
        let key = key.to_string();
        let existed = self.inner.has(&key);
        let inner = mem::take(&mut self.inner);
        self.inner = inner.delete(&key);
        existed
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.inner.get(&key.to_string()).cloned()
    }

    pub fn has(&self, key: &str) -> bool {
        self.inner.has(&key.to_string())
    }

    pub fn size(&self) -> usize {
        self.inner.size()
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    #[wasm_bindgen(js_name = "loadFactor")]
    pub fn load_factor(&self) -> f64 {
        self.inner.load_factor()
    }

    #[wasm_bindgen(js_name = "needsResize")]
    pub fn needs_resize(&self) -> bool {
        self.inner.needs_resize()
    }

    pub fn strategy(&self) -> String {
        match self.inner.strategy() {
            CollisionStrategy::Chaining => "chaining".to_string(),
            CollisionStrategy::OpenAddressing => "open_addressing".to_string(),
        }
    }

    #[wasm_bindgen(js_name = "hashAlgorithm")]
    pub fn hash_algorithm(&self) -> String {
        match self.inner.hash_algorithm() {
            HashAlgorithm::SipHash24 => "siphash_2_4".to_string(),
            HashAlgorithm::Fnv1a32 => "fnv1a_32".to_string(),
            HashAlgorithm::Murmur3_32 => "murmur3_32".to_string(),
            HashAlgorithm::Djb2 => "djb2".to_string(),
        }
    }

    pub fn keys(&self) -> Vec<String> {
        self.inner.keys()
    }

    pub fn values(&self) -> Vec<String> {
        self.inner.values()
    }

    pub fn entries(&self) -> Array {
        entries_array(self.inner.entries())
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl Default for WasmHashMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_hash_map_wraps_core_operations() {
        let mut map = WasmHashMap::new();
        map.set("alpha", "one");
        map.set("beta", "two");

        assert_eq!(map.get("alpha"), Some("one".to_string()));
        assert!(map.has("beta"));
        assert_eq!(map.delete("beta"), true);
        assert!(!map.has("beta"));
        assert_eq!(map.size(), 1);
    }

    #[test]
    fn wasm_hash_map_exposes_entries_and_configuration() {
        let map = WasmHashMap::with_options(4, "open_addressing", "djb2").unwrap();
        assert_eq!(map.strategy(), "open_addressing");
        assert_eq!(map.hash_algorithm(), "djb2");
        assert_eq!(map.inner.entries().len(), 0);
        assert_eq!(map.capacity(), 4);
    }
}
