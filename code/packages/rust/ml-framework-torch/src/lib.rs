//! # ML Framework Torch -- PyTorch-Compatible API Layer
//!
//! This crate provides a PyTorch-compatible API built on top of
//! `ml-framework-core`. It implements the familiar PyTorch interfaces:
//!
//! - `nn::Module` -- base trait for layers with parameter registration
//! - `nn::Linear`, `nn::ReLU`, etc. -- standard neural network layers
//! - `nn::MSELoss`, `nn::CrossEntropyLoss` -- loss functions
//! - `optim::SGD`, `optim::Adam` -- parameter update algorithms
//! - `data::TensorDataset`, `data::DataLoader` -- batch training
//!
//! # Architecture
//!
//! This package is a THIN WRAPPER. Almost all computation is done by
//! ml-framework-core. This layer adds the PyTorch API conventions:
//! ```text
//! ml-framework-core  = the engine (tensors, autograd, math)
//! ml-framework-torch = the car (layers, optimizers, training loop)
//! ```

use ml_framework_core::{Tensor, Parameter};

// =========================================================================
// Top-level tensor creation functions
// =========================================================================

/// Create a tensor from a slice (like torch.tensor()).
pub fn tensor(data: &[f64], shape: &[usize], requires_grad: bool) -> Tensor {
    Tensor::from_slice(data, shape, requires_grad, "cpu")
}

/// Create a tensor filled with zeros.
pub fn zeros(shape: &[usize]) -> Tensor { Tensor::zeros(shape, "cpu") }

/// Create a tensor filled with ones.
pub fn ones(shape: &[usize]) -> Tensor { Tensor::ones(shape, "cpu") }

/// Create a tensor with random normal values.
pub fn randn(shape: &[usize], requires_grad: bool) -> Tensor {
    let t = Tensor::randn(shape, "cpu");
    if requires_grad { t.set_requires_grad(true); }
    t
}

/// Create an n x n identity matrix.
pub fn eye(n: usize) -> Tensor { Tensor::eye(n, "cpu") }

/// Create a 1-D tensor with values from start to end.
pub fn arange(start: f64, end: f64, step: f64) -> Tensor { Tensor::arange(start, end, step, "cpu") }

/// Create a tensor filled with a constant value.
pub fn full(shape: &[usize], fill_value: f64) -> Tensor { Tensor::full(shape, fill_value, "cpu") }

// =========================================================================
// nn module -- Neural network layers
// =========================================================================

/// Neural network modules, layers, activations, and loss functions.
pub mod nn {
    use ml_framework_core::{Tensor, Parameter};

    /// Base trait for all neural network layers.
    ///
    /// Every layer must implement `forward()` for the computation and
    /// `parameters()` to return all learnable parameters.
    pub trait Module {
        /// Compute the layer's output.
        fn forward(&self, x: &Tensor) -> Tensor;
        /// Return all learnable parameters.
        fn parameters(&self) -> Vec<&Parameter>;
        /// Set training/eval mode.
        fn train(&mut self, _mode: bool) {}
    }

    // ----- Linear layer -----

    /// Fully connected layer: y = x @ W^T + b.
    ///
    /// We store W as (out_features, in_features) because each ROW of W
    /// corresponds to one output neuron's weights (matching PyTorch convention).
    pub struct Linear {
        pub weight: Parameter,
        pub bias: Option<Parameter>,
        pub in_features: usize,
        pub out_features: usize,
    }

    impl Linear {
        /// Create a new Linear layer with Xavier initialization.
        pub fn new(in_features: usize, out_features: usize, bias: bool) -> Self {
            let stddev = 1.0 / (in_features as f64).sqrt();
            let weight_data: Vec<f64> = Tensor::randn(&[out_features, in_features], "cpu")
                .data().iter().map(|x| x * stddev).collect();
            let weight = Parameter::new(
                Tensor::from_slice(&weight_data, &[out_features, in_features], true, "cpu")
            );
            let bias_param = if bias {
                Some(Parameter::zeros(&[out_features], "cpu"))
            } else {
                None
            };
            Linear { weight, bias: bias_param, in_features, out_features }
        }
    }

