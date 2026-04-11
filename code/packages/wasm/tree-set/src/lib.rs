use std::mem;

use tree_set::TreeSet;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmTreeSet {
    inner: TreeSet<i32>,
}

#[wasm_bindgen]
impl WasmTreeSet {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: TreeSet::new(),
        }
    }

    pub fn insert(&mut self, value: i32) {
        let inner = mem::take(&mut self.inner);
        self.inner = inner.insert(value);
    }

    pub fn delete(&mut self, value: i32) {
        let inner = mem::take(&mut self.inner);
        self.inner = inner.delete(&value);
    }

    pub fn contains(&self, value: i32) -> bool {
        self.inner.contains(&value)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
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

    #[wasm_bindgen(js_name = "range")]
    pub fn range(&self, min: i32, max: i32, inclusive: bool) -> Vec<i32> {
        self.inner.range(&min, &max, inclusive)
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.union(&other.inner),
        }
    }

    pub fn intersection(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.intersection(&other.inner),
        }
    }

    pub fn difference(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.difference(&other.inner),
        }
    }

    #[wasm_bindgen(js_name = "symmetricDifference")]
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.symmetric_difference(&other.inner),
        }
    }

    #[wasm_bindgen(js_name = "isSubset")]
    pub fn is_subset(&self, other: &Self) -> bool {
        self.inner.is_subset(&other.inner)
    }

    #[wasm_bindgen(js_name = "isSuperset")]
    pub fn is_superset(&self, other: &Self) -> bool {
        self.inner.is_superset(&other.inner)
    }

    #[wasm_bindgen(js_name = "isDisjoint")]
    pub fn is_disjoint(&self, other: &Self) -> bool {
        self.inner.is_disjoint(&other.inner)
    }

    pub fn equals(&self, other: &Self) -> bool {
        self.inner.equals(&other.inner)
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl Default for WasmTreeSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_tree_set_wraps_core_operations() {
        let mut set = WasmTreeSet::new();
        for value in [7, 3, 9, 1, 5, 3] {
            set.insert(value);
        }
        assert!(set.contains(5));
        assert_eq!(set.len(), 5);
        assert_eq!(set.min_value(), Some(1));
        assert_eq!(set.max_value(), Some(9));
        assert_eq!(set.to_sorted_array(), vec![1, 3, 5, 7, 9]);
        assert_eq!(set.range(3, 7, true), vec![3, 5, 7]);
    }

    #[test]
    fn wasm_tree_set_exposes_set_algebra() {
        let mut left = WasmTreeSet::new();
        for value in [1, 2, 3, 5] {
            left.insert(value);
        }
        let mut right = WasmTreeSet::new();
        for value in [3, 4, 5, 6] {
            right.insert(value);
        }

        assert_eq!(left.union(&right).to_sorted_array(), vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(left.intersection(&right).to_sorted_array(), vec![3, 5]);
        assert_eq!(left.difference(&right).to_sorted_array(), vec![1, 2]);
        assert!(left.is_subset(&left.union(&right)));
        assert!(left.is_disjoint(&WasmTreeSet::new().union(&WasmTreeSet::new())));
        assert!(left.symmetric_difference(&right).contains(4));
    }
}
