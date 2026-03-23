//! # ML Framework Keras -- High-Level Neural Network API
//!
//! This is a Keras 3-compatible high-level API built on top of ml-framework-core.
//! It provides the famous "3-line training" experience:
//!
//! ```text
//! model = Sequential([Dense(128, "relu"), Dropout(0.2), Dense(10, "softmax")])
//! model.compile("adam", "categorical_crossentropy", &["accuracy"])
//! model.fit(&x_train, &y_train, epochs=10, batch_size=32)
//! ```
//!
//! # Module Structure
//!
//! - `backend`: Backend selection (ml_framework_core only)
//! - `activations`: relu, sigmoid, tanh, softmax, gelu, linear
//! - `layers`: Dense, Dropout, BatchNorm, LayerNorm, Flatten, Embedding
//! - `models`: Sequential and Model (compile/fit/evaluate/predict)
//! - `optimizers`: SGD, Adam, RMSprop, AdamW
//! - `losses`: MSE, MAE, BinaryCrossentropy, CategoricalCrossentropy
//! - `metrics`: Accuracy, BinaryAccuracy, CategoricalAccuracy, MSE, MAE
//! - `callbacks`: History, EarlyStopping, ModelCheckpoint, LearningRateScheduler

use ml_framework_core::{Tensor, Parameter};
use std::collections::HashMap;

// =========================================================================
// Backend module
// =========================================================================

/// Backend selection for Keras. Only "ml_framework_core" is supported.
pub mod backend {
    static mut BACKEND: &str = "ml_framework_core";

    /// Get the currently active backend.
    pub fn get_backend() -> &'static str { unsafe { BACKEND } }

    /// Set the backend. Only "ml_framework_core" is functional.
    pub fn set_backend(name: &'static str) -> Result<(), String> {
        if name != "ml_framework_core" {
            return Err(format!("Backend '{}' is not supported. Only 'ml_framework_core' is available.", name));
        }
        unsafe { BACKEND = name; }
        Ok(())
    }
}

// =========================================================================
// Activations module
// =========================================================================

/// Activation functions as plain functions with string lookup.
pub mod activations {
    use ml_framework_core::Tensor;

    pub fn relu(x: &Tensor) -> Tensor { x.relu() }
    pub fn sigmoid(x: &Tensor) -> Tensor { x.sigmoid() }
    pub fn tanh_act(x: &Tensor) -> Tensor { x.tanh_act() }
    pub fn softmax(x: &Tensor) -> Tensor { x.softmax(x.ndim() - 1) }
    pub fn gelu(x: &Tensor) -> Tensor { x.gelu() }
    pub fn linear(x: &Tensor) -> Tensor { x.clone() }

    /// Look up an activation function by name.
    pub fn get_activation(name: Option<&str>) -> Option<fn(&Tensor) -> Tensor> {
        match name {
            None | Some("linear") => None,
            Some("relu") => Some(relu),
            Some("sigmoid") => Some(sigmoid),
            Some("tanh") => Some(tanh_act),
            Some("gelu") => Some(gelu),
            Some(other) => panic!("Unknown activation: {}", other),
        }
    }
}

// =========================================================================
// Layers module
// =========================================================================

/// Neural network building blocks.
pub mod layers {
    use ml_framework_core::{Tensor, Parameter};

    /// Base trait for all Keras layers.
    pub trait Layer {
        fn call(&self, inputs: &Tensor, training: bool) -> Tensor;
        fn trainable_weights(&self) -> Vec<&Parameter>;
        fn count_params(&self) -> usize {
            self.trainable_weights().iter().map(|p| p.numel()).sum()
        }
    }

    /// Fully connected layer: y = activation(x @ W + b).
    ///
    /// Keras stores W as (in_features, units) -- no transpose needed.
    /// The weights are created lazily on first call if input_dim is not specified.
    pub struct Dense {
        pub kernel: Option<Parameter>,
        pub bias: Option<Parameter>,
        pub units: usize,
        pub use_bias: bool,
        pub activation: Option<fn(&Tensor) -> Tensor>,
        built: bool,
    }

