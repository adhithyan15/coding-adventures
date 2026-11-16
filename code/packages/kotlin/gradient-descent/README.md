# gradient-descent (Kotlin)

Pure Kotlin 2.1 implementation of the ML02 Stochastic Gradient Descent (SGD) parameter optimization algorithm conforming to `code/specs/ML02-gradient-descent.md`.

## Formula

$$w_{\text{new}}[i] = w_{\text{old}}[i] - (\alpha \cdot \nabla w[i])$$

## Usage

```kotlin
import com.codingadventures.gradientdescent.GradientDescent

val weights = doubleArrayOf(1.0, -0.5, 2.0)
val gradients = doubleArrayOf(0.1, -0.2, 0.0)
val learningRate = 0.1

val updated = GradientDescent.sgd(weights, gradients, learningRate)
// [0.99, -0.48, 2.0]
```

## 5-Class ML02 Optimizer Security Resilience Showcase

| Class | Vulnerability / Threat | Architectural Defense & Mitigation |
|---|---|---|
| **Class 1: Validation Precedence** | Ambiguous or order-dependent diagnostic errors | Deterministic validation hierarchy: (1) `null-input` -> (2) `length-mismatch` -> (3) `empty-input` -> (4) `input-too-large` -> (5) `non-finite-learning-rate` -> (6) `non-finite-input` -> (7) `numeric-overflow`. |
| **Class 2: Vector DoS Ceiling** | CWE-400 Unbounded memory allocation / resource exhaustion | Maximum vector length ceiling of 1,048,576 elements enforced strictly prior to output array allocation. |
| **Class 3: Poisoning & Overflow Refusal** | CWE-682 NaN/Infinity poisoning and overflow | Complete pre-pass validation rejects non-finite weights, gradients, and learning rates; intermediate/final overflow fails closed with `numeric-overflow`. |
| **Class 4: Alias & Caller Mutation** | State corruption via aliasing | Full alias safety on `sgd(x, x, lr)`; returns a fresh array and leaves caller inputs completely unmodified. |
| **Class 5: Payload-Blind Security** | CWE-209 Sensitive model weight reflection in error strings | Closed `GradientDescentErrorCode` enum and payload-blind `InvalidGradientDescentInputException` guarantee zero reflection of caller values or dimensions. |

## Build and Test

```bash
sh BUILD
```
