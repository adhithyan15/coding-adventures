//! # ML Framework TF -- TensorFlow-Compatible API
//!
//! This crate provides a TensorFlow-compatible API built on top of
//! `ml-framework-core`. It implements the key TensorFlow abstractions:
//!
//! 1. **tf.constant / tf.Variable** -- Immutable vs mutable tensors
//! 2. **tf.GradientTape** -- Explicit gradient tracking
//! 3. **tf.keras** -- High-level layers, models, optimizers, losses
//! 4. **tf.nn** -- Neural network activation functions
//! 5. **tf.math** -- Element-wise mathematical operations
//! 6. **tf.random** -- Random tensor generation
//! 7. **tf.data** -- Data pipeline utilities
//!
//! # TF vs PyTorch: The Key Difference
//!
//! TensorFlow uses **explicit gradient tracking** via GradientTape:
//! ```text
//! // TF style: you choose what to record
//! let mut tape = GradientTape::new(false);
//! tape.watch(&variable.tensor);
//! // ... operations inside tape scope ...
//! let grads = tape.gradient(&loss, &sources);
//!
//! // PyTorch style: implicit, always-on for requires_grad tensors
//! let loss = model.forward(&input);
//! loss.backward();
//! // gradients are in each parameter's .grad field
//! ```

use ml_framework_core::{Tensor, TensorError};

// =========================================================================
// Variable -- mutable tensor with name and trainability
// =========================================================================

/// A mutable, named Tensor tracked by GradientTape when trainable.
///
/// In TensorFlow, a Variable is the primary way to hold mutable state
/// that persists across calls. Think of it as a named container for a
/// Tensor whose value can change over time.
///
/// ## Variables vs Constants
///
/// - **tf.constant**: Immutable. Once created, the value never changes.
///   Used for input data, hyperparameters, etc.
/// - **tf.Variable**: Mutable. Can be updated in-place via assign().
///   Used for model weights and biases.
pub struct Variable {
    /// The underlying tensor.
    pub tensor: Tensor,
    /// Whether GradientTape should track gradients for this Variable.
    pub trainable: bool,
    /// Human-readable name for debugging and serialization.
    pub name: String,
}

/// Counter for auto-generating unique Variable names.
static mut VARIABLE_COUNTER: usize = 0;

impl Variable {
    /// Create a Variable from a tensor.
    pub fn new(tensor: Tensor, trainable: bool, name: Option<&str>) -> Self {
        let name = match name {
            Some(n) => n.to_string(),
            None => {
                let count = unsafe {
                    let c = VARIABLE_COUNTER;
                    VARIABLE_COUNTER += 1;
                    c
                };
                format!("Variable:{}", count)
            }
        };
        if trainable {
            tensor.set_requires_grad(true);
        }
        Variable { tensor, trainable, name }
    }

    /// Create a Variable from a slice of data.
    pub fn new_from_slice(data: &[f64], shape: &[usize], trainable: bool, name: Option<&str>) -> Self {
        let tensor = Tensor::from_slice(data, shape, trainable, "cpu");
        Variable::new(tensor, trainable, name)
    }

    /// Replace this Variable's value entirely (in-place mutation).
    pub fn assign(&mut self, value: &Tensor) {
        self.tensor.set_data(value.data());
    }

    /// Add delta to this Variable in-place: var += delta.
    pub fn assign_add(&mut self, delta: &Tensor) {
        let current = self.tensor.data();
        let delta_data = delta.data();
        let new_data: Vec<f64> = current.iter().zip(delta_data.iter()).map(|(a, b)| a + b).collect();
        self.tensor.set_data(new_data);
    }

    /// Subtract delta from this Variable in-place: var -= delta.
    pub fn assign_sub(&mut self, delta: &Tensor) {
        let current = self.tensor.data();
        let delta_data = delta.data();
        let new_data: Vec<f64> = current.iter().zip(delta_data.iter()).map(|(a, b)| a - b).collect();
        self.tensor.set_data(new_data);
    }
}

// =========================================================================
// GradientTape -- explicit gradient tracking
// =========================================================================

