# loss-functions

Pure vector loss functions and gradients for machine learning, ported into F#
from the repo's standalone Rust math layer.

## What it provides

- `LossFunctions.mse` and `LossFunctions.mseDerivative`
- `LossFunctions.mae` and `LossFunctions.maeDerivative`
- `LossFunctions.bce` and `LossFunctions.bceDerivative`
- `LossFunctions.cce` and `LossFunctions.cceDerivative`

## Why this is a good starter port

`loss-functions` stays leaf-level while still covering the things later ports
will need to get right: validation, floating-point edge cases, and parity with
other language implementations.

## Example

```fsharp
open CodingAdventures.LossFunctions

let mse = LossFunctions.mse [| 1.0; 0.0 |] [| 0.9; 0.1 |]
let grad = LossFunctions.mseDerivative [| 1.0; 0.0 |] [| 0.9; 0.1 |]
```

## Development

```bash
bash BUILD
```
