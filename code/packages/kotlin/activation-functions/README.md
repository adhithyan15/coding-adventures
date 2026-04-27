# activation-functions (Kotlin)

Non-linear activation functions for neural networks: Sigmoid, ReLU, and Tanh with derivatives for backpropagation.

## Where It Fits

- **Layer:** ML04 (leaf package, zero dependencies)
- **Spec:** `code/specs/ML04-activation-functions.md`

## Usage

```kotlin
import com.codingadventures.activationfunctions.ActivationFunctions

val output = ActivationFunctions.sigmoid(0.0)    // 0.5
val grad = ActivationFunctions.sigmoidDerivative(0.0) // 0.25
val activated = ActivationFunctions.relu(-3.0)    // 0.0
```

## Running Tests

```bash
gradle test
```