/// Records operations for automatic differentiation.
///
/// This is TF's primary mechanism for computing gradients. It records
/// operations on watched tensors, then computes gradients via `gradient()`.
///
/// ## The Tape Metaphor
///
/// Think of GradientTape like a cassette tape recorder:
/// 1. You press RECORD (create the tape and watch tensors)
/// 2. All operations on watched tensors are recorded
/// 3. You press STOP and REWIND (call tape.gradient())
///
/// ## One-shot vs Persistent
///
/// By default, a tape is consumed after one `gradient()` call.
/// With `persistent=true`, you can call `gradient()` multiple times.
pub struct GradientTape {
    persistent: bool,
    used: bool,
}

impl GradientTape {
    /// Create a new GradientTape.
    ///
    /// If persistent is true, the tape can be used for multiple gradient() calls.
    pub fn new(persistent: bool) -> Self {
        GradientTape { persistent, used: false }
    }

    /// Explicitly watch a tensor for gradient computation.
    ///
    /// By default, GradientTape only watches Variables with trainable=true.
    /// Call watch() to track constants or non-trainable Variables.
    pub fn watch(&mut self, tensor: &Tensor) {
        tensor.set_requires_grad(true);
    }

    /// Compute gradients of target with respect to sources.
    ///
    /// Returns a list of gradient tensors, one per source. If a source
    /// has no gradient path to the target, its entry is None.
    pub fn gradient(
        &mut self,
        target: &Tensor,
        sources: &[&Tensor],
    ) -> Result<Vec<Option<Vec<f64>>>, TensorError> {
        if self.used && !self.persistent {
            return Err(TensorError::OperationError(
                "GradientTape.gradient() can only be called once on a non-persistent tape. \
                 Set persistent=true for multiple calls.".to_string()
            ));
        }
        self.used = true;

        // Ensure sources have requires_grad
        for source in sources {
            source.set_requires_grad(true);
        }

        // Clear existing gradients
        for source in sources {
            source.zero_grad();
        }

        // Run backward pass
        target.backward(None)?;

        // Collect gradients from each source
        Ok(sources.iter().map(|s| s.grad_data()).collect())
    }
}

// =========================================================================
// Top-level tensor creation functions
// =========================================================================

/// Create an immutable tensor (no gradient tracking).
pub fn constant(data: &[f64], shape: &[usize]) -> Tensor {
    Tensor::from_slice(data, shape, false, "cpu")
}

/// Create a tensor filled with zeros.
pub fn zeros(shape: &[usize]) -> Tensor {
    Tensor::zeros(shape, "cpu")
}

/// Create a tensor filled with ones.
pub fn ones(shape: &[usize]) -> Tensor {
    Tensor::ones(shape, "cpu")
}

/// Create an n x n identity matrix.
pub fn eye(n: usize) -> Tensor {
    Tensor::eye(n, "cpu")
}

/// Create a 1-D tensor with evenly spaced values (like tf.range).
pub fn range_(start: f64, limit: f64, delta: f64) -> Tensor {
    Tensor::arange(start, limit, delta, "cpu")
}

// =========================================================================
// Top-level math operations
// =========================================================================

/// Matrix multiplication: C = A @ B.
pub fn matmul(a: &Tensor, b: &Tensor) -> Tensor {
    a.matmul(b)
}

/// Element-wise addition.
pub fn add(a: &Tensor, b: &Tensor) -> Tensor {
    a.add(b)
}

/// Element-wise multiplication.
pub fn multiply(a: &Tensor, b: &Tensor) -> Tensor {
    a.mul(b)
}

/// Sum elements, optionally along an axis.
///
/// In TensorFlow, 'axis' replaces PyTorch's 'dim'.
pub fn reduce_sum(x: &Tensor, axis: Option<usize>, keepdims: bool) -> Tensor {
    x.sum(axis, keepdims)
}

/// Mean of elements, optionally along an axis.
pub fn reduce_mean(x: &Tensor, axis: Option<usize>, keepdims: bool) -> Tensor {
    x.mean(axis, keepdims)
}

/// Reshape a tensor to a new shape.
pub fn reshape(x: &Tensor, shape: &[usize]) -> Tensor {
    x.reshape(shape)
}

/// Transpose a 2-D tensor.
pub fn transpose(x: &Tensor) -> Tensor {
    x.t()
}

