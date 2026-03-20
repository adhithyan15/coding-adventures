//! Mathematical Activation boundaries for strict structural inference natively.

pub fn sigmoid(x: f64) -> f64 {
    if x < -709.0 { return 0.0; }
    if x > 709.0 { return 1.0; }
    1.0 / (1.0 + (-x).exp())
}

pub fn sigmoid_derivative(x: f64) -> f64 {
    let sig = sigmoid(x);
    sig * (1.0 - sig)
}

pub fn relu(x: f64) -> f64 {
    if x > 0.0 { x } else { 0.0 }
}

pub fn relu_derivative(x: f64) -> f64 {
    if x > 0.0 { 1.0 } else { 0.0 }
}

pub fn tanh(x: f64) -> f64 {
    x.tanh()
}

pub fn tanh_derivative(x: f64) -> f64 {
    let t = x.tanh();
    1.0 - (t * t)
}
