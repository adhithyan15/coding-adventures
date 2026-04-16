# activation-functions

Scalar neural-network activation functions with derivatives, ported into F#
from the repo's dependency-free Rust math layer.

## What it provides

- `ActivationFunctions.sigmoid` and `ActivationFunctions.sigmoidDerivative`
- `ActivationFunctions.relu` and `ActivationFunctions.reluDerivative`
- `ActivationFunctions.tanh` and `ActivationFunctions.tanhDerivative`

## Why this is a good starter port

This package is a leaf: it depends only on `System.Math`, so it lets the new
.NET package roots grow without a sibling dependency chain. It also establishes
the literate F# style we want for later ports: public functions that are small,
parity-tested, and explained inline.

## Example

```fsharp
open CodingAdventures.ActivationFunctions

let probability = ActivationFunctions.sigmoid 1.0
let slope = ActivationFunctions.sigmoidDerivative 1.0
```

## Development

```bash
bash BUILD
```
