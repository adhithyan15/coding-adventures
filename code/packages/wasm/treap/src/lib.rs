use std::mem;

use treap::Treap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmTreap {
    inner: Treap<i32>,
}

#[wasm_bindgen]
pub struct WasmTreapSplit {
    left: Treap<i32>,
    right: Treap<i32>,
}

#[wasm_bindgen]
impl WasmTreap {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Treap::empty(),
        }
    }

    #[wasm_bindgen(js_name = "rootValue")]
    pub fn root_value(&self) -> Option<i32> {
        self.inner.root().map(|node| node.key)
    }

    pub fn insert(&mut self, key: i32, priority: Option<f64>) {
        let inner = mem::take(&mut self.inner);
        self.inner = inner.insert(key, priority);
    }

    pub fn delete(&mut self, key: i32) {
        let inner = mem::take(&mut self.inner);
        self.inner = inner.delete(&key);
    }

    pub fn split(&self, key: i32) -> WasmTreapSplit {
        let (left, right) = self.inner.split(&key);
        WasmTreapSplit { left, right }
    }

    #[wasm_bindgen(js_name = "merge")]
    pub fn merge(left: WasmTreap, right: WasmTreap) -> Self {
        Self {
            inner: Treap::merge(left.inner, right.inner),
        }
    }

    pub fn search(&self, key: i32) -> Option<i32> {
        self.inner.search(&key).map(|node| node.key)
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

    pub fn predecessor(&self, key: i32) -> Option<i32> {
        self.inner.predecessor(&key).copied()
    }

    pub fn successor(&self, key: i32) -> Option<i32> {
        self.inner.successor(&key).copied()
    }

    #[wasm_bindgen(js_name = "kthSmallest")]
    pub fn kth_smallest(&self, k: usize) -> Option<i32> {
        self.inner.kth_smallest(k).copied()
    }

    #[wasm_bindgen(js_name = "toSortedArray")]
    pub fn to_sorted_array(&self) -> Vec<i32> {
        self.inner.to_sorted_array()
    }

    #[wasm_bindgen(js_name = "isValidTreap")]
    pub fn is_valid_treap(&self) -> bool {
        self.inner.is_valid_treap()
    }

    pub fn height(&self) -> isize {
        self.inner.height()
    }

    pub fn size(&self) -> usize {
        self.inner.size()
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        format!("{:?}", self.inner)
    }
}

#[wasm_bindgen]
impl WasmTreapSplit {
    pub fn left(&self) -> WasmTreap {
        WasmTreap {
            inner: self.left.clone(),
        }
    }

    pub fn right(&self) -> WasmTreap {
        WasmTreap {
            inner: self.right.clone(),
        }
    }
}

impl Default for WasmTreap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_treap_wraps_core_operations() {
        let mut treap = WasmTreap::new();
        for (key, priority) in [
            (8, Some(0.8)),
            (3, Some(0.7)),
            (10, Some(0.6)),
            (1, Some(0.9)),
        ] {
            treap.insert(key, priority);
        }
        assert!(treap.contains(3));
        assert_eq!(treap.min_key(), Some(1));
        assert_eq!(treap.max_key(), Some(10));
        assert!(treap.is_valid_treap());
    }

    #[test]
    fn wasm_treap_exposes_order_statistics() {
        let mut treap = WasmTreap::new();
        for (key, priority) in [
            (8, Some(0.8)),
            (3, Some(0.7)),
            (10, Some(0.6)),
            (1, Some(0.9)),
            (6, Some(0.5)),
        ] {
            treap.insert(key, priority);
        }
        assert_eq!(treap.kth_smallest(3), Some(6));
        assert_eq!(treap.predecessor(6), Some(3));
        assert_eq!(treap.successor(6), Some(8));
        assert_eq!(treap.to_sorted_array(), vec![1, 3, 6, 8, 10]);
    }

    #[test]
    fn wasm_treap_exposes_split_and_merge() {
        let mut treap = WasmTreap::new();
        for (key, priority) in [
            (8, Some(0.8)),
            (3, Some(0.7)),
            (10, Some(0.6)),
            (1, Some(0.9)),
            (6, Some(0.5)),
        ] {
            treap.insert(key, priority);
        }
        let split = treap.split(6);
        assert!(split.left().to_sorted_array().iter().all(|key| *key <= 6));
        assert!(split.right().to_sorted_array().iter().all(|key| *key > 6));
        let merged = WasmTreap::merge(split.left(), split.right());
        assert!(merged.is_valid_treap());
        assert_eq!(merged.to_sorted_array(), vec![1, 3, 6, 8, 10]);
    }
}