    impl Module for Linear {
        fn forward(&self, x: &Tensor) -> Tensor {
            // x @ W^T
            let wt = self.weight.tensor.t();
            let mut output = x.matmul(&wt);
            // Add bias with broadcasting: ones(batch, 1) @ bias.reshape(1, out) -> (batch, out)
            if let Some(ref bias) = self.bias {
                let batch_size = x.shape()[0];
                let ones_col = Tensor::ones(&[batch_size, 1], "cpu");
                let bias_row = bias.tensor.reshape(&[1, self.out_features]);
                let bias_broadcast = ones_col.matmul(&bias_row);
                output = output.add(&bias_broadcast);
            }
            output
        }

        fn parameters(&self) -> Vec<&Parameter> {
            let mut params = vec![&self.weight];
            if let Some(ref bias) = self.bias { params.push(bias); }
            params
        }
    }

    // ----- Activation layers -----

    /// ReLU activation layer: y = max(0, x).
    pub struct ReLU;
    impl Module for ReLU {
        fn forward(&self, x: &Tensor) -> Tensor { x.relu() }
        fn parameters(&self) -> Vec<&Parameter> { vec![] }
    }

    /// GELU activation layer (used in transformers).
    pub struct GELU;
    impl Module for GELU {
        fn forward(&self, x: &Tensor) -> Tensor { x.gelu() }
        fn parameters(&self) -> Vec<&Parameter> { vec![] }
    }

    /// Sigmoid activation layer.
    pub struct Sigmoid;
    impl Module for Sigmoid {
        fn forward(&self, x: &Tensor) -> Tensor { x.sigmoid() }
        fn parameters(&self) -> Vec<&Parameter> { vec![] }
    }

    /// Tanh activation layer.
    pub struct Tanh;
    impl Module for Tanh {
        fn forward(&self, x: &Tensor) -> Tensor { x.tanh_act() }
        fn parameters(&self) -> Vec<&Parameter> { vec![] }
    }

    /// Softmax activation layer.
    pub struct Softmax { pub dim: usize }
    impl Module for Softmax {
        fn forward(&self, x: &Tensor) -> Tensor { x.softmax(self.dim) }
        fn parameters(&self) -> Vec<&Parameter> { vec![] }
    }

    /// Flatten layer: collapses all dims except batch into one.
    pub struct Flatten;
    impl Module for Flatten {
        fn forward(&self, x: &Tensor) -> Tensor {
            let shape = x.shape();
            if shape.len() <= 2 { return x.clone(); }
            let batch = shape[0];
            let flat: usize = shape[1..].iter().product();
            x.reshape(&[batch, flat])
        }
        fn parameters(&self) -> Vec<&Parameter> { vec![] }
    }

    /// Dropout layer (pass-through in this simulation).
    pub struct Dropout { pub p: f64, pub training: bool }
    impl Dropout {
        pub fn new(p: f64) -> Self { Dropout { p, training: true } }
    }
    impl Module for Dropout {
        fn forward(&self, x: &Tensor) -> Tensor { x.clone() }
        fn parameters(&self) -> Vec<&Parameter> { vec![] }
        fn train(&mut self, mode: bool) { self.training = mode; }
    }

    // ----- Sequential container -----

    /// A sequential container that applies layers in order.
    ///
    /// Layers are applied left-to-right: x -> layer0 -> layer1 -> ... -> output.
    pub struct Sequential {
        pub layers: Vec<Box<dyn Module>>,
    }

    impl Sequential {
        pub fn new(layers: Vec<Box<dyn Module>>) -> Self {
            Sequential { layers }
        }
    }

    impl Module for Sequential {
        fn forward(&self, x: &Tensor) -> Tensor {
            let mut out = x.clone();
            for layer in &self.layers {
                out = layer.forward(&out);
            }
            out
        }

