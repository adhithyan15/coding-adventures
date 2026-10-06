# Activation Functions (Go)

A pure-Go collection of activation functions and derivatives used by the
repository's learning-oriented neural-network packages. The package provides
sigmoid, ReLU, leaky ReLU, tanh, softplus, and linear functions without host,
filesystem, network, or process authority.

## Usage

```go
import activation "github.com/adhithyan15/coding-adventures/code/packages/go/activation-functions"

probability := activation.Sigmoid(1.0)
slope := activation.SigmoidDerivative(1.0)
positive := activation.Relu(-3.0)
```

The implementations handle large sigmoid and softplus inputs without avoidable
floating-point overflow. Public calls participate in the repository's generated
Operations instrumentation while preserving their ordinary numeric return
values.

## Running Tests

```bash
go test ./... -v -cover
go vet ./...
```