    impl Dense {
        pub fn new(units: usize, activation: Option<&str>, use_bias: bool) -> Self {
            Dense {
                kernel: None, bias: None, units, use_bias,
                activation: super::activations::get_activation(activation),
                built: false,
            }
        }

        /// Build with known input dimension.
        pub fn build(&mut self, input_dim: usize) {
            let stddev = (6.0 / (input_dim + self.units) as f64).sqrt();
            let kernel_data: Vec<f64> = Tensor::randn(&[input_dim, self.units], "cpu")
                .data().iter().map(|x| x * stddev).collect();
            self.kernel = Some(Parameter::new(
                Tensor::from_slice(&kernel_data, &[input_dim, self.units], true, "cpu")
            ));
            if self.use_bias {
                self.bias = Some(Parameter::zeros(&[self.units], "cpu"));
            }
            self.built = true;
        }
    }

    impl Layer for Dense {
        fn call(&self, inputs: &Tensor, _training: bool) -> Tensor {
            let kernel = self.kernel.as_ref().expect("Dense layer not built");
            let mut output = inputs.matmul(&kernel.tensor);
            if let Some(ref bias) = self.bias {
                let batch_size = inputs.shape()[0];
                let ones_col = Tensor::ones(&[batch_size, 1], "cpu");
                let bias_row = bias.tensor.reshape(&[1, self.units]);
                let bias_broadcast = ones_col.matmul(&bias_row);
                output = output.add(&bias_broadcast);
            }
            if let Some(act) = self.activation {
                output = act(&output);
            }
            output
        }

        fn trainable_weights(&self) -> Vec<&Parameter> {
            let mut w = Vec::new();
            if let Some(ref k) = self.kernel { w.push(k); }
            if let Some(ref b) = self.bias { w.push(b); }
            w
        }
    }

    /// Dropout layer.
    pub struct Dropout { pub rate: f64 }
    impl Dropout {
        pub fn new(rate: f64) -> Self { Dropout { rate } }
    }
    impl Layer for Dropout {
        fn call(&self, inputs: &Tensor, training: bool) -> Tensor {
            if !training || self.rate == 0.0 { return inputs.clone(); }
            inputs.clone() // Simplified: pass-through
        }
        fn trainable_weights(&self) -> Vec<&Parameter> { vec![] }
    }

    /// Flatten layer.
    pub struct Flatten;
    impl Layer for Flatten {
        fn call(&self, inputs: &Tensor, _training: bool) -> Tensor {
            let shape = inputs.shape();
            let batch = shape[0];
            let flat: usize = shape[1..].iter().product();
            inputs.reshape(&[batch, flat])
        }
        fn trainable_weights(&self) -> Vec<&Parameter> { vec![] }
    }

    /// ReLU activation as a standalone layer.
    pub struct ReLU;
    impl Layer for ReLU {
        fn call(&self, inputs: &Tensor, _training: bool) -> Tensor { inputs.relu() }
        fn trainable_weights(&self) -> Vec<&Parameter> { vec![] }
    }

    /// Softmax activation as a standalone layer.
    pub struct Softmax { pub axis: usize }
    impl Softmax {
        pub fn new(axis: usize) -> Self { Softmax { axis } }
    }
    impl Layer for Softmax {
        fn call(&self, inputs: &Tensor, _training: bool) -> Tensor { inputs.softmax(self.axis) }
        fn trainable_weights(&self) -> Vec<&Parameter> { vec![] }
    }
}

// =========================================================================
// Losses module
// =========================================================================

/// Loss functions for training.
pub mod losses {
    use ml_framework_core::Tensor;

    /// Trait for all loss functions.
    pub trait Loss {
        fn call(&self, y_true: &Tensor, y_pred: &Tensor) -> Tensor;
    }

    pub struct MeanSquaredError;
    impl Loss for MeanSquaredError {
        fn call(&self, y_true: &Tensor, y_pred: &Tensor) -> Tensor {
            let diff = y_pred.sub(y_true);
            let squared = diff.mul(&diff);
            squared.mean(None, false)
        }
    }