        fn parameters(&self) -> Vec<&Parameter> {
            self.layers.iter().flat_map(|l| l.parameters()).collect()
        }
    }

    // ----- Loss functions -----

    /// MSE Loss: mean((pred - target)^2).
    pub struct MSELoss { pub reduction: String }
    impl MSELoss {
        pub fn new(reduction: &str) -> Self { MSELoss { reduction: reduction.to_string() } }
        pub fn forward(&self, pred: &Tensor, target: &Tensor) -> Tensor {
            let diff = pred.sub(target);
            let squared = diff.pow(2.0);
            match self.reduction.as_str() {
                "mean" => squared.mean(None, false),
                "sum" => squared.sum(None, false),
                _ => squared,
            }
        }
    }

    /// L1 Loss: mean(|pred - target|).
    pub struct L1Loss { pub reduction: String }
    impl L1Loss {
        pub fn new(reduction: &str) -> Self { L1Loss { reduction: reduction.to_string() } }
        pub fn forward(&self, pred: &Tensor, target: &Tensor) -> Tensor {
            let diff = pred.sub(target);
            let abs_diff = diff.abs();
            match self.reduction.as_str() {
                "mean" => abs_diff.mean(None, false),
                "sum" => abs_diff.sum(None, false),
                _ => abs_diff,
            }
        }
    }

    /// Cross-entropy loss (LogSoftmax + NLLLoss).
    pub struct CrossEntropyLoss { pub reduction: String }
    impl CrossEntropyLoss {
        pub fn new(reduction: &str) -> Self { CrossEntropyLoss { reduction: reduction.to_string() } }
        pub fn forward(&self, pred: &Tensor, target: &Tensor) -> Tensor {
            let shape = pred.shape();
            let batch_size = shape[0];
            let num_classes = shape[1];
            let pred_data = pred.data();
            let target_data = target.data();
            // Compute log-softmax then select correct class
            let log_softmax = pred.softmax(1).log();
            let log_data = log_softmax.data();
            // Build one-hot from integer labels
            let mut one_hot = vec![0.0; batch_size * num_classes];
            for i in 0..batch_size {
                let class_idx = target_data[i] as usize;
                one_hot[i * num_classes + class_idx] = 1.0;
            }
            let one_hot_t = Tensor::from_slice(&one_hot, &[batch_size, num_classes], false, "cpu");
            let elementwise = log_softmax.mul(&one_hot_t).neg();
            match self.reduction.as_str() {
                "mean" => {
                    let total = elementwise.sum(None, false);
                    let bs = batch_size as f64;
                    Tensor::from_slice(&[total.data()[0] / bs], &[1], total.requires_grad(), "cpu")
                }
                "sum" => elementwise.sum(None, false),
                _ => elementwise.sum(Some(1), false),
            }
        }
    }

    /// BCE Loss: binary cross-entropy.
    pub struct BCELoss { pub reduction: String }
    impl BCELoss {
        pub fn new(reduction: &str) -> Self { BCELoss { reduction: reduction.to_string() } }
        pub fn forward(&self, pred: &Tensor, target: &Tensor) -> Tensor {
            let eps = 1e-7;
            let pred_clamped = pred.clamp(Some(eps), Some(1.0 - eps));
            let log_pred = pred_clamped.log();
            let one_minus = pred_clamped.neg().add_scalar(1.0);
            let log_one_minus = one_minus.log();
            let target_neg = target.neg().add_scalar(1.0);
            let loss = target.mul(&log_pred).add(&target_neg.mul(&log_one_minus)).neg();
            match self.reduction.as_str() {
                "mean" => loss.mean(None, false),
                "sum" => loss.sum(None, false),
                _ => loss,
            }
        }
    }

    /// Functional API: stateless operations.
    pub mod functional {
        use ml_framework_core::Tensor;

