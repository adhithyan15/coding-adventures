# CodingAdventures.Rng (C#)

Pseudorandom number generator library implementing LCG, Xorshift64, and PCG32.

See `code/specs/rng.md` for the full specification.

## Usage

```csharp
var rng = new CodingAdventures.Rng.Pcg32(42);
uint value = rng.NextU32();
double f = rng.NextFloat();
long roll = rng.NextIntInRange(1, 6);
```

## Development

```bash
bash BUILD
```
