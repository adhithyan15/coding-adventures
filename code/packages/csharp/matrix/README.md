# matrix

Pure C# immutable matrix mathematics helpers for double-precision values.

## What It Includes

- Construction from rectangular 2D arrays, scalars, row vectors, and zero-filled dimensions
- Element access with deep-copy data export
- Element-wise add/subtract, scalar add/subtract/scale, transpose, and dot product
- Value equality, hashing, and dimension validation

## Example

```csharp
using CodingAdventures.Matrix;

var a = new Matrix(new[] { new[] { 1.0, 2.0 }, new[] { 3.0, 4.0 } });
var b = Matrix.FromScalar(2.0);
var c = a.Scale(b[0, 0]).Transpose();
```

## Development

```bash
bash BUILD
```