        pub fn relu(x: &Tensor) -> Tensor { x.relu() }
        pub fn gelu(x: &Tensor) -> Tensor { x.gelu() }
        pub fn sigmoid(x: &Tensor) -> Tensor { x.sigmoid() }
        pub fn tanh(x: &Tensor) -> Tensor { x.tanh_act() }
        pub fn softmax(x: &Tensor, dim: usize) -> Tensor { x.softmax(dim) }

        pub fn mse_loss(pred: &Tensor, target: &Tensor, reduction: &str) -> Tensor {
            let diff = pred.sub(target);
            let squared = diff.pow(2.0);
            match reduction {
                "mean" => squared.mean(None, false),
                "sum" => squared.sum(None, false),
                _ => squared,
            }
        }

        pub fn l1_loss(pred: &Tensor, target: &Tensor, reduction: &str) -> Tensor {
            let diff = pred.sub(target);
            let abs_diff = diff.abs();
            match reduction {
                "mean" => abs_diff.mean(None, false),
                "sum" => abs_diff.sum(None, false),
                _ => abs_diff,
            }
        }
    }
}

// =========================================================================
// optim module -- Parameter update algorithms
// =========================================================================

/// Optimizers that update model parameters based on gradients.
pub mod optim {
    use ml_framework_core::Parameter;

    /// Base trait for all optimizers.
    pub trait Optimizer {
        /// Reset all parameter gradients.
        fn zero_grad(&self);
        /// Update parameters using their gradients.
        fn step(&mut self);
    }

    /// SGD optimizer with momentum and weight decay.
    pub struct SGD {
        pub params: Vec<*const Parameter>, // Raw pointers for interior mutability
        pub lr: f64,
        pub momentum: f64,
        pub weight_decay: f64,
        velocity: Vec<Vec<f64>>,
    }

    // Safety: Parameters are only accessed from a single thread
    unsafe impl Send for SGD {}

    impl SGD {
        pub fn new(params: Vec<&Parameter>, lr: f64, momentum: f64, weight_decay: f64) -> Self {
            let velocity: Vec<Vec<f64>> = params.iter().map(|p| vec![0.0; p.numel()]).collect();
            let ptrs: Vec<*const Parameter> = params.iter().map(|p| *p as *const Parameter).collect();
            SGD { params: ptrs, lr, momentum, weight_decay, velocity }
        }
    }

    impl Optimizer for SGD {
        fn zero_grad(&self) {
            for &ptr in &self.params {
                unsafe { (*ptr).zero_grad(); }
            }
        }

        fn step(&mut self) {
            for (i, &ptr) in self.params.iter().enumerate() {
                let p = unsafe { &*ptr };
                if let Some(grad) = p.grad_data() {
                    let mut grad_data = grad;
                    if self.weight_decay != 0.0 {
                        let data = p.data();
                        grad_data = grad_data.iter().zip(data.iter())
                            .map(|(g, w)| g + self.weight_decay * w).collect();
                    }
                    if self.momentum != 0.0 {
                        let v = &mut self.velocity[i];
                        for j in 0..v.len() {
                            v[j] = self.momentum * v[j] + grad_data[j];
                        }
                        let new_data: Vec<f64> = p.data().iter().zip(v.iter())
                            .map(|(w, vj)| w - self.lr * vj).collect();
                        p.set_data(new_data);
                    } else {
                        let new_data: Vec<f64> = p.data().iter().zip(grad_data.iter())
                            .map(|(w, g)| w - self.lr * g).collect();
                        p.set_data(new_data);
                    }
                }
            }
        }
    }

    /// Adam optimizer.
    pub struct Adam {
        pub params: Vec<*const Parameter>,
        pub lr: f64,
        pub beta1: f64,
        pub beta2: f64,
        pub eps: f64,
        pub weight_decay: f64,
        m: Vec<Vec<f64>>,
        v: Vec<Vec<f64>>,
        t: usize,
    }

    unsafe impl Send for Adam {}