/// Clamp tensor values to [min, max].
pub fn clip_by_value(x: &Tensor, min_val: f64, max_val: f64) -> Tensor {
    x.clamp(Some(min_val), Some(max_val))
}

// =========================================================================
// nn module -- activation functions
// =========================================================================

/// Neural network activation functions (stateless functional versions).
pub mod nn {
    use ml_framework_core::Tensor;

    /// ReLU: y = max(0, x).
    pub fn relu(x: &Tensor) -> Tensor { x.relu() }

    /// Sigmoid: y = 1 / (1 + e^(-x)).
    pub fn sigmoid(x: &Tensor) -> Tensor { x.sigmoid() }

    /// Softmax along the specified axis.
    pub fn softmax(x: &Tensor, axis: usize) -> Tensor { x.softmax(axis) }

    /// GELU activation (used in transformers).
    pub fn gelu(x: &Tensor) -> Tensor { x.gelu() }
}

// =========================================================================
// math module
// =========================================================================

/// Element-wise mathematical operations.
pub mod math_ops {
    use ml_framework_core::Tensor;

    /// Natural logarithm: y = ln(x).
    pub fn log(x: &Tensor) -> Tensor { x.log() }

    /// Exponential: y = e^x.
    pub fn exp(x: &Tensor) -> Tensor { x.exp() }

    /// Square root: y = sqrt(x).
    pub fn sqrt(x: &Tensor) -> Tensor { x.sqrt() }

    /// Absolute value: y = |x|.
    pub fn abs(x: &Tensor) -> Tensor { x.abs() }
}

// =========================================================================
// random module
// =========================================================================

/// Random tensor generation.
pub mod random {
    use ml_framework_core::Tensor;

    /// Generate a tensor of random values from a normal distribution.
    pub fn normal(shape: &[usize], mean: f64, stddev: f64) -> Tensor {
        let t = Tensor::randn(shape, "cpu");
        if stddev != 1.0 || mean != 0.0 {
            let data: Vec<f64> = t.data().iter().map(|x| x * stddev + mean).collect();
            Tensor::from_slice(&data, shape, false, "cpu")
        } else {
            t
        }
    }
}

// =========================================================================
// keras module -- high-level API
// =========================================================================

/// High-level Keras-style API for building and training neural networks.
///
/// This module provides layers, models, optimizers, losses, metrics,
/// callbacks, and activations matching the TensorFlow Keras API.
pub mod keras {
    use ml_framework_core::{Tensor, Parameter};

    // ----- Activations -----

    /// Activation function lookup by name.
    pub fn get_activation(name: Option<&str>) -> Box<dyn Fn(&Tensor) -> Tensor> {
        match name {
            None | Some("linear") => Box::new(|x: &Tensor| x.clone()),
            Some("relu") => Box::new(|x: &Tensor| x.relu()),
            Some("sigmoid") => Box::new(|x: &Tensor| x.sigmoid()),
            Some("tanh") => Box::new(|x: &Tensor| x.tanh_act()),
            Some("softmax") => Box::new(|x: &Tensor| x.softmax(x.ndim() - 1)),
            Some("gelu") => Box::new(|x: &Tensor| x.gelu()),
            Some(other) => panic!("Unknown activation: {}", other),
        }
    }

    // ----- Layer trait -----

    /// Base trait for all Keras layers.
    pub trait Layer {
        /// Forward computation.
        fn call(&self, x: &Tensor) -> Tensor;
        /// Get all trainable weights.
        fn trainable_weights(&self) -> Vec<&Parameter>;
    }

    // ----- Dense layer -----

    /// Fully connected layer: y = activation(x @ W + b).
    ///
    /// In TensorFlow/Keras, you specify the number of output units
    /// and optionally an activation function.
    pub struct Dense {
        pub kernel: Parameter,
        pub bias: Option<Parameter>,
        pub activation: Box<dyn Fn(&Tensor) -> Tensor>,
        pub units: usize,
    }

