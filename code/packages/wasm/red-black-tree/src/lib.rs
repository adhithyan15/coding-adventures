use std::mem;

use red_black_tree::{Color, RBTree};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmRedBlackTree {
    inner: RBTree<i32>,
}

#[wasm_bindgen]
impl WasmRedBlackTree {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RBTree::empty(),
        }
    }

    #[wasm_bindgen(js_name = "rootValue")]
    pub fn root_value(&self) -> Option<i32> {
        self.inner.root().map(|node| node.value)
    }

    #[wasm_bindgen(js_name = "rootColor")]
    pub fn root_color(&self) -> Option<String> {
        self.inner.root().map(|node| match node.color {
            Color::Red => "red".to_string(),
            Color::Black => "black".to_string(),
        })
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

    #[wasm_bindgen(js_name = "toSortedArray")]
    pub fn to_sorted_array(&self) -> Vec<i32> {
        self.inner.to_sorted_array()
    }

    #[wasm_bindgen(js_name = "isValidRb")]
    pub fn is_valid_rb(&self) -> bool {
        self.inner.is_valid_rb()
    }

    #[wasm_bindgen(js_name = "blackHeight")]
    pub fn black_height(&self) -> usize {
        self.inner.black_height()
    }

    pub fn size(&self) -> usize {
        self.inner.to_sorted_array().len()
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl Default for WasmRedBlackTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_red_black_tree_wraps_core_operations() {
        let mut tree = WasmRedBlackTree::new();
        for value in [10, 20, 30, 15, 25] {
            tree.insert(value);
        }
        assert!(tree.contains(25));
        assert_eq!(tree.min_value(), Some(10));
        assert_eq!(tree.max_value(), Some(30));
        assert!(tree.is_valid_rb());
    }

    #[test]
    fn wasm_red_black_tree_exposes_root_color_and_order_statistics() {
        let mut tree = WasmRedBlackTree::new();
        for value in [7, 3, 18, 10, 22, 8, 11, 26] {
            tree.insert(value);
        }
        assert!(matches!(tree.root_color().as_deref(), Some("black")));
        assert_eq!(tree.kth_smallest(3), Some(8));
        assert!(tree.black_height() > 0);
        assert_eq!(tree.to_sorted_array(), vec![3, 7, 8, 10, 11, 18, 22, 26]);
    }
}
