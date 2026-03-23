# ml-framework-tf

TensorFlow-compatible API built on top of ml-framework-core.

## Key Abstractions

1. **tf.constant / tf.Variable** -- Immutable vs mutable tensors
2. **tf.GradientTape** -- Explicit gradient tracking (TF's defining feature)
3. **tf.keras** -- High-level layers, models, optimizers, losses
4. **tf.nn** -- Neural network activation functions
5. **tf.math** -- Element-wise mathematical operations
6. **tf.random** -- Random tensor generation
7. **tf.data** -- Data pipeline utilities (Dataset)

## TF vs PyTorch: Key Differences

| Feature              | TensorFlow (this)           | PyTorch                   |
|----------------------|-----------------------------|---------------------------|
| Gradient tracking    | Explicit (GradientTape)     | Implicit (requires_grad)  |
| Mutable tensors      | tf.Variable                 | Any tensor                |
| Training loop        | model.fit() (batteries)     | Manual loop (flexible)    |
| Axis naming          | axis= parameter             | dim= parameter            |
| Loss arg order       | (y_true, y_pred)            | (pred, target)            |
| Optimizer update     | apply_gradients(zip(...))   | optimizer.step()          |

## Usage

```rust
use ml_framework_tf::*;

// Create variables
let w = Variable::new_from_slice(&[1.0, 2.0, 3.0], &[3], true, None);

// Compute gradients with GradientTape
let mut tape = GradientTape::new(false);
tape.watch(&w.tensor);
let y = w.tensor.pow(2.0);
let loss = reduce_sum(&y, None, false);
let grads = tape.gradient(&loss, &[&w.tensor]).unwrap();
```
