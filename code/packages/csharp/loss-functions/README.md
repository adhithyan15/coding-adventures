# loss-functions

Pure vector loss functions and gradients for machine learning, ported into C#
from the repo's standalone Rust math layer.

## What it provides

- `LossFunctions.Mse` and `LossFunctions.MseDerivative`
- `LossFunctions.Mae` and `LossFunctions.MaeDerivative`
- `LossFunctions.Bce` and `LossFunctions.BceDerivative`
- `LossFunctions.Cce` and `LossFunctions.CceDerivative`

## Why this is a good starter port

`loss-functions` is another true leaf package. It has no sibling dependencies,
but it still exercises real numerical concerns such as epsilon clamping,
argument validation, and vector-shaped parity tests. That makes it a good
baseline for future .NET ML/math ports.

## Example

```csharp
using CodingAdventures.LossFunctions;

var mse = LossFunctions.Mse([1.0, 0.0], [0.9, 0.1]);
var grad = LossFunctions.MseDerivative([1.0, 0.0], [0.9, 0.1]);
```

## Development

```bash
bash BUILD
```
