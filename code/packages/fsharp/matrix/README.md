# matrix

Pure F# immutable matrix mathematics helpers for double-precision values.

## What It Includes

- Construction from rectangular 2D arrays, scalars, row vectors, and zero-filled dimensions
- Element access with deep-copy data export
- Element-wise add/subtract, scalar add/subtract/scale, transpose, and dot product
- Value equality, hashing, and dimension validation

## Example

```fsharp
open CodingAdventures.Matrix.FSharp

let a = Matrix [| [| 1.0; 2.0 |]; [| 3.0; 4.0 |] |]
let b = Matrix.FromScalar 2.0
let c = a.Scale(b[0, 0]).Transpose()
```

## Development

```bash
bash BUILD
```
