# loss-functions (Java)

Fundamental error functions for machine learning: MSE, MAE, BCE, and CCE with derivatives for backpropagation.

## What It Does

| Function | Task | Formula |
|----------|------|---------|
| MSE | Regression | (1/n) sum((y - y_hat)^2) |
| MAE | Regression | (1/n) sum(abs(y - y_hat)) |
| BCE | Binary classification | -(1/n) sum(y log(y_hat) + (1-y) log(1-y_hat)) |
| CCE | Multi-class classification | -(1/n) sum(y log(y_hat)) |

Each function has a corresponding derivative for use in gradient descent.

## Where It Fits

- **Layer:** ML01 (leaf package, zero dependencies)
- **Spec:** `code/specs/ML01-loss-functions.md`
- **Used by:** Gradient descent, neural network training loops

## Usage

```java
import com.codingadventures.lossfunctions.LossFunctions;

double[] yTrue = {1.0, 0.0, 0.0};
double[] yPred = {0.9, 0.1, 0.2};

double loss = LossFunctions.mse(yTrue, yPred);       // 0.02
double[] grad = LossFunctions.mseDerivative(yTrue, yPred);
```

## Running Tests

```bash
gradle test
```