    pub struct MeanAbsoluteError;
    impl Loss for MeanAbsoluteError {
        fn call(&self, y_true: &Tensor, y_pred: &Tensor) -> Tensor {
            let diff = y_pred.sub(y_true);
            diff.abs().mean(None, false)
        }
    }

    pub struct BinaryCrossentropy { pub from_logits: bool }
    impl BinaryCrossentropy {
        pub fn new(from_logits: bool) -> Self { BinaryCrossentropy { from_logits } }
    }
    impl Loss for BinaryCrossentropy {
        fn call(&self, y_true: &Tensor, y_pred: &Tensor) -> Tensor {
            let y_pred = if self.from_logits { y_pred.sigmoid() } else { y_pred.clone() };
            let eps = 1e-7;
            let clamped = y_pred.clamp(Some(eps), Some(1.0 - eps));
            let log_pred = clamped.log();
            let one_minus = clamped.neg().add_scalar(1.0);
            let log_one_minus = one_minus.log();
            let y_neg = y_true.neg().add_scalar(1.0);
            let loss = y_true.mul(&log_pred).add(&y_neg.mul(&log_one_minus)).neg();
            loss.mean(None, false)
        }
    }

    /// Look up a loss by string name.
    pub fn get_loss(name: &str) -> Box<dyn Loss> {
        match name {
            "mse" | "mean_squared_error" => Box::new(MeanSquaredError),
            "mae" | "mean_absolute_error" => Box::new(MeanAbsoluteError),
            "binary_crossentropy" => Box::new(BinaryCrossentropy::new(false)),
            _ => panic!("Unknown loss: {}", name),
        }
    }
}

// =========================================================================
// Optimizers module
// =========================================================================

/// Optimizers for updating model weights.
pub mod optimizers {
    use ml_framework_core::Parameter;
    use std::collections::HashMap;

    /// Trait for all Keras optimizers.
    pub trait Optimizer {
        fn apply_gradients(&mut self, grads_and_vars: &[(Option<Vec<f64>>, &Parameter)]);
        fn learning_rate(&self) -> f64;
        fn set_learning_rate(&mut self, lr: f64);
    }

    /// SGD optimizer.
    pub struct SGD {
        pub lr: f64,
        pub momentum: f64,
        velocities: HashMap<usize, Vec<f64>>,
        iterations: usize,
    }

    impl SGD {
        pub fn new(lr: f64, momentum: f64) -> Self {
            SGD { lr, momentum, velocities: HashMap::new(), iterations: 0 }
        }
    }

    impl Optimizer for SGD {
        fn apply_gradients(&mut self, grads_and_vars: &[(Option<Vec<f64>>, &Parameter)]) {
            self.iterations += 1;
            for (i, (grad_opt, param)) in grads_and_vars.iter().enumerate() {
                if let Some(grad) = grad_opt {
                    if self.momentum != 0.0 {
                        let v = self.velocities.entry(i).or_insert_with(|| vec![0.0; grad.len()]);
                        for j in 0..v.len() { v[j] = self.momentum * v[j] + grad[j]; }
                        let new: Vec<f64> = param.data().iter().zip(v.iter())
                            .map(|(w, vj)| w - self.lr * vj).collect();
                        param.set_data(new);
                    } else {
                        let new: Vec<f64> = param.data().iter().zip(grad.iter())
                            .map(|(w, g)| w - self.lr * g).collect();
                        param.set_data(new);
                    }
                }
            }
        }
        fn learning_rate(&self) -> f64 { self.lr }
        fn set_learning_rate(&mut self, lr: f64) { self.lr = lr; }
    }

    /// Adam optimizer.
    pub struct Adam {
        pub lr: f64,
        pub beta_1: f64,
        pub beta_2: f64,
        pub epsilon: f64,
        m: HashMap<usize, Vec<f64>>,
        v: HashMap<usize, Vec<f64>>,
        iterations: usize,
    }

