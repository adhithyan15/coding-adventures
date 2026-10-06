# Perceptron (Go)

A small educational binary classifier built from the repository's Go matrix,
activation-functions, and loss-functions packages. Training uses batch gradient
descent with sigmoid activation and binary cross-entropy loss.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/perceptron"

features := [][]float64{{0, 0}, {0, 1}, {1, 0}, {1, 1}}
labels := [][]float64{{0}, {0}, {0}, {1}}

model := perceptron.New(0.8, 5000)
model.Fit(features, labels, 5001)
predictions := model.Predict(features)
```

`Fit` prints progress every `logSteps` epochs, so callers must provide a
positive interval. `Predict` returns `nil` and prints a diagnostic when called
before training. The trained weight matrix has one row per input feature and
one output column.

## Running Tests

```bash
go test ./... -v -cover
go vet ./...
```