    impl Adam {
        pub fn new(params: Vec<&Parameter>, lr: f64, betas: (f64, f64), eps: f64, weight_decay: f64) -> Self {
            let m: Vec<Vec<f64>> = params.iter().map(|p| vec![0.0; p.numel()]).collect();
            let v: Vec<Vec<f64>> = params.iter().map(|p| vec![0.0; p.numel()]).collect();
            let ptrs: Vec<*const Parameter> = params.iter().map(|p| *p as *const Parameter).collect();
            Adam { params: ptrs, lr, beta1: betas.0, beta2: betas.1, eps, weight_decay, m, v, t: 0 }
        }

        pub fn default_with_params(params: Vec<&Parameter>) -> Self {
            Self::new(params, 0.001, (0.9, 0.999), 1e-8, 0.0)
        }
    }

    impl Optimizer for Adam {
        fn zero_grad(&self) {
            for &ptr in &self.params {
                unsafe { (*ptr).zero_grad(); }
            }
        }

        fn step(&mut self) {
            self.t += 1;
            for (i, &ptr) in self.params.iter().enumerate() {
                let p = unsafe { &*ptr };
                if let Some(grad) = p.grad_data() {
                    let mut grad_data = grad;
                    if self.weight_decay != 0.0 {
                        let data = p.data();
                        grad_data = grad_data.iter().zip(data.iter())
                            .map(|(g, w)| g + self.weight_decay * w).collect();
                    }
                    let m = &mut self.m[i];
                    let v = &mut self.v[i];
                    for j in 0..m.len() {
                        let g = grad_data[j];
                        m[j] = self.beta1 * m[j] + (1.0 - self.beta1) * g;
                        v[j] = self.beta2 * v[j] + (1.0 - self.beta2) * g * g;
                    }
                    let bc1 = 1.0 - self.beta1.powi(self.t as i32);
                    let bc2 = 1.0 - self.beta2.powi(self.t as i32);
                    let new_data: Vec<f64> = p.data().iter().zip(m.iter().zip(v.iter()))
                        .map(|(w, (mj, vj))| w - self.lr * (mj / bc1) / ((vj / bc2).sqrt() + self.eps))
                        .collect();
                    p.set_data(new_data);
                }
            }
        }
    }

    /// RMSprop optimizer.
    pub struct RMSprop {
        pub params: Vec<*const Parameter>,
        pub lr: f64,
        pub alpha: f64,
        pub eps: f64,
        v: Vec<Vec<f64>>,
    }

    unsafe impl Send for RMSprop {}

    impl RMSprop {
        pub fn new(params: Vec<&Parameter>, lr: f64, alpha: f64, eps: f64) -> Self {
            let v: Vec<Vec<f64>> = params.iter().map(|p| vec![0.0; p.numel()]).collect();
            let ptrs: Vec<*const Parameter> = params.iter().map(|p| *p as *const Parameter).collect();
            RMSprop { params: ptrs, lr, alpha, eps, v }
        }
    }

    impl Optimizer for RMSprop {
        fn zero_grad(&self) {
            for &ptr in &self.params {
                unsafe { (*ptr).zero_grad(); }
            }
        }

        fn step(&mut self) {
            for (i, &ptr) in self.params.iter().enumerate() {
                let p = unsafe { &*ptr };
                if let Some(grad) = p.grad_data() {
                    let v = &mut self.v[i];
                    for j in 0..v.len() {
                        let g = grad[j];
                        v[j] = self.alpha * v[j] + (1.0 - self.alpha) * g * g;
                    }
                    let new_data: Vec<f64> = p.data().iter().zip(grad.iter().zip(v.iter()))
                        .map(|(w, (g, vj))| w - self.lr * g / (vj.sqrt() + self.eps))
                        .collect();
                    p.set_data(new_data);
                }
            }
        }
    }
}

// =========================================================================
// data module -- Dataset and DataLoader
// =========================================================================

/// Data loading utilities for batch training.
pub mod data {
    use ml_framework_core::Tensor;