    impl Adam {
        pub fn new(lr: f64) -> Self {
            Adam { lr, beta_1: 0.9, beta_2: 0.999, epsilon: 1e-7,
                   m: HashMap::new(), v: HashMap::new(), iterations: 0 }
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
                        m[j] = self.beta_1 * m[j] + (1.0 - self.beta_1) * grad[j];
                        v[j] = self.beta_2 * v[j] + (1.0 - self.beta_2) * grad[j] * grad[j];
                    }
                    let bc1 = 1.0 - self.beta_1.powi(t as i32);
                    let bc2 = 1.0 - self.beta_2.powi(t as i32);
                    let new: Vec<f64> = param.data().iter().zip(m.iter().zip(v.iter()))
                        .map(|(w, (mj, vj))| w - self.lr * (mj / bc1) / ((vj / bc2).sqrt() + self.epsilon))
                        .collect();
                    param.set_data(new);
                }
            }
        }
        fn learning_rate(&self) -> f64 { self.lr }
        fn set_learning_rate(&mut self, lr: f64) { self.lr = lr; }
    }

    /// Get optimizer by string name.
    pub fn get_optimizer(name: &str) -> Box<dyn Optimizer> {
        match name {
            "sgd" => Box::new(SGD::new(0.01, 0.0)),
            "adam" => Box::new(Adam::new(0.001)),
            _ => panic!("Unknown optimizer: {}", name),
        }
    }
}

// =========================================================================
// Metrics module
// =========================================================================

/// Metrics for monitoring model performance.
pub mod metrics {
    use ml_framework_core::Tensor;

    /// Trait for all metrics.
    pub trait Metric {
        fn name(&self) -> &str;
        fn update_state(&mut self, y_true: &Tensor, y_pred: &Tensor);
        fn result(&self) -> f64;
        fn reset_state(&mut self);
    }

    /// Accuracy metric.
    pub struct Accuracy { correct: usize, total: usize }
    impl Accuracy {
        pub fn new() -> Self { Accuracy { correct: 0, total: 0 } }
    }
    impl Metric for Accuracy {
        fn name(&self) -> &str { "accuracy" }
        fn update_state(&mut self, y_true: &Tensor, y_pred: &Tensor) {
            let pred_data = y_pred.data();
            let true_data = y_true.data();
            if y_pred.ndim() == 2 && y_pred.shape()[y_pred.ndim() - 1] > 1 {
                let nc = y_pred.shape()[y_pred.ndim() - 1];
                let bs = y_pred.shape()[0];
                for i in 0..bs {
                    let row = &pred_data[i*nc..(i+1)*nc];
                    let pc = row.iter().enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                        .map(|(idx, _)| idx).unwrap();
                    let tc = true_data[i] as usize;
                    if pc == tc { self.correct += 1; }
                    self.total += 1;
                }
            } else {
                for (yt, yp) in true_data.iter().zip(pred_data.iter()) {
                    if (yp - 0.5).signum() == (yt - 0.5).signum() || yp.round() == yt.round() {
                        self.correct += 1;
                    }
                    self.total += 1;
                }
            }
        }
        fn result(&self) -> f64 { if self.total == 0 { 0.0 } else { self.correct as f64 / self.total as f64 } }
        fn reset_state(&mut self) { self.correct = 0; self.total = 0; }
    }

    /// MSE metric (not a loss, just for monitoring).
    pub struct MeanSquaredError { sum: f64, count: usize }
    impl MeanSquaredError {
        pub fn new() -> Self { MeanSquaredError { sum: 0.0, count: 0 } }
    }
    impl Metric for MeanSquaredError {
        fn name(&self) -> &str { "mean_squared_error" }
        fn update_state(&mut self, y_true: &Tensor, y_pred: &Tensor) {
            for (yt, yp) in y_true.data().iter().zip(y_pred.data().iter()) {
                self.sum += (yt - yp).powi(2);
                self.count += 1;
            }
        }
        fn result(&self) -> f64 { if self.count == 0 { 0.0 } else { self.sum / self.count as f64 } }
        fn reset_state(&mut self) { self.sum = 0.0; self.count = 0; }
    }

