# gradient-descent (Swift)

Dependency-free Swift port of **ML02 Gradient Descent** (`code/specs/ML02-gradient-descent.md`).
It provides the vanilla stochastic gradient descent (SGD) update:

$$w_{\text{new}}[i] = w_{\text{old}}[i] - (\alpha \cdot \nabla w[i])$$

## Usage

```swift
import GradientDescent

let weights = [1.0, -0.5, 2.0]
let gradients = [0.1, -0.2, 0.0]
let learningRate = 0.1

let updated = try GradientDescent.sgd(weights, gradients, learningRate: learningRate)
// [0.99, -0.48, 2.0]
```

## Numeric contract

- Uses IEEE-754 binary64 with **unfused** multiply-then-subtract (`step = lr * g; updated = w - step`) and no FMA, so every language lane produces bit-identical results.
- Returns a **fresh array** and never mutates the inputs. Aliased calls such as `GradientDescent.sgd(x, x, learningRate: lr)` work correctly.
- Checks errors in a fixed order and throws `GradientDescentError` with a closed `GradientDescentErrorCode`:
  1. `length-mismatch`
  2. `empty-input`
  3. `input-too-large` (more than 1,048,576 elements; checked before allocation)
  4. `non-finite-learning-rate`
  5. `non-finite-input`
  6. `numeric-overflow`

  The `null-input` code exists only for parity with Java and Kotlin. Swift's non-optional array parameters make it unreachable.

## 5-Class ML02 Optimizer Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **Class 1: Validation Precedence** | Ambiguous or input-dependent error reporting | A fixed, documented check order where the first failure wins. It is identical across Dart, Java, Kotlin and Swift. |
| **Class 2: CWE-400 Vector DoS Ceiling** | Memory exhaustion from huge inputs | `maxVectorLength = 1_048_576` is enforced **before** the output buffer is allocated. |
| **Class 3: CWE-682 NaN/Infinity Poisoning & Overflow** | One poisoned gradient silently turns a model's weights into NaN or Infinity | A full pre-pass rejects non-finite weights, gradients and learning rates. Any update that overflows fails closed with `numeric-overflow` instead of returning ±Infinity. |
| **Class 4: Alias & Caller-Mutation Isolation** | Corruption through shared buffers | The output is always a fresh array. Inputs are read-only, and `sgd(x, x, learningRate: lr)` is safe. |
| **Class 5: CWE-209 Payload-Blind Confidentiality** | Model weights, learning rates or shapes leaking through error messages | Errors carry only a closed code and a static message. They never include values, indices or lengths. |

## Development

```bash
sh BUILD   # swift format lint --strict, tests with coverage (>= 95%), release build
```
