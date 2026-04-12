use b_tree::BTree;
use js_sys::Array;
use wasm_bindgen::prelude::*;

fn pair_array(key: i32, value: String) -> Array {
    let entry = Array::new();
    entry.push(&JsValue::from_f64(key as f64));
    entry.push(&JsValue::from_str(&value));
    entry
}

fn entries_array(entries: impl IntoIterator<Item = (i32, String)>) -> Array {
    let array = Array::new();
    for (key, value) in entries {
        array.push(&pair_array(key, value));
    }
    array
}

#[wasm_bindgen]
pub struct WasmBTree {
    inner: BTree<i32, String>,
}

#[wasm_bindgen]
impl WasmBTree {
    #[wasm_bindgen(constructor)]
    pub fn new(t: usize) -> Self {
        Self {
            inner: BTree::new(t),
        }
    }

    pub fn insert(&mut self, key: i32, value: &str) {
        self.inner.insert(key, value.to_string());
    }

    pub fn delete(&mut self, key: i32) {
        self.inner.delete(&key);
    }

    pub fn search(&self, key: i32) -> Option<String> {
        self.inner.search(&key).cloned()
    }

    pub fn contains(&self, key: i32) -> bool {
        self.inner.contains(&key)
    }

    #[wasm_bindgen(js_name = "minKey")]
    pub fn min_key(&self) -> Option<i32> {
        self.inner.min_key().copied()
    }

    #[wasm_bindgen(js_name = "maxKey")]
    pub fn max_key(&self) -> Option<i32> {
        self.inner.max_key().copied()
    }

    #[wasm_bindgen(js_name = "rangeQuery")]
    pub fn range_query(&self, low: i32, high: i32) -> Array {
        entries_array(
            self.inner
                .range_query(&low, &high)
                .into_iter()
                .map(|(key, value)| (*key, value.clone())),
        )
    }

    pub fn inorder(&self) -> Array {
        entries_array(
            self.inner
                .inorder()
                .into_iter()
                .map(|(key, value)| (*key, value.clone())),
        )
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn height(&self) -> usize {
        self.inner.height()
    }

    pub fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        format!(
            "BTree(len={}, height={}, valid={})",
            self.inner.len(),
            self.inner.height(),
            self.inner.is_valid()
        )
    }
}

impl Default for WasmBTree {
    fn default() -> Self {
        Self::new(2)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_b_tree_wraps_core_operations() {
        let mut tree = WasmBTree::new(2);
        tree.insert(5, "five");
        tree.insert(1, "one");
        tree.insert(3, "three");
        assert_eq!(tree.search(3), Some("three".to_string()));
        assert_eq!(tree.min_key(), Some(1));
        assert!(tree.is_valid());
    }

    #[test]
    fn wasm_b_tree_exposes_scans() {
        let mut tree = WasmBTree::new(3);
        tree.insert(10, "ten");
        tree.insert(20, "twenty");
        tree.insert(30, "thirty");
        assert_eq!(tree.inner.range_query(&10, &30).len(), 3);
        assert_eq!(tree.inner.inorder().len(), 3);
        assert!(tree.height() <= 1);
    }
}
