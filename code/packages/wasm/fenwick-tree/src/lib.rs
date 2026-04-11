use fenwick_tree::{FenwickError, FenwickTree};
use wasm_bindgen::prelude::*;

fn to_js_error(error: FenwickError) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
pub struct WasmFenwickTree {
    inner: FenwickTree,
}

#[wasm_bindgen]
impl WasmFenwickTree {
    #[wasm_bindgen(constructor)]
    pub fn new(size: usize) -> Self {
        Self {
            inner: FenwickTree::new(size),
        }
    }

    #[wasm_bindgen(js_name = "fromValues")]
    pub fn from_values(values: Vec<f64>) -> Self {
        Self {
            inner: FenwickTree::from_iterable(values),
        }
    }

    pub fn update(&mut self, index: usize, delta: f64) -> Result<(), JsValue> {
        self.inner.update(index, delta).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "prefixSum")]
    pub fn prefix_sum(&self, index: usize) -> Result<f64, JsValue> {
        self.inner.prefix_sum(index).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "rangeSum")]
    pub fn range_sum(&self, left: usize, right: usize) -> Result<f64, JsValue> {
        self.inner.range_sum(left, right).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "pointQuery")]
    pub fn point_query(&self, index: usize) -> Result<f64, JsValue> {
        self.inner.point_query(index).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "findKth")]
    pub fn find_kth(&self, target: f64) -> Result<usize, JsValue> {
        self.inner.find_kth(target).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "len")]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[wasm_bindgen(js_name = "bitArray")]
    pub fn bit_array(&self) -> Vec<f64> {
        self.inner.bit_array().to_vec()
    }

    pub fn values(&self) -> Vec<f64> {
        (1..=self.inner.len())
            .map(|index| self.inner.point_query(index).unwrap_or(0.0))
            .collect()
    }
}

impl Default for WasmFenwickTree {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn wasm_wrapper_exposes_queries_and_updates() {
        let mut tree = WasmFenwickTree::from_values(vec![3.0, 2.0, 1.0, 7.0, 4.0]);
        assert_close(tree.prefix_sum(3).unwrap(), 6.0);
        assert_close(tree.range_sum(2, 4).unwrap(), 10.0);
        tree.update(3, 5.0).unwrap();
        assert_close(tree.point_query(3).unwrap(), 6.0);
    }

    #[test]
    fn wasm_wrapper_exposes_find_kth_and_values() {
        let tree = WasmFenwickTree::from_values(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(tree.find_kth(10.0).unwrap(), 4);
        assert_eq!(tree.values(), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }
}