    impl Dense {
        /// Create a new Dense layer.
        ///
        /// Uses Xavier/Glorot initialization: stddev = 1 / sqrt(input_dim).
        pub fn new(input_dim: usize, units: usize, activation: Option<&str>, use_bias: bool) -> Self {
            let stddev = 1.0 / (input_dim as f64).sqrt();
            let kernel_data: Vec<f64> = Tensor::randn(&[input_dim, units], "cpu")
                .data().iter().map(|x| x * stddev).collect();
            let kernel = Parameter::new(Tensor::from_slice(&kernel_data, &[input_dim, units], true, "cpu"));

            let bias = if use_bias {
                Some(Parameter::zeros(&[units], "cpu"))
            } else {
                None
            };

            Dense {
                kernel,
                bias,
                activation: get_activation(activation),
                units,
            }
        }
    }

    impl Layer for Dense {
        fn call(&self, x: &Tensor) -> Tensor {
            let mut output = x.matmul(&self.kernel.tensor);
            if let Some(ref bias) = self.bias {
                let batch_size = x.shape()[0];
                let ones_col = Tensor::ones(&[batch_size, 1], "cpu");
                let bias_row = bias.tensor.reshape(&[1, self.units]);
                let bias_broadcast = ones_col.matmul(&bias_row);
                output = output.add(&bias_broadcast);
            }
            (self.activation)(&output)
        }

        fn trainable_weights(&self) -> Vec<&Parameter> {
            let mut weights = vec![&self.kernel];
            if let Some(ref bias) = self.bias {
                weights.push(bias);
            }
            weights
        }
    }

    // ----- Flatten layer -----

    /// Flatten all dimensions except the batch dimension.
    pub struct Flatten;

    impl Layer for Flatten {
        fn call(&self, x: &Tensor) -> Tensor {
            let shape = x.shape();
            if shape.len() <= 2 { return x.clone(); }
            let batch_size = shape[0];
            let flat_size: usize = shape[1..].iter().product();
            x.reshape(&[batch_size, flat_size])
        }

        fn trainable_weights(&self) -> Vec<&Parameter> { vec![] }
    }

    // ----- Dropout layer -----

    /// Randomly zero elements during training for regularization.
    pub struct Dropout {
        pub rate: f64,
        pub training: bool,
    }

    impl Dropout {
        pub fn new(rate: f64) -> Self {
            Dropout { rate, training: true }
        }
    }

    impl Layer for Dropout {
        fn call(&self, x: &Tensor) -> Tensor {
            if !self.training || self.rate == 0.0 {
                return x.clone();
            }
            // In a real implementation, we'd generate a random mask.
            // For simulation, we pass through (inference mode behavior).
            x.clone()
        }

        fn trainable_weights(&self) -> Vec<&Parameter> { vec![] }
    }

    // ----- Loss functions -----

    /// MSE Loss: mean((y_true - y_pred)^2).
    pub fn mse_loss(y_true: &Tensor, y_pred: &Tensor) -> Tensor {
        let diff = y_pred.sub(y_true);
        let squared = diff.pow(2.0);
        squared.mean(None, false)
    }

    /// MAE Loss: mean(|y_true - y_pred|).
    pub fn mae_loss(y_true: &Tensor, y_pred: &Tensor) -> Tensor {
        let diff = y_pred.sub(y_true);
        diff.abs().mean(None, false)
    }

    // ----- Optimizer trait -----

    /// Base trait for Keras optimizers.
    pub trait Optimizer {
        /// Apply gradients to parameters.
        fn apply_gradients(&mut self, grads_and_vars: &[(Option<Vec<f64>>, &Parameter)]);
    }

    // ----- SGD optimizer -----

    /// Stochastic Gradient Descent with optional momentum.
    pub struct SGD {
        pub learning_rate: f64,
        pub momentum: f64,
        velocities: std::collections::HashMap<usize, Vec<f64>>,
        iterations: usize,
    }

    impl SGD {
        pub fn new(learning_rate: f64, momentum: f64) -> Self {
            SGD { learning_rate, momentum, velocities: std::collections::HashMap::new(), iterations: 0 }
        }
    }

