use js_sys::Array;
use trie::Trie;
use wasm_bindgen::prelude::*;

fn string_array(values: impl IntoIterator<Item = String>) -> Array {
    let array = Array::new();
    for value in values {
        array.push(&JsValue::from_str(&value));
    }
    array
}

fn entry_array(key: String, value: String) -> Array {
    let pair = Array::new();
    pair.push(&JsValue::from_str(&key));
    pair.push(&JsValue::from_str(&value));
    pair
}

fn entries_array(entries: impl IntoIterator<Item = (String, String)>) -> Array {
    let array = Array::new();
    for (key, value) in entries {
        array.push(&entry_array(key, value));
    }
    array
}

#[wasm_bindgen]
pub struct WasmTrie {
    inner: Trie<String>,
}

#[wasm_bindgen]
impl WasmTrie {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: Trie::new() }
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
        entries_array(self.inner.words_with_prefix(prefix))
    }

    #[wasm_bindgen(js_name = "allWords")]
    pub fn all_words(&self) -> Array {
        entries_array(self.inner.all_words())
    }

    pub fn keys(&self) -> Array {
        string_array(self.inner.keys())
    }

    #[wasm_bindgen(js_name = "longestPrefixMatch")]
    pub fn longest_prefix_match(&self, string: &str) -> JsValue {
        match self.inner.longest_prefix_match(string) {
            Some((key, value)) => entry_array(key, value).into(),
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen(js_name = "len")]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[wasm_bindgen(js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }
}

impl Default for WasmTrie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_trie_wraps_insert_search_and_delete() {
        let mut trie = WasmTrie::new();
        trie.insert("app", "short");
        trie.insert("apple", "fruit");

        assert_eq!(trie.search("app"), Some("short".to_string()));
        assert_eq!(trie.search("apple"), Some("fruit".to_string()));
        assert!(trie.delete("app"));
        assert_eq!(trie.search("app"), None);
        assert_eq!(trie.search("apple"), Some("fruit".to_string()));
    }

    #[test]
    fn wasm_trie_exposes_prefix_queries_and_invariants() {
        let mut trie = WasmTrie::new();
        trie.insert("", "root");
        trie.insert("cat", "animal");
        trie.insert("cater", "verb");

        assert!(trie.starts_with("cat"));
        assert!(trie.contains_key(""));
        assert!(trie.is_valid());
        assert_eq!(
            trie.inner.words_with_prefix("cat"),
            vec![
                ("cat".to_string(), "animal".to_string()),
                ("cater".to_string(), "verb".to_string())
            ]
        );
        assert_eq!(
            trie.inner.longest_prefix_match("caterpillar"),
            Some(("cater".to_string(), "verb".to_string()))
        );
    }
}
