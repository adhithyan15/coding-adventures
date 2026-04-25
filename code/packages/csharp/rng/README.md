# CodingAdventures.Rng (C#)

Three classic pseudorandom number generators implemented in C# (.NET 9).

See `code/specs/rng.md` for the full specification.

## Algorithms

| Class | Algorithm | Period | Quality |
|---|---|---|---|
| `Lcg` | Linear Congruential Generator (Knuth 1948) | 2^64 | Moderate — fast, full period, some statistical correlations |
| `Xorshift64` | Xorshift (Marsaglia 2003) | 2^64 − 1 | Good — no multiplication, no correlations in lower bits |
| `Pcg32` | Permuted Congruential Generator (O'Neill 2014) | 2^64 | Excellent — passes TestU01 BigCrush and PractRand |

All three use identical Knuth constants:
- Multiplier: `6364136223846793005`
- Increment: `1442695040888963407`

## API

Every generator exposes five methods:

```csharp
uint  NextU32()                       // uniform uint in [0, 2^32)
ulong NextU64()                       // uniform ulong in [0, 2^64)
double NextFloat()                    // uniform double in [0.0, 1.0)
long  NextIntInRange(long min, long max)  // uniform long in [min, max]
```

## Usage

```csharp
using CodingAdventures.Rng;

// LCG — simplest, fastest
var lcg = new Lcg(42);
uint roll = lcg.NextU32();
long dice = lcg.NextIntInRange(1, 6);

// Xorshift64 — good quality, no multiplication
var xor = new Xorshift64(42);
double prob = xor.NextFloat();

// PCG32 — best quality, still fast
var pcg = new Pcg32(42);
ulong big = pcg.NextU64();
```

## Reference values (seed = 1)

| Generator | Output 1 | Output 2 | Output 3 |
|---|---|---|---|
| `Lcg` | 1817669548 | 2187888307 | 2784682393 |
| `Xorshift64` | 1082269761 | 201397313 | 1854285353 |
| `Pcg32` | 1412771199 | 1791099446 | 124312908 |

## Development

```bash
cd code/packages/csharp/rng
dotnet test tests/CodingAdventures.Rng.Tests/CodingAdventures.Rng.Tests.csproj --disable-build-servers
```

Or use the repo build tool:

```bash
bash BUILD
```
