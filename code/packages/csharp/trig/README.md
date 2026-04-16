# trig

C# implementation of the `trig` foundation package.

This package computes `sin`, `cos`, `tan`, `sqrt`, `atan`, and `atan2`
without delegating to the host runtime's trigonometry helpers. It is intended
as a leaf-level teaching package that other geometry and physics packages can
build on.

## API

- `Trig.PI`
- `Trig.Sin(x)`
- `Trig.Cos(x)`
- `Trig.Tan(x)`
- `Trig.Sqrt(x)`
- `Trig.Atan(x)`
- `Trig.Atan2(y, x)`
- `Trig.Radians(deg)`
- `Trig.Degrees(rad)`

## Usage

```csharp
using CodingAdventures.Trig;

var theta = Trig.Radians(45.0);
var sine = Trig.Sin(theta);
var cosine = Trig.Cos(theta);
var tangent = Trig.Tan(theta);
```

## Development

```bash
bash BUILD
```