    impl Optimizer for SGD {
        fn apply_gradients(&mut self, grads_and_vars: &[(Option<Vec<f64>>, &Parameter)]) {
            self.iterations += 1;
            for (i, (grad_opt, param)) in grads_and_vars.iter().enumerate() {
                if let Some(grad) = grad_opt {
                    if self.momentum != 0.0 {
                        let v = self.velocities.entry(i).or_insert_with(|| vec![0.0; grad.len()]);
                        for j in 0..v.len() {
                            v[j] = self.momentum * v[j] + grad[j];
                        }
                        let new_data: Vec<f64> = param.data().iter().zip(v.iter())
                            .map(|(w, vj)| w - self.learning_rate * vj).collect();
                        param.set_data(new_data);
                    } else {
                        let new_data: Vec<f64> = param.data().iter().zip(grad.iter())
                            .map(|(w, g)| w - self.learning_rate * g).collect();
                        param.set_data(new_data);
                    }
                }
            }
        }
    }

    // ----- Adam optimizer -----

    /// Adam optimizer (Adaptive Moment Estimation).
    pub struct Adam {
        pub learning_rate: f64,
        pub beta_1: f64,
        pub beta_2: f64,
        pub epsilon: f64,
        m: std::collections::HashMap<usize, Vec<f64>>,
        v: std::collections::HashMap<usize, Vec<f64>>,
        iterations: usize,
    }

    impl Adam {
        pub fn new(learning_rate: f64, beta_1: f64, beta_2: f64, epsilon: f64) -> Self {
            Adam {
                learning_rate, beta_1, beta_2, epsilon,
                m: std::collections::HashMap::new(),
                v: std::collections::HashMap::new(),
                iterations: 0,
            }
        }

        pub fn default() -> Self {
            Self::new(0.001, 0.9, 0.999, 1e-7)
        }
    }

    impl Optimizer for Adam {
        fn apply_gradients(&mut self, grads_and_vars: &[(Option<Vec<f64>>, &Parameter)]) {
            self.iterations += 1;
            let t = self.iterations;
            for (i, (grad_opt, param)) in grads_and_vars.iter().enumerate() {
                if let Some(grad) = grad_opt {
                    let m = self.m.entry(i).or_insert_with(|| vec![0.0; grad.len()]);
                    let v = self.v.entry(i).or_insert_with(|| vec![0.0; grad.len()]);
                    for j in 0..m.len() {
                        let g = grad[j];
                        m[j] = self.beta_1 * m[j] + (1.0 - self.beta_1) * g;
                        v[j] = self.beta_2 * v[j] + (1.0 - self.beta_2) * g * g;
                    }
                    let bc1 = 1.0 - self.beta_1.powi(t as i32);
                    let bc2 = 1.0 - self.beta_2.powi(t as i32);
                    let new_data: Vec<f64> = param.data().iter().zip(m.iter().zip(v.iter()))
                        .map(|(w, (mj, vj))| {
                            w - self.learning_rate * (mj / bc1) / ((vj / bc2).sqrt() + self.epsilon)
                        }).collect();
                    param.set_data(new_data);
                }
            }
        }
    }

    // ----- Metrics -----

    /// Accuracy metric: fraction of correct predictions.
    pub struct AccuracyMetric {
        correct: usize,
        total: usize,
    }

    impl AccuracyMetric {
        pub fn new() -> Self { AccuracyMetric { correct: 0, total: 0 } }

        pub fn update_state(&mut self, y_true: &Tensor, y_pred: &Tensor) {
            let pred_data = y_pred.data();
            let true_data = y_true.data();
            if y_pred.ndim() == 2 {
                let num_classes = y_pred.shape()[y_pred.ndim() - 1];
                let batch_size = y_pred.shape()[0];
                for i in 0..batch_size {
                    let start = i * num_classes;
                    let row = &pred_data[start..start + num_classes];
                    let pred_class = row.iter().enumerate()
                        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                        .map(|(idx, _)| idx).unwrap();
                    let true_class = true_data[i] as usize;
                    if pred_class == true_class { self.correct += 1; }
                    self.total += 1;
                }
            } else {
                for (yt, yp) in true_data.iter().zip(pred_data.iter()) {
                    if yp.round() == yt.round() { self.correct += 1; }
                    self.total += 1;
                }
            }
        }

        pub fn result(&self) -> f64 {
            if self.total == 0 { 0.0 } else { self.correct as f64 / self.total as f64 }
        }

        pub fn reset_state(&mut self) { self.correct = 0; self.total = 0; }
    }

    // ----- Callbacks -----

