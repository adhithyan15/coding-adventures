# gradient-descent (Dart)

Dependency-free Dart port of **ML02 Gradient Descent** (`code/specs/ML02-gradient-descent.md`).
It provides the vanilla stochastic gradient descent (SGD) update:

$$w_{new}[i] = w_{old}[i] - (\alpha \cdot \nabla w[i])$$

## Usage

```dart
import 'package:coding_adventures_gradient_descent/gradient_descent.dart';

final updated = sgd([1.0, -0.5, 2.0], [0.1, -0.2, 0.0], 0.1);
// [0.99, -0.48, 2.0]  (an unmodifiable List<double>)
```

## Numeric contract

- Uses IEEE-754 binary64 with **unfused** multiply-then-subtract (`step = lr * g; w - step`) and no FMA, so every language lane produces bit-identical results.
- Returns a **new unmodifiable list** and never mutates the inputs. Aliased calls such as `sgd(x, x, lr)` work correctly.
- Checks errors in a fixed order and throws `InvalidGradientDescentInputError` with a closed `GradientDescentErrorCode`:
  1. `length-mismatch`
  2. `empty-input`
  3. `input-too-large` (more than 1,048,576 elements; checked before allocation)
  4. `non-finite-learning-rate`
  5. `non-finite-input`
  6. `numeric-overflow`

  The `null-input` code exists only for parity with Java and Kotlin. Dart's sound null safety makes it unreachable.

## 5-Class ML02 Optimizer Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **1: Validation Precedence** | Ambiguous or input-dependent error reporting | A fixed, documented check order where the first failure wins. It is identical across Dart, Java, Kotlin and Swift. |
| **2: CWE-400 Vector DoS Ceiling** | Memory exhaustion from huge inputs | `maxVectorLength = 1048576` is enforced **before** the output buffer is allocated. |
| **3: CWE-682 NaN/Infinity Poisoning & Overflow** | One poisoned gradient silently turns a model's weights into NaN or Infinity | A full pre-pass rejects non-finite weights, gradients and learning rates. Any update that overflows fails closed with `numeric-overflow` instead of returning ±Infinity. |
| **4: Alias & Caller-Mutation Isolation** | Corruption through shared buffers | The output is always a fresh unmodifiable list. Inputs are read-only, and `sgd(x, x, lr)` is safe. |
| **5: CWE-209 Payload-Blind Confidentiality** | Model weights, learning rates or shapes leaking through error messages | Errors carry only a closed code and a static message. They never include values, indices or lengths. |

## Development

```bash
sh BUILD   # format, analyze, tests, coverage (>= 95%)
```
