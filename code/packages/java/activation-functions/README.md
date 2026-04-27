# activation-functions (Java)

Non-linear activation functions for neural networks: Sigmoid, ReLU, and Tanh with derivatives for backpropagation.

## Where It Fits

- **Layer:** ML04 (leaf package, zero dependencies)
- **Spec:** `code/specs/ML04-activation-functions.md`

## Usage

```java
import com.codingadventures.activationfunctions.ActivationFunctions;

double output = ActivationFunctions.sigmoid(0.0);    // 0.5
double grad = ActivationFunctions.sigmoidDerivative(0.0); // 0.25
double activated = ActivationFunctions.relu(-3.0);    // 0.0
```

## Running Tests

```bash
gradle test
```