    /// A dataset wrapping two tensors (features and labels).
    pub struct TensorDataset {
        pub x: Tensor,
        pub y: Tensor,
        len: usize,
    }

    impl TensorDataset {
        pub fn new(x: Tensor, y: Tensor) -> Self {
            let len = x.shape()[0];
            TensorDataset { x, y, len }
        }

        pub fn len(&self) -> usize { self.len }
        pub fn is_empty(&self) -> bool { self.len == 0 }
    }

    /// Iterate over a dataset in batches.
    pub struct DataLoader {
        pub dataset: TensorDataset,
        pub batch_size: usize,
        pub shuffle: bool,
    }

    impl DataLoader {
        pub fn new(dataset: TensorDataset, batch_size: usize, shuffle: bool) -> Self {
            DataLoader { dataset, batch_size, shuffle }
        }

        /// Return batches as a vector of (x_batch, y_batch) tuples.
        pub fn batches(&self) -> Vec<(Tensor, Tensor)> {
            let n = self.dataset.len();
            let x_data = self.dataset.x.data();
            let y_data = self.dataset.y.data();
            let x_shape = self.dataset.x.shape();
            let y_shape = self.dataset.y.shape();
            let x_inner: usize = x_shape[1..].iter().product::<usize>().max(1);
            let y_inner: usize = y_shape[1..].iter().product::<usize>().max(1);

            let mut result = Vec::new();
            let mut start = 0;
            while start < n {
                let end = (start + self.batch_size).min(n);
                let batch_len = end - start;
                let x_batch_data: Vec<f64> = x_data[start * x_inner..end * x_inner].to_vec();
                let y_batch_data: Vec<f64> = y_data[start * y_inner..end * y_inner].to_vec();
                let mut x_batch_shape = vec![batch_len];
                x_batch_shape.extend_from_slice(&x_shape[1..]);
                let mut y_batch_shape = vec![batch_len];
                y_batch_shape.extend_from_slice(&y_shape[1..]);
                result.push((
                    Tensor::from_slice(&x_batch_data, &x_batch_shape, false, "cpu"),
                    Tensor::from_slice(&y_batch_data, &y_batch_shape, false, "cpu"),
                ));
                start = end;
            }
            result
        }