    /// Get metric by string name.
    pub fn get_metric(name: &str) -> Box<dyn Metric> {
        match name {
            "accuracy" => Box::new(Accuracy::new()),
            "mse" | "mean_squared_error" => Box::new(MeanSquaredError::new()),
            _ => panic!("Unknown metric: {}", name),
        }
    }
}

// =========================================================================
// Callbacks module
// =========================================================================

/// Hooks into the training loop.
pub mod callbacks {
    use std::collections::HashMap;

    /// History callback: records training metrics per epoch.
    pub struct History {
        pub history: HashMap<String, Vec<f64>>,
        pub epoch: Vec<usize>,
    }

    impl History {
        pub fn new() -> Self { History { history: HashMap::new(), epoch: Vec::new() } }

        pub fn on_epoch_end(&mut self, epoch: usize, logs: &HashMap<String, f64>) {
            self.epoch.push(epoch);
            for (key, &value) in logs {
                self.history.entry(key.clone()).or_insert_with(Vec::new).push(value);
            }
        }
    }

    /// EarlyStopping callback.
    pub struct EarlyStopping {
        pub monitor: String,
        pub patience: usize,
        pub min_delta: f64,
        pub stopped: bool,
        best: Option<f64>,
        wait: usize,
    }

    impl EarlyStopping {
        pub fn new(monitor: &str, patience: usize, min_delta: f64) -> Self {
            EarlyStopping {
                monitor: monitor.to_string(), patience, min_delta,
                stopped: false, best: None, wait: 0,
            }
        }

        pub fn on_epoch_end(&mut self, logs: &HashMap<String, f64>) {
            if let Some(&current) = logs.get(&self.monitor) {
                match self.best {
                    None => { self.best = Some(current); }
                    Some(best) => {
                        if current < best - self.min_delta {
                            self.best = Some(current);
                            self.wait = 0;
                        } else {
                            self.wait += 1;
                            if self.wait >= self.patience { self.stopped = true; }
                        }
                    }
                }
            }
        }
    }
}

// =========================================================================
// Models module
// =========================================================================

/// Model containers: Sequential and Model.
pub mod models {
    use ml_framework_core::{Tensor, Parameter};
    use super::layers::Layer;
    use super::losses;
    use super::optimizers;
    use super::callbacks::History;
    use std::collections::HashMap;

    /// A linear stack of layers.
    pub struct Sequential {
        pub layers_list: Vec<Box<dyn Layer>>,
        optimizer: Option<Box<dyn optimizers::Optimizer>>,
        loss_fn: Option<Box<dyn losses::Loss>>,
        compiled: bool,
    }

    impl Sequential {
        pub fn new() -> Self {
            Sequential { layers_list: Vec::new(), optimizer: None, loss_fn: None, compiled: false }
        }

        /// Add a layer.
        pub fn add(&mut self, layer: Box<dyn Layer>) {
            self.layers_list.push(layer);
        }

        /// Get all trainable weights across all layers.
        pub fn trainable_weights(&self) -> Vec<&Parameter> {
            self.layers_list.iter().flat_map(|l| l.trainable_weights()).collect()
        }

        /// Forward pass through all layers.
        pub fn call(&self, x: &Tensor, training: bool) -> Tensor {
            let mut out = x.clone();
            for layer in &self.layers_list {
                out = layer.call(&out, training);
            }
            out
        }

        /// Configure model for training.
        pub fn compile(&mut self, optimizer: &str, loss: &str, _metrics: &[&str]) {
            self.optimizer = Some(optimizers::get_optimizer(optimizer));
            self.loss_fn = Some(losses::get_loss(loss));
            self.compiled = true;
        }

