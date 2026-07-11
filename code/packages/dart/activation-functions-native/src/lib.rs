//! # activation-functions-native — a C ABI over the Rust `activation-functions` crate
//!
//! The *native-through-Rust* half of the Dart `activation-functions` package,
//! and the **simplest possible native binding** in the campaign: every function
//! is a pure `double -> double`. There are no strings, no byte buffers, no
//! opaque handles, and nothing to allocate or free — the C ABI passes and
//! returns `f64` (`c_double`) by value.
//!
//! This complements the other native shapes already in the repo:
//! * `caesar-cipher-native` — C-string I/O,
//! * `sha256-native`/`md5-native`/`sha1-native` — byte buffers + opaque handles,
//! * this crate — pure scalar values.

use std::os::raw::c_double;

use activation_functions as af;

/// Generate an `extern "C"` wrapper `af_<name>` that forwards a single `f64`
/// argument to the crate function `af::<name>` and returns its `f64` result.
macro_rules! wrap {
    ($($c_name:ident => $rust_fn:ident),* $(,)?) => {
        $(
            #[no_mangle]
            pub extern "C" fn $c_name(x: c_double) -> c_double {
                af::$rust_fn(x)
            }
        )*
    };
}

wrap! {
    af_linear => linear,
    af_linear_derivative => linear_derivative,
    af_sigmoid => sigmoid,
    af_sigmoid_derivative => sigmoid_derivative,
    af_relu => relu,
    af_relu_derivative => relu_derivative,
    af_leaky_relu => leaky_relu,
    af_leaky_relu_derivative => leaky_relu_derivative,
    af_tanh => tanh,
    af_tanh_derivative => tanh_derivative,
    af_softplus => softplus,
    af_softplus_derivative => softplus_derivative,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-12
    }

    #[test]
    fn wrappers_forward_to_the_crate() {
        assert!(close(af_sigmoid(0.0), 0.5));
        assert!(close(af_sigmoid_derivative(0.0), 0.25));
        assert!(close(af_relu(-3.0), 0.0));
        assert!(close(af_relu(5.0), 5.0));
        assert!(close(af_leaky_relu(-3.0), -0.03));
        assert!(close(af_tanh(1.0), 0.7615941559557649));
        assert!(close(af_tanh_derivative(0.0), 1.0));
        assert!(close(af_linear(5.0), 5.0));
        assert!(close(af_linear_derivative(9.0), 1.0));
        assert!(close(af_softplus(0.0), std::f64::consts::LN_2));
        assert!(close(af_softplus_derivative(1.0), af_sigmoid(1.0)));
    }
}
