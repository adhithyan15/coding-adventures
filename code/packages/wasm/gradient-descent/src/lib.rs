use wasm_bindgen::prelude::*;
use gradient_descent;

#[wasm_bindgen]
pub fn sgd(weights: &[f64], gradients: &[f64], learning_rate: f64) -> Result<Vec<f64>, JsValue> {
    match gradient_descent::sgd(weights, gradients, learning_rate) {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}