    /// History callback: records training metrics per epoch.
    pub struct History {
        pub history: std::collections::HashMap<String, Vec<f64>>,
    }

    impl History {
        pub fn new() -> Self { History { history: std::collections::HashMap::new() } }

        pub fn on_epoch_end(&mut self, logs: &std::collections::HashMap<String, f64>) {
            for (key, value) in logs {
                self.history.entry(key.clone()).or_insert_with(Vec::new).push(*value);
            }
        }
    }

    /// EarlyStopping callback.
    pub struct EarlyStopping {
        pub monitor: String,
        pub patience: usize,
        pub min_delta: f64,
        pub stop_training: bool,
        best: Option<f64>,
        wait: usize,
    }

    impl EarlyStopping {
        pub fn new(monitor: &str, patience: usize, min_delta: f64) -> Self {
            EarlyStopping {
                monitor: monitor.to_string(), patience, min_delta,
                stop_training: false, best: None, wait: 0,
            }
        }

        pub fn on_epoch_end(&mut self, logs: &std::collections::HashMap<String, f64>) {
            if let Some(&current) = logs.get(&self.monitor) {
                match self.best {
                    None => { self.best = Some(current); }
                    Some(best) => {
                        if current < best - self.min_delta {
                            self.best = Some(current);
                            self.wait = 0;
                        } else {
                            self.wait += 1;
                            if self.wait >= self.patience {
                                self.stop_training = true;
                            }
                        }
                    }
                }
            }
        }
    }

    // ----- Dataset -----

    /// A sequence of elements that can be iterated over.
    pub struct Dataset {
        elements: Vec<(Tensor, Tensor)>,
    }

    impl Dataset {
        /// Create a Dataset by slicing two tensors along the first dimension.
        pub fn from_tensor_slices(x: &Tensor, y: &Tensor) -> Self {
            let n = x.shape()[0];
            let x_data = x.data();
            let y_data = y.data();
            let x_inner: usize = x.shape()[1..].iter().product::<usize>().max(1);
            let y_inner: usize = y.shape()[1..].iter().product::<usize>().max(1);
            let x_inner_shape: Vec<usize> = if x.ndim() > 1 { x.shape()[1..].to_vec() } else { vec![1] };
            let y_inner_shape: Vec<usize> = if y.ndim() > 1 { y.shape()[1..].to_vec() } else { vec![1] };

            let mut elements = Vec::with_capacity(n);
            for i in 0..n {
                let x_slice = Tensor::from_slice(
                    &x_data[i * x_inner..(i + 1) * x_inner], &x_inner_shape, false, "cpu"
                );
                let y_slice = Tensor::from_slice(
                    &y_data[i * y_inner..(i + 1) * y_inner], &y_inner_shape, false, "cpu"
                );
                elements.push((x_slice, y_slice));
            }
            Dataset { elements }
        }

        /// Group elements into batches.
        pub fn batch(self, batch_size: usize) -> Vec<(Tensor, Tensor)> {
            let mut batches = Vec::new();
            for chunk in self.elements.chunks(batch_size) {
                let x_data: Vec<f64> = chunk.iter().flat_map(|(x, _)| x.data()).collect();
                let y_data: Vec<f64> = chunk.iter().flat_map(|(_, y)| y.data()).collect();
                let x_inner = chunk[0].0.shape();
                let y_inner = chunk[0].1.shape();
                let batch_len = chunk.len();
                let mut x_shape = vec![batch_len];
                x_shape.extend_from_slice(&x_inner);
                let mut y_shape = vec![batch_len];
                y_shape.extend_from_slice(&y_inner);
                batches.push((
                    Tensor::from_slice(&x_data, &x_shape, false, "cpu"),
                    Tensor::from_slice(&y_data, &y_shape, false, "cpu"),
                ));
            }
            batches
        }

        /// Get the number of elements.
        pub fn len(&self) -> usize { self.elements.len() }