        pub fn num_batches(&self) -> usize {
            (self.dataset.len() + self.batch_size - 1) / self.batch_size
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
    fn test_tensor_creation() {
        let t = tensor(&[1.0, 2.0, 3.0], &[3], false);
        assert_eq!(t.data(), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_linear() {
        let layer = nn::Linear::new(3, 2, true);
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0], &[1, 3], false, "cpu");
        let y = nn::Module::forward(&layer, &x);
        assert_eq!(y.shape(), vec![1, 2]);
        assert_eq!(nn::Module::parameters(&layer).len(), 2); // weight + bias
    }

    #[test]
    fn test_linear_no_bias() {
        let layer = nn::Linear::new(3, 2, false);
        assert_eq!(nn::Module::parameters(&layer).len(), 1); // weight only
    }

    #[test]
    fn test_relu_module() {
        let relu = nn::ReLU;
        let x = Tensor::from_slice(&[-1.0, 0.0, 1.0], &[1, 3], false, "cpu");
        let y = nn::Module::forward(&relu, &x);
        assert_eq!(y.data(), vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_sequential() {
        let model = nn::Sequential::new(vec![
            Box::new(nn::Linear::new(3, 4, true)),
            Box::new(nn::ReLU),
            Box::new(nn::Linear::new(4, 2, true)),
        ]);
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0], &[1, 3], false, "cpu");
        let y = nn::Module::forward(&model, &x);
        assert_eq!(y.shape(), vec![1, 2]);
        // Should have 4 parameters: 2 weights + 2 biases
        assert_eq!(nn::Module::parameters(&model).len(), 4);
    }

    #[test]
    fn test_mse_loss() {
        let loss_fn = nn::MSELoss::new("mean");
        let pred = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], false, "cpu");
        let target = Tensor::from_slice(&[1.5, 2.5, 3.5], &[3], false, "cpu");
        let loss = loss_fn.forward(&pred, &target);
        assert!((loss.item().unwrap() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_l1_loss() {
        let loss_fn = nn::L1Loss::new("mean");
        let pred = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], false, "cpu");
        let target = Tensor::from_slice(&[1.5, 2.5, 3.5], &[3], false, "cpu");
        let loss = loss_fn.forward(&pred, &target);
        assert!((loss.item().unwrap() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_sgd_optimizer() {
        let p = Parameter::new(Tensor::from_slice(&[1.0, 2.0], &[2], true, "cpu"));
        // Simulate gradient
        let y = p.tensor.pow(2.0);
        let loss = y.sum(None, false);
        loss.backward(None).unwrap();

        let mut sgd = optim::SGD::new(vec![&p], 0.1, 0.0, 0.0);
        optim::Optimizer::step(&mut sgd);
        let data = p.data();
        // grad of x^2 at x=1 is 2, x=2 is 4
        // new = old - lr * grad = 1 - 0.1*2 = 0.8, 2 - 0.1*4 = 1.6
        assert!((data[0] - 0.8).abs() < 1e-10);
        assert!((data[1] - 1.6).abs() < 1e-10);
    }

    #[test]
    fn test_adam_optimizer() {
        let p = Parameter::new(Tensor::from_slice(&[1.0], &[1], true, "cpu"));
        let y = p.tensor.pow(2.0);
        y.backward(None).unwrap();

        let mut adam = optim::Adam::default_with_params(vec![&p]);
        optim::Optimizer::step(&mut adam);
        assert!(p.data()[0] != 1.0); // Should have changed
    }

    #[test]
    fn test_functional_relu() {
        let x = Tensor::from_slice(&[-1.0, 0.0, 1.0], &[3], false, "cpu");
        let y = nn::functional::relu(&x);
        assert_eq!(y.data(), vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_flatten() {
        let flatten = nn::Flatten;
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[1, 2, 3], false, "cpu");
        let y = nn::Module::forward(&flatten, &x);
        assert_eq!(y.shape(), vec![1, 6]);
    }

    #[test]
    fn test_tensor_dataset() {
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[3, 2], false, "cpu");
        let y = Tensor::from_slice(&[0.0, 1.0, 1.0], &[3], false, "cpu");
        let ds = data::TensorDataset::new(x, y);
        assert_eq!(ds.len(), 3);
    }

    #[test]
    fn test_dataloader() {
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[3, 2], false, "cpu");
        let y = Tensor::from_slice(&[0.0, 1.0, 1.0], &[3], false, "cpu");
        let ds = data::TensorDataset::new(x, y);
        let loader = data::DataLoader::new(ds, 2, false);
        let batches = loader.batches();
        assert_eq!(batches.len(), 2); // 2 + 1
        assert_eq!(batches[0].0.shape()[0], 2);
        assert_eq!(batches[1].0.shape()[0], 1);
    }

    #[test]
    fn test_bce_loss() {
        let loss_fn = nn::BCELoss::new("mean");
        let pred = Tensor::from_slice(&[0.9, 0.1, 0.8], &[3], false, "cpu");
        let target = Tensor::from_slice(&[1.0, 0.0, 1.0], &[3], false, "cpu");
        let loss = loss_fn.forward(&pred, &target);
        // Loss should be small since predictions are close to targets
        assert!(loss.item().unwrap() < 0.5);
    }

    #[test]
    fn test_rmsprop_optimizer() {
        let p = Parameter::new(Tensor::from_slice(&[1.0], &[1], true, "cpu"));
        let y = p.tensor.pow(2.0);
        y.backward(None).unwrap();
        let mut rms = optim::RMSprop::new(vec![&p], 0.01, 0.99, 1e-8);
        optim::Optimizer::step(&mut rms);
        assert!(p.data()[0] != 1.0);
    }
}
