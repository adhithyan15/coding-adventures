use std::mem;

use hash_set::HashSet;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmHashSet {
    inner: HashSet<String>,
}

#[wasm_bindgen]
impl WasmHashSet {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: HashSet::new(),
        }
    }

    #[wasm_bindgen(js_name = "withOptions")]
    pub fn with_options(capacity: usize, strategy: &str, hash_fn: &str) -> Self {
        Self {
            inner: HashSet::with_options(capacity, strategy, hash_fn),
        }
    }

    #[wasm_bindgen(js_name = "fromValues")]
    pub fn from_values(values: Vec<String>) -> Self {
        Self {
            inner: HashSet::from_list(values),
        }
    }

    pub fn add(&mut self, element: &str) {
        let inner = mem::take(&mut self.inner);
        self.inner = inner.add(element.to_string());
    }

    #[wasm_bindgen(js_name = "remove")]
    pub fn remove(&mut self, element: &str) -> bool {
        let element = element.to_string();
        let existed = self.inner.contains(&element);
        let inner = mem::take(&mut self.inner);
        self.inner = inner.remove(&element);
        existed
    }

    pub fn discard(&mut self, element: &str) -> bool {
        self.remove(element)
    }

    pub fn contains(&self, element: &str) -> bool {
        self.inner.contains(&element.to_string())
    }

    pub fn size(&self) -> usize {
        self.inner.size()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[wasm_bindgen(js_name = "toArray")]
    pub fn to_array(&self) -> Vec<String> {
        self.inner.to_list()
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.clone().union(other.inner.clone()),
        }
    }

    pub fn intersection(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.clone().intersection(other.inner.clone()),
        }
    }

    pub fn difference(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.clone().difference(other.inner.clone()),
        }
    }

    #[wasm_bindgen(js_name = "symmetricDifference")]
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.clone().symmetric_difference(other.inner.clone()),
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

impl Default for WasmHashSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_hash_set_wraps_core_operations() {
        let mut set = WasmHashSet::from_values(vec!["a".into(), "b".into()]);
        set.add("c");
        assert!(set.contains("b"));
        assert_eq!(set.remove("b"), true);
        assert!(!set.contains("b"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn wasm_hash_set_exposes_set_algebra() {
        let left = WasmHashSet::from_values(vec!["a".into(), "b".into()]);
        let right = WasmHashSet::from_values(vec!["b".into(), "c".into()]);
        assert!(left.is_subset(&left.union(&right)));
        assert!(left.union(&right).is_superset(&left));
        assert!(left.intersection(&right).contains("b"));
        assert!(left.difference(&right).contains("a"));
        assert!(left.symmetric_difference(&right).contains("c"));
    }
}
