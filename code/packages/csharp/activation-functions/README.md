# activation-functions

Scalar neural-network activation functions with derivatives, ported into C#
from the repo's dependency-free Rust math layer.

## What it provides

- `ActivationFunctions.Sigmoid(x)` and `ActivationFunctions.SigmoidDerivative(x)`
- `ActivationFunctions.Relu(x)` and `ActivationFunctions.ReluDerivative(x)`
- `ActivationFunctions.Tanh(x)` and `ActivationFunctions.TanhDerivative(x)`

## Why this is a good starter port

This package is a leaf: it depends only on `System.Math`, so it lets the new
.NET package roots grow without pulling in any sibling packages first. It also
sets the style baseline for future ports: small public API, parity tests, and
Knuth-style inline commentary in the source.

## Example

```csharp
using CodingAdventures.ActivationFunctions;

var probability = ActivationFunctions.Sigmoid(1.0);
var slope = ActivationFunctions.SigmoidDerivative(1.0);
```

## Development

```bash
bash BUILD
```
