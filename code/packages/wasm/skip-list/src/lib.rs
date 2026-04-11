use js_sys::Array;
use skip_list::SkipList;
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
pub struct WasmSkipList {
    inner: SkipList<i32, String>,
}

#[wasm_bindgen]
impl WasmSkipList {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: SkipList::new(),
        }
    }

    #[wasm_bindgen(js_name = "withParams")]
    pub fn with_params(max_level: usize, probability: f64) -> Self {
        Self {
            inner: SkipList::with_params(max_level, probability),
        }
    }

    pub fn insert(&mut self, key: i32, value: &str) {
        self.inner.insert(key, value.to_string());
    }

    pub fn delete(&mut self, key: i32) -> bool {
        self.inner.delete(&key)
    }

    pub fn search(&self, key: i32) -> Option<String> {
        self.inner.search(&key)
    }

    pub fn contains(&self, key: i32) -> bool {
        self.inner.contains(&key)
    }

    #[wasm_bindgen(js_name = "containsKey")]
    pub fn contains_key(&self, key: i32) -> bool {
        self.inner.contains_key(&key)
    }

    pub fn rank(&self, key: i32) -> Option<usize> {
        self.inner.rank(&key)
    }

    #[wasm_bindgen(js_name = "byRank")]
    pub fn by_rank(&self, rank: usize) -> Option<i32> {
        self.inner.by_rank(rank)
    }

    #[wasm_bindgen(js_name = "rangeQuery")]
    pub fn range_query(&self, low: i32, high: i32, inclusive: bool) -> Array {
        entries_array(self.inner.range_query(&low, &high, inclusive))
    }

    pub fn range(&self, low: i32, high: i32, inclusive: bool) -> Array {
        entries_array(self.inner.range(&low, &high, inclusive))
    }

    #[wasm_bindgen(js_name = "toArray")]
    pub fn to_array(&self) -> Vec<i32> {
        self.inner.to_list()
    }

    pub fn entries(&self) -> Array {
        entries_array(self.inner.entries())
    }

    pub fn min(&self) -> Option<i32> {
        self.inner.min()
    }

    pub fn max(&self) -> Option<i32> {
        self.inner.max()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn size(&self) -> usize {
        self.inner.size()
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[wasm_bindgen(js_name = "maxLevel")]
    pub fn max_level(&self) -> usize {
        self.inner.max_level()
    }

    pub fn probability(&self) -> f64 {
        self.inner.probability()
    }

    #[wasm_bindgen(js_name = "currentMax")]
    pub fn current_max(&self) -> usize {
        self.inner.current_max()
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        self.inner.to_string()
    }
}

impl Default for WasmSkipList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_skip_list_wraps_core_operations() {
        let mut list = WasmSkipList::new();
        list.insert(3, "three");
        list.insert(1, "one");
        list.insert(2, "two");

        assert_eq!(list.search(1), Some("one".to_string()));
        assert_eq!(list.rank(2), Some(1));
        assert_eq!(list.by_rank(0), Some(1));
        assert!(list.delete(2));
        assert!(!list.contains(2));
    }

    #[test]
    fn wasm_skip_list_exposes_ranges_and_metadata() {
        let mut list = WasmSkipList::with_params(16, 0.5);
        list.insert(10, "ten");
        list.insert(20, "twenty");
        list.insert(30, "thirty");

        assert_eq!(list.min(), Some(10));
        assert_eq!(list.max(), Some(30));
        assert_eq!(list.len(), 3);
        assert_eq!(list.inner.range_query(&10, &30, true).len(), 3);
        assert_eq!(list.to_array(), vec![10, 20, 30]);
    }
}