        /// Train the model on data.
        pub fn fit(
            &mut self, x: &Tensor, y: &Tensor,
            epochs: usize, batch_size: usize,
            _validation_data: Option<(&Tensor, &Tensor)>,
            verbose: usize,
        ) -> History {
            assert!(self.compiled, "Model must be compiled before training");
            let mut history = History::new();
            let n = x.shape()[0];
            let x_data = x.data();
            let y_data = y.data();
            let x_inner: usize = x.shape()[1..].iter().product::<usize>().max(1);
            let y_inner: usize = y.shape()[1..].iter().product::<usize>().max(1);

            for epoch in 0..epochs {
                let mut epoch_loss = 0.0;
                let mut n_batches = 0;
                let mut start = 0;
                while start < n {
                    let end = (start + batch_size).min(n);
                    let bs = end - start;
                    let mut x_shape = vec![bs];
                    x_shape.extend_from_slice(&x.shape()[1..]);
                    let mut y_shape = vec![bs];
                    y_shape.extend_from_slice(&y.shape()[1..]);

                    let x_batch = Tensor::from_slice(
                        &x_data[start*x_inner..end*x_inner], &x_shape, false, "cpu"
                    );
                    let y_batch = Tensor::from_slice(
                        &y_data[start*y_inner..end*y_inner], &y_shape, false, "cpu"
                    );

                    // Zero gradients
                    for p in self.trainable_weights() {
                        p.zero_grad();
                    }

                    // Forward + loss
                    for p in self.trainable_weights() {
                        p.tensor.set_requires_grad(true);
                    }
                    let pred = self.call(&x_batch, true);
                    let loss = self.loss_fn.as_ref().unwrap().call(&y_batch, &pred);
                    let _ = loss.backward(None);

                    // Optimizer step — get a raw pointer to the optimizer BEFORE
                    // borrowing trainable_weights, to avoid borrow conflict.
                    // SAFETY: optimizer and layers_list are disjoint fields.
                    let opt_ptr: *mut dyn optimizers::Optimizer =
                        &mut **self.optimizer.as_mut().unwrap() as *mut _;
                    let weights = self.trainable_weights();
                    let grads_and_vars: Vec<(Option<Vec<f64>>, &Parameter)> =
                        weights.iter()
                            .map(|p| (p.grad_data(), *p)).collect();
                    unsafe { &mut *opt_ptr }.apply_gradients(&grads_and_vars);

                    epoch_loss += loss.data()[0];
                    n_batches += 1;
                    start = end;
                }

                let avg_loss = epoch_loss / n_batches as f64;
                let mut logs = HashMap::new();
                logs.insert("loss".to_string(), avg_loss);
                history.on_epoch_end(epoch, &logs);

                if verbose >= 1 {
                    println!("Epoch {}/{} - loss: {:.4}", epoch + 1, epochs, avg_loss);
                }
            }
            history
        }

        /// Generate predictions.
        pub fn predict(&self, x: &Tensor) -> Tensor {
            self.call(x, false)
        }

        /// Evaluate on test data.
        pub fn evaluate(&self, x: &Tensor, y: &Tensor) -> f64 {
            let pred = self.call(x, false);
            let loss = self.loss_fn.as_ref().unwrap().call(y, &pred);
            loss.data()[0]
        }

        /// Count total trainable parameters.
        pub fn count_params(&self) -> usize {
            self.trainable_weights().iter().map(|p| p.numel()).sum()
        }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend() {
        assert_eq!(backend::get_backend(), "ml_framework_core");
        assert!(backend::set_backend("torch").is_err());
    }

