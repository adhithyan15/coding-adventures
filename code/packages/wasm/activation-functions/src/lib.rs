use wasm_bindgen::prelude::*;
use activation_functions;

#[wasm_bindgen]
pub fn sigmoid(x: f64) -> f64 {
    activation_functions::sigmoid(x)
}

#[wasm_bindgen]
pub fn sigmoid_derivative(x: f64) -> f64 {
    activation_functions::sigmoid_derivative(x)
}

#[wasm_bindgen]
pub fn relu(x: f64) -> f64 {
    activation_functions::relu(x)
}

#[wasm_bindgen]
pub fn relu_derivative(x: f64) -> f64 {
    activation_functions::relu_derivative(x)
}

#[wasm_bindgen]
pub fn tanh(x: f64) -> f64 {
    activation_functions::tanh(x)
}

#[wasm_bindgen]
pub fn tanh_derivative(x: f64) -> f64 {
    activation_functions::tanh_derivative(x)
}
