use heap::{
    heap_sort as core_heap_sort, heapify as core_heapify, nlargest as core_nlargest,
    nsmallest as core_nsmallest, MaxHeap, MinHeap,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmMinHeap {
    inner: MinHeap<i32>,
}

#[wasm_bindgen]
impl WasmMinHeap {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: MinHeap::new(),
        }
    }

    #[wasm_bindgen(js_name = "fromValues")]
    pub fn from_values(values: Vec<i32>) -> Self {
        Self {
            inner: MinHeap::from_iterable(values),
        }
    }

    pub fn push(&mut self, value: i32) {
        self.inner.push(value);
    }

    pub fn pop(&mut self) -> Option<i32> {
        self.inner.pop()
    }

    pub fn peek(&self) -> Option<i32> {
        self.inner.peek().copied()
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[wasm_bindgen(js_name = "len")]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[wasm_bindgen(js_name = "toArray")]
    pub fn to_array(&self) -> Vec<i32> {
        self.inner.to_vec()
    }
}

impl Default for WasmMinHeap {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
pub struct WasmMaxHeap {
    inner: MaxHeap<i32>,
}

#[wasm_bindgen]
impl WasmMaxHeap {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: MaxHeap::new(),
        }
    }

    #[wasm_bindgen(js_name = "fromValues")]
    pub fn from_values(values: Vec<i32>) -> Self {
        Self {
            inner: MaxHeap::from_iterable(values),
        }
    }

    pub fn push(&mut self, value: i32) {
        self.inner.push(value);
    }

    pub fn pop(&mut self) -> Option<i32> {
        self.inner.pop()
    }

    pub fn peek(&self) -> Option<i32> {
        self.inner.peek().copied()
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[wasm_bindgen(js_name = "len")]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[wasm_bindgen(js_name = "toArray")]
    pub fn to_array(&self) -> Vec<i32> {
        self.inner.to_vec()
    }
}

impl Default for WasmMaxHeap {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_name = "heapify")]
pub fn heapify_values(values: Vec<i32>) -> Vec<i32> {
    core_heapify(values)
}

#[wasm_bindgen(js_name = "heapSort")]
pub fn heap_sort_values(values: Vec<i32>) -> Vec<i32> {
    core_heap_sort(values)
}

#[wasm_bindgen]
pub fn nlargest(values: Vec<i32>, n: usize) -> Vec<i32> {
    core_nlargest(values, n)
}

#[wasm_bindgen]
pub fn nsmallest(values: Vec<i32>, n: usize) -> Vec<i32> {
    core_nsmallest(values, n)
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_min_heap_wraps_core_heap() {
        let mut heap = WasmMinHeap::from_values(vec![5, 3, 8, 1, 4]);
        assert_eq!(heap.peek(), Some(1));
        heap.push(0);
        assert_eq!(heap.pop(), Some(0));
        assert_eq!(heap.peek(), Some(1));
    }

    #[test]
    fn wasm_max_heap_wraps_core_heap() {
        let mut heap = WasmMaxHeap::from_values(vec![5, 3, 8, 1, 4]);
        assert_eq!(heap.peek(), Some(8));
        heap.push(10);
        assert_eq!(heap.pop(), Some(10));
        assert_eq!(heap.peek(), Some(8));
    }

    #[test]
    fn exported_helpers_match_expected_values() {
        assert_eq!(heap_sort_values(vec![3, 1, 4, 1, 5]), vec![1, 1, 3, 4, 5]);
        assert_eq!(heapify_values(vec![3, 1, 4, 1, 5]).len(), 5);
        assert_eq!(nlargest(vec![3, 1, 4, 1, 5, 9], 2), vec![9, 5]);
        assert_eq!(nsmallest(vec![3, 1, 4, 1, 5, 9], 2), vec![1, 1]);
    }
}