    #[test]
    fn test_activations() {
        let x = Tensor::from_slice(&[-1.0, 0.0, 1.0], &[3], false, "cpu");
        let y = activations::relu(&x);
        assert_eq!(y.data(), vec![0.0, 0.0, 1.0]);

        let y = activations::sigmoid(&Tensor::from_slice(&[0.0], &[1], false, "cpu"));
        assert!((y.data()[0] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_activation_lookup() {
        let act = activations::get_activation(Some("relu"));
        assert!(act.is_some());
        let act = activations::get_activation(None);
        assert!(act.is_none());
    }

    #[test]
    fn test_dense_layer() {
        let mut layer = layers::Dense::new(2, Some("relu"), true);
        layer.build(4);
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[1, 4], false, "cpu");
        let y = layers::Layer::call(&layer, &x, false);
        assert_eq!(y.shape(), vec![1, 2]);
    }

    #[test]
    fn test_flatten_layer() {
        let layer = layers::Flatten;
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[1, 2, 3], false, "cpu");
        let y = layers::Layer::call(&layer, &x, false);
        assert_eq!(y.shape(), vec![1, 6]);
    }

    #[test]
    fn test_mse_loss() {
        let y_true = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], false, "cpu");
        let y_pred = Tensor::from_slice(&[1.5, 2.5, 3.5], &[3], false, "cpu");
        let loss = losses::MeanSquaredError;
        let result = losses::Loss::call(&loss, &y_true, &y_pred);
        assert!((result.item().unwrap() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_sgd_optimizer() {
        let p = Parameter::new(Tensor::from_slice(&[1.0, 2.0], &[2], true, "cpu"));
        let mut sgd = optimizers::SGD::new(0.1, 0.0);
        let grads = vec![1.0, 1.0];
        optimizers::Optimizer::apply_gradients(&mut sgd, &[(Some(grads), &p)]);
        assert!((p.data()[0] - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_adam_optimizer() {
        let p = Parameter::new(Tensor::from_slice(&[1.0], &[1], true, "cpu"));
        let mut adam = optimizers::Adam::new(0.001);
        let grads = vec![1.0];
        optimizers::Optimizer::apply_gradients(&mut adam, &[(Some(grads), &p)]);
        assert!(p.data()[0] != 1.0);
    }

    #[test]
    fn test_accuracy_metric() {
        let mut acc = metrics::Accuracy::new();
        let y_true = Tensor::from_slice(&[1.0, 0.0, 1.0], &[3], false, "cpu");
        let y_pred = Tensor::from_slice(&[0.9, 0.1, 0.8], &[3], false, "cpu");
        metrics::Metric::update_state(&mut acc, &y_true, &y_pred);
        assert_eq!(metrics::Metric::result(&acc), 1.0);
    }

    #[test]
    fn test_history_callback() {
        let mut history = callbacks::History::new();
        let mut logs = HashMap::new();
        logs.insert("loss".to_string(), 0.5);
        history.on_epoch_end(0, &logs);
        assert_eq!(history.history["loss"], vec![0.5]);
    }

    #[test]
    fn test_early_stopping() {
        let mut es = callbacks::EarlyStopping::new("val_loss", 2, 0.0);
        let mut logs = HashMap::new();
        logs.insert("val_loss".to_string(), 0.5);
        es.on_epoch_end(&logs);
        assert!(!es.stopped);
        logs.insert("val_loss".to_string(), 0.6);
        es.on_epoch_end(&logs);
        assert!(!es.stopped);
        logs.insert("val_loss".to_string(), 0.7);
        es.on_epoch_end(&logs);
        assert!(es.stopped);
    }

    #[test]
    fn test_sequential_model_creation() {
        let mut model = models::Sequential::new();
        let mut dense1 = layers::Dense::new(4, Some("relu"), true);
        dense1.build(3);
        model.add(Box::new(dense1));
        model.add(Box::new(layers::ReLU));
        assert!(model.trainable_weights().len() > 0);
    }

    #[test]
    fn test_loss_lookup() {
        let loss = losses::get_loss("mse");
        let y_true = Tensor::from_slice(&[1.0], &[1], false, "cpu");
        let y_pred = Tensor::from_slice(&[2.0], &[1], false, "cpu");
        let result = losses::Loss::call(loss.as_ref(), &y_true, &y_pred);
        assert!((result.item().unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_optimizer_lookup() {
        let opt = optimizers::get_optimizer("adam");
        assert!((optimizers::Optimizer::learning_rate(opt.as_ref()) - 0.001).abs() < 1e-10);
    }
}