        /// Check if empty.
        pub fn is_empty(&self) -> bool { self.elements.is_empty() }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_creation() {
        let v = Variable::new_from_slice(&[1.0, 2.0, 3.0], &[3], true, Some("w"));
        assert_eq!(v.name, "w");
        assert!(v.trainable);
        assert_eq!(v.tensor.data(), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_variable_assign() {
        let mut v = Variable::new_from_slice(&[1.0, 2.0], &[2], true, None);
        let new_val = Tensor::from_slice(&[10.0, 20.0], &[2], false, "cpu");
        v.assign(&new_val);
        assert_eq!(v.tensor.data(), vec![10.0, 20.0]);
    }

    #[test]
    fn test_variable_assign_add() {
        let mut v = Variable::new_from_slice(&[1.0, 2.0], &[2], true, None);
        let delta = Tensor::from_slice(&[0.1, 0.1], &[2], false, "cpu");
        v.assign_add(&delta);
        let data = v.tensor.data();
        assert!((data[0] - 1.1).abs() < 1e-10);
        assert!((data[1] - 2.1).abs() < 1e-10);
    }

    #[test]
    fn test_variable_assign_sub() {
        let mut v = Variable::new_from_slice(&[1.0, 2.0], &[2], true, None);
        let delta = Tensor::from_slice(&[0.5, 0.5], &[2], false, "cpu");
        v.assign_sub(&delta);
        let data = v.tensor.data();
        assert!((data[0] - 0.5).abs() < 1e-10);
        assert!((data[1] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_gradient_tape_basic() {
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], true, "cpu");
        let mut tape = GradientTape::new(false);
        tape.watch(&x);
        let y = x.pow(2.0);
        let loss = reduce_sum(&y, None, false);
        let grads = tape.gradient(&loss, &[&x]).unwrap();
        // d(x^2)/dx = 2x
        let grad = grads[0].as_ref().unwrap();
        assert!((grad[0] - 2.0).abs() < 1e-10);
        assert!((grad[1] - 4.0).abs() < 1e-10);
        assert!((grad[2] - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_gradient_tape_non_persistent_error() {
        let x = Tensor::from_slice(&[1.0], &[1], true, "cpu");
        let mut tape = GradientTape::new(false);
        tape.watch(&x);
        let y = x.pow(2.0);
        let _ = tape.gradient(&y, &[&x]).unwrap();
        // Second call should fail on non-persistent tape
        assert!(tape.gradient(&y, &[&x]).is_err());
    }

    #[test]
    fn test_gradient_tape_persistent() {
        let x = Tensor::from_slice(&[2.0], &[1], true, "cpu");
        let mut tape = GradientTape::new(true);
        tape.watch(&x);
        let y = x.pow(2.0);
        let loss = y.sum(None, false);
        let _ = tape.gradient(&loss, &[&x]).unwrap();
        // Second call should succeed on persistent tape
        x.zero_grad();
        let _ = tape.gradient(&loss, &[&x]).unwrap();
    }

    #[test]
    fn test_constant() {
        let c = constant(&[1.0, 2.0], &[2]);
        assert!(!c.requires_grad());
        assert_eq!(c.data(), vec![1.0, 2.0]);
    }

    #[test]
    fn test_reduce_sum() {
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[2, 2], false, "cpu");
        let s = reduce_sum(&x, None, false);
        assert_eq!(s.data(), vec![10.0]);
    }

    #[test]
    fn test_reduce_mean() {
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[4], false, "cpu");
        let m = reduce_mean(&x, None, false);
        assert_eq!(m.data(), vec![2.5]);
    }

    #[test]
    fn test_nn_relu() {
        let x = Tensor::from_slice(&[-1.0, 0.0, 1.0], &[3], false, "cpu");
        let y = nn::relu(&x);
        assert_eq!(y.data(), vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_nn_sigmoid() {
        let x = Tensor::from_slice(&[0.0], &[1], false, "cpu");
        let y = nn::sigmoid(&x);
        assert!((y.data()[0] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_math_log() {
        let x = Tensor::from_slice(&[1.0, std::f64::consts::E], &[2], false, "cpu");
        let y = math_ops::log(&x);
        assert!((y.data()[0]).abs() < 1e-10);
        assert!((y.data()[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_math_exp() {
        let x = Tensor::from_slice(&[0.0, 1.0], &[2], false, "cpu");
        let y = math_ops::exp(&x);
        assert!((y.data()[0] - 1.0).abs() < 1e-10);
        assert!((y.data()[1] - std::f64::consts::E).abs() < 1e-10);
    }

    #[test]
    fn test_random_normal() {
        let t = random::normal(&[2, 3], 0.0, 1.0);
        assert_eq!(t.shape(), vec![2, 3]);
        assert_eq!(t.numel(), 6);
    }

    #[test]
    fn test_clip_by_value() {
        let x = Tensor::from_slice(&[-2.0, 0.5, 3.0], &[3], false, "cpu");
        let y = clip_by_value(&x, 0.0, 1.0);
        assert_eq!(y.data(), vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn test_keras_dense() {
        let layer = keras::Dense::new(3, 2, Some("relu"), true);
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0], &[1, 3], false, "cpu");
        let y = keras::Layer::call(&layer, &x);
        assert_eq!(y.shape(), vec![1, 2]);
    }

    #[test]
    fn test_keras_flatten() {
        let layer = keras::Flatten;
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[1, 2, 3], false, "cpu");
        let y = keras::Layer::call(&layer, &x);
        assert_eq!(y.shape(), vec![1, 6]);
    }

    #[test]
    fn test_keras_mse_loss() {
        let y_true = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], false, "cpu");
        let y_pred = Tensor::from_slice(&[1.5, 2.5, 3.5], &[3], false, "cpu");
        let loss = keras::mse_loss(&y_true, &y_pred);
        assert!((loss.item().unwrap() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_keras_sgd() {
        let p = ml_framework_core::Parameter::new(
            Tensor::from_slice(&[1.0, 2.0], &[2], true, "cpu")
        );
        let mut sgd = keras::SGD::new(0.1, 0.0);
        let grads = vec![1.0, 1.0];
        keras::Optimizer::apply_gradients(&mut sgd, &[(Some(grads), &p)]);
        let data = p.data();
        assert!((data[0] - 0.9).abs() < 1e-10);
        assert!((data[1] - 1.9).abs() < 1e-10);
    }

    #[test]
    fn test_keras_adam() {
        let p = ml_framework_core::Parameter::new(
            Tensor::from_slice(&[1.0], &[1], true, "cpu")
        );
        let mut adam = keras::Adam::default();
        let grads = vec![1.0];
        keras::Optimizer::apply_gradients(&mut adam, &[(Some(grads), &p)]);
        // After one step, parameter should have changed
        assert!(p.data()[0] != 1.0);
    }

    #[test]
    fn test_keras_accuracy_metric() {
        let mut acc = keras::AccuracyMetric::new();
        let y_true = Tensor::from_slice(&[1.0, 0.0, 1.0], &[3], false, "cpu");
        let y_pred = Tensor::from_slice(&[0.9, 0.1, 0.8], &[3], false, "cpu");
        acc.update_state(&y_true, &y_pred);
        assert_eq!(acc.result(), 1.0); // all rounded correctly
    }

    #[test]
    fn test_keras_history() {
        let mut history = keras::History::new();
        let mut logs = std::collections::HashMap::new();
        logs.insert("loss".to_string(), 0.5);
        history.on_epoch_end(&logs);
        logs.insert("loss".to_string(), 0.3);
        history.on_epoch_end(&logs);
        assert_eq!(history.history["loss"], vec![0.5, 0.3]);
    }

    #[test]
    fn test_keras_early_stopping() {
        let mut es = keras::EarlyStopping::new("val_loss", 2, 0.0);
        let mut logs = std::collections::HashMap::new();
        logs.insert("val_loss".to_string(), 0.5);
        es.on_epoch_end(&logs);
        assert!(!es.stop_training);
        logs.insert("val_loss".to_string(), 0.6);
        es.on_epoch_end(&logs);
        assert!(!es.stop_training);
        logs.insert("val_loss".to_string(), 0.7);
        es.on_epoch_end(&logs);
        assert!(es.stop_training); // patience=2, no improvement for 2 epochs
    }

    #[test]
    fn test_dataset() {
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[3, 2], false, "cpu");
        let y = Tensor::from_slice(&[0.0, 1.0, 1.0], &[3], false, "cpu");
        let ds = keras::Dataset::from_tensor_slices(&x, &y);
        assert_eq!(ds.len(), 3);
        let batches = ds.batch(2);
        assert_eq!(batches.len(), 2); // 2 + 1
    }
}
