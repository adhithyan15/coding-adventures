use std::mem;

use avl_tree::{balance_factor, AVLTree};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmAvlTree {
    inner: AVLTree<i32>,
}

#[wasm_bindgen]
impl WasmAvlTree {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: AVLTree::empty(),
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

    #[wasm_bindgen(js_name = "isValidBst")]
    pub fn is_valid_bst(&self) -> bool {
        self.inner.is_valid_bst()
    }

    #[wasm_bindgen(js_name = "isValidAvl")]
    pub fn is_valid_avl(&self) -> bool {
        self.inner.is_valid_avl()
    }

    #[wasm_bindgen(js_name = "balanceFactor")]
    pub fn balance_factor_value(&self) -> Option<isize> {
        self.inner.root().map(balance_factor)
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

impl Default for WasmAvlTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_avl_tree_wraps_core_operations() {
        let mut tree = WasmAvlTree::new();
        for value in [10, 20, 30, 40, 50, 25] {
            tree.insert(value);
        }
        assert!(tree.contains(25));
        assert_eq!(tree.min_value(), Some(10));
        assert_eq!(tree.max_value(), Some(50));
        assert!(tree.is_valid_avl());
        assert!(tree.is_valid_bst());
    }

    #[test]
    fn wasm_avl_tree_exposes_order_statistics_and_balance() {
        let tree = {
            let mut tree = WasmAvlTree::new();
            for value in [3, 2, 1, 4, 5, 6] {
                tree.insert(value);
            }
            tree
        };
        assert_eq!(tree.kth_smallest(3), Some(3));
        assert_eq!(tree.rank(4), 3);
        assert!(tree.balance_factor_value().is_some());
        assert_eq!(tree.to_sorted_array(), vec![1, 2, 3, 4, 5, 6]);
    }
}
