use js_sys::{Array, Map};
use radix_tree::RadixTree;
use wasm_bindgen::prelude::*;

fn string_array(values: impl IntoIterator<Item = String>) -> Array {
    let array = Array::new();
    for value in values {
        array.push(&JsValue::from_str(&value));
    }
    array
}

fn to_js_map(entries: impl IntoIterator<Item = (String, String)>) -> Map {
    let map = Map::new();
    for (key, value) in entries {
        map.set(&JsValue::from_str(&key), &JsValue::from_str(&value));
    }
    map
}

#[wasm_bindgen]
pub struct WasmRadixTree {
    inner: RadixTree<String>,
}

#[wasm_bindgen]
impl WasmRadixTree {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RadixTree::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.inner.insert(key, value.to_string());
    }

    pub fn search(&self, key: &str) -> Option<String> {
        self.inner.search(key).cloned()
    }

    #[wasm_bindgen(js_name = "containsKey")]
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    pub fn delete(&mut self, key: &str) -> bool {
        self.inner.delete(key)
    }

    #[wasm_bindgen(js_name = "startsWith")]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.inner.starts_with(prefix)
    }

    #[wasm_bindgen(js_name = "wordsWithPrefix")]
    pub fn words_with_prefix(&self, prefix: &str) -> Array {
        string_array(self.inner.words_with_prefix(prefix))
    }

    #[wasm_bindgen(js_name = "longestPrefixMatch")]
    pub fn longest_prefix_match(&self, key: &str) -> Option<String> {
        self.inner.longest_prefix_match(key)
    }

    pub fn keys(&self) -> Array {
        string_array(self.inner.keys())
    }

    #[wasm_bindgen(js_name = "toMap")]
    pub fn to_map(&self) -> Map {
        to_js_map(self.inner.to_map().into_iter())
    }

    #[wasm_bindgen(js_name = "len")]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[wasm_bindgen(js_name = "nodeCount")]
    pub fn node_count(&self) -> usize {
        self.inner.node_count()
    }
}

impl Default for WasmRadixTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_radix_tree_wraps_insert_search_and_delete() {
        let mut tree = WasmRadixTree::new();
        tree.insert("app", "short");
        tree.insert("apple", "fruit");
        tree.insert("application", "software");

        assert_eq!(tree.search("app"), Some("short".to_string()));
        assert_eq!(tree.search("apple"), Some("fruit".to_string()));
        assert!(tree.delete("app"));
        assert_eq!(tree.search("app"), None);
        assert_eq!(tree.search("apple"), Some("fruit".to_string()));
    }

    #[test]
    fn wasm_radix_tree_exposes_prefix_queries_and_shape() {
        let mut tree = WasmRadixTree::new();
        tree.insert("", "root");
        tree.insert("search", "base");
        tree.insert("searcher", "person");
        tree.insert("searching", "progressive");

        assert!(tree.starts_with("sear"));
        assert!(tree.contains_key(""));
        assert_eq!(
            tree.longest_prefix_match("searching-party"),
            Some("searching".to_string())
        );
        assert_eq!(
            tree.inner.words_with_prefix("search"),
            vec![
                "search".to_string(),
                "searcher".to_string(),
                "searching".to_string()
            ]
        );
        assert!(tree.node_count() >= 4);
    }
}
