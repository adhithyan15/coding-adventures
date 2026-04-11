use hyperloglog::{HyperLogLog, HyperLogLogError};
use wasm_bindgen::prelude::*;

fn to_js_error(error: HyperLogLogError) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
pub struct WasmHyperLogLog {
    inner: HyperLogLog,
}

#[wasm_bindgen]
impl WasmHyperLogLog {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: HyperLogLog::new(),
        }
    }

    #[wasm_bindgen(js_name = "tryWithPrecision")]
    pub fn try_with_precision(precision: u8) -> Result<Self, JsValue> {
        HyperLogLog::try_with_precision(precision)
            .map(|inner| Self { inner })
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "withPrecision")]
    pub fn with_precision(precision: u8) -> Self {
        Self {
            inner: HyperLogLog::with_precision(precision),
        }
    }

    #[wasm_bindgen(js_name = "addString")]
    pub fn add_string(&mut self, value: &str) {
        self.inner.add_bytes(value.as_bytes());
    }

    #[wasm_bindgen(js_name = "addNumber")]
    pub fn add_number(&mut self, value: i32) {
        self.inner.add(value);
    }

    pub fn count(&self) -> usize {
        self.inner.count()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn precision(&self) -> u8 {
        self.inner.precision()
    }

    #[wasm_bindgen(js_name = "numRegisters")]
    pub fn num_registers(&self) -> usize {
        self.inner.num_registers()
    }

    #[wasm_bindgen(js_name = "errorRate")]
    pub fn error_rate(&self) -> f64 {
        self.inner.error_rate()
    }

    #[wasm_bindgen(js_name = "merge")]
    pub fn merge(&mut self, other: &Self) -> Result<(), JsValue> {
        self.inner = self.inner.try_merge(&other.inner).map_err(to_js_error)?;
        Ok(())
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name = "errorRateForPrecision")]
    pub fn error_rate_for_precision(precision: u8) -> f64 {
        HyperLogLog::error_rate_for_precision(precision)
    }

    #[wasm_bindgen(js_name = "memoryBytes")]
    pub fn memory_bytes(precision: u8) -> usize {
        HyperLogLog::memory_bytes(precision)
    }

    #[wasm_bindgen(js_name = "optimalPrecision")]
    pub fn optimal_precision(desired_error: f64) -> u8 {
        HyperLogLog::optimal_precision(desired_error)
    }
}

impl Default for WasmHyperLogLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_hyperloglog_wraps_core_operations() {
        let mut hll = WasmHyperLogLog::new();
        hll.add_string("alpha");
        hll.add_string("beta");
        hll.add_number(42);
        assert!(hll.count() >= 2);
        assert_eq!(hll.len(), hll.count());
    }

    #[test]
    fn wasm_hyperloglog_supports_precision_and_merge() {
        let mut left = WasmHyperLogLog::with_precision(10);
        let mut right = WasmHyperLogLog::with_precision(10);
        left.add_string("left");
        right.add_string("right");
        left.merge(&right).unwrap();
        assert!(left.count() >= 2);
        assert_eq!(
            WasmHyperLogLog::memory_bytes(10),
            HyperLogLog::memory_bytes(10)
        );
    }
}
