//! Mathematical Activation boundaries for strict structural inference natively.

const LEAKY_RELU_SLOPE: f64 = 0.01;

pub fn linear(x: f64) -> f64 {
    x
}

pub fn linear_derivative(_x: f64) -> f64 {
    1.0
}

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

pub fn leaky_relu(x: f64) -> f64 {
    if x > 0.0 { x } else { LEAKY_RELU_SLOPE * x }
}

pub fn leaky_relu_derivative(x: f64) -> f64 {
    if x > 0.0 { 1.0 } else { LEAKY_RELU_SLOPE }
}

pub fn tanh(x: f64) -> f64 {
    x.tanh()
}

pub fn tanh_derivative(x: f64) -> f64 {
    let t = x.tanh();
    1.0 - (t * t)
}

pub fn softplus(x: f64) -> f64 {
    (-x.abs()).exp().ln_1p() + x.max(0.0)
}

pub fn softplus_derivative(x: f64) -> f64 {
    sigmoid(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(expected: f64, actual: f64) {
        assert!(
            (expected - actual).abs() <= 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn linear_matches_identity() {
        assert_close(-3.0, linear(-3.0));
        assert_close(0.0, linear(0.0));
        assert_close(5.0, linear(5.0));
        assert_close(1.0, linear_derivative(-3.0));
        assert_close(1.0, linear_derivative(0.0));
        assert_close(1.0, linear_derivative(5.0));
    }

    #[test]
    fn sigmoid_matches_reference_values() {
        assert_close(0.5, sigmoid(0.0));
        assert_close(0.7310585786300049, sigmoid(1.0));
        assert_close(0.2689414213699951, sigmoid(-1.0));
        assert_close(0.9999546021312976, sigmoid(10.0));
        assert_close(0.0, sigmoid(-710.0));
        assert_close(1.0, sigmoid(710.0));
        assert_close(0.25, sigmoid_derivative(0.0));
        assert_close(0.19661193324148185, sigmoid_derivative(1.0));
    }

    #[test]
    fn relu_matches_piecewise_definition() {
        assert_close(5.0, relu(5.0));
        assert_close(0.0, relu(-3.0));
        assert_close(0.0, relu(0.0));
        assert_close(1.0, relu_derivative(5.0));
        assert_close(0.0, relu_derivative(-3.0));
        assert_close(0.0, relu_derivative(0.0));
    }

    #[test]
    fn leaky_relu_keeps_negative_slope() {
        assert_close(5.0, leaky_relu(5.0));
        assert_close(-0.03, leaky_relu(-3.0));
        assert_close(0.0, leaky_relu(0.0));
        assert_close(1.0, leaky_relu_derivative(5.0));
        assert_close(0.01, leaky_relu_derivative(-3.0));
        assert_close(0.01, leaky_relu_derivative(0.0));
    }

    #[test]
    fn tanh_matches_reference_values() {
        assert_close(0.0, tanh(0.0));
        assert_close(0.7615941559557649, tanh(1.0));
        assert_close(-0.7615941559557649, tanh(-1.0));
        assert_close(1.0, tanh_derivative(0.0));
        assert_close(0.41997434161402614, tanh_derivative(1.0));
    }

    #[test]
    fn softplus_matches_reference_values() {
        assert_close(0.6931471805599453, softplus(0.0));
        assert_close(1.3132616875182228, softplus(1.0));
        assert_close(0.31326168751822286, softplus(-1.0));
        assert!(softplus(1000.0) > 999.0);
        assert_close(0.5, softplus_derivative(0.0));
        assert_close(sigmoid(1.0), softplus_derivative(1.0));
        assert_close(sigmoid(-1.0), softplus_derivative(-1.0));
    }
}
