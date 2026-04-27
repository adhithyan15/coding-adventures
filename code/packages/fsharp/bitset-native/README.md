# bitset-native

Native F# wrapper over the Rust `bitset-c` library.

## What It Includes

- A managed `Bitset` type backed by the Rust `bitset` implementation
- P/Invoke bindings to the `bitset-c` shared library
- The same core API shape as the pure F# package: constructors, single-bit operations, bulk ops, queries, and conversions
- Deterministic cleanup through `IDisposable`

## Example

```fsharp
open CodingAdventures.BitsetNative

use left = Bitset.FromBinaryString("1100")
use right = Bitset.FromBinaryString("1010")
use intersection = left.And(right)

printfn "%s" (intersection.ToBinaryString()) // 1000
```

## Build Notes

The package `BUILD` script first compiles `rust/bitset-c`, then runs
`dotnet test`. The test project copies the resulting shared library into its
output directory so the runtime loader can resolve `bitset_c`.

## Development

```bash
# Run tests
bash BUILD
```
