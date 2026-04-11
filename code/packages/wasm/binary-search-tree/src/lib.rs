use std::mem;

use binary_search_tree::BST;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmBinarySearchTree {
    inner: BST<i32>,
}

#[wasm_bindgen]
impl WasmBinarySearchTree {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: BST::empty(),
        }
    }

    #[wasm_bindgen(js_name = "fromSortedArray")]
    pub fn from_sorted_array(values: Vec<i32>) -> Self {
        Self {
            inner: BST::from_sorted_array(values),
        }
    }

    #[wasm_bindgen(js_name = "rootValue")]
    pub fn root_value(&self) -> Option<i32> {
        self.inner.root().map(|node| node.value)
    }

    pub fn insert(&mut self, value: i32) {
        let inner = mem::take(&mut self.inner);
        self.inner = inner.insert(value);
    }

    pub fn delete(&mut self, value: i32) {
        let inner = mem::take(&mut self.inner);
        self.inner = inner.delete(&value);
    }

    pub fn search(&self, value: i32) -> Option<i32> {
        self.inner.search(&value).map(|node| node.value)
    }

    pub fn contains(&self, value: i32) -> bool {
        self.inner.contains(&value)
    }

    #[wasm_bindgen(js_name = "minValue")]
    pub fn min_value(&self) -> Option<i32> {
        self.inner.min_value().copied()
    }

    #[wasm_bindgen(js_name = "maxValue")]
    pub fn max_value(&self) -> Option<i32> {
        self.inner.max_value().copied()
    }

    pub fn predecessor(&self, value: i32) -> Option<i32> {
        self.inner.predecessor(&value).copied()
    }

    pub fn successor(&self, value: i32) -> Option<i32> {
        self.inner.successor(&value).copied()
    }

    #[wasm_bindgen(js_name = "kthSmallest")]
    pub fn kth_smallest(&self, k: usize) -> Option<i32> {
        self.inner.kth_smallest(k).copied()
    }

    pub fn rank(&self, value: i32) -> usize {
        self.inner.rank(&value)
    }

    #[wasm_bindgen(js_name = "toSortedArray")]
    pub fn to_sorted_array(&self) -> Vec<i32> {
        self.inner.to_sorted_array()
    }

    #[wasm_bindgen(js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.inner.is_valid()
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

impl Default for WasmBinarySearchTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_binary_search_tree_wraps_core_operations() {
        let mut tree = WasmBinarySearchTree::from_sorted_array(vec![1, 3, 5, 7, 9]);
        assert_eq!(tree.search(5), Some(5));
        assert!(tree.contains(7));
        assert_eq!(tree.min_value(), Some(1));
        assert_eq!(tree.max_value(), Some(9));
        tree.insert(6);
        assert!(tree.contains(6));
        tree.delete(3);
        assert!(!tree.contains(3));
    }

    #[test]
    fn wasm_binary_search_tree_exposes_order_statistics() {
        let tree = WasmBinarySearchTree::from_sorted_array(vec![2, 4, 6, 8]);
        assert_eq!(tree.kth_smallest(2), Some(4));
        assert_eq!(tree.rank(6), 2);
        assert_eq!(tree.to_sorted_array(), vec![2, 4, 6, 8]);
        assert!(tree.is_valid());
    }
}
