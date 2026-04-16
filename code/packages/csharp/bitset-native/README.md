# bitset-native

Native C# wrapper over the Rust `bitset-c` library.

## What It Includes

- A managed `Bitset` class backed by the Rust `bitset` implementation
- P/Invoke bindings to the `bitset-c` shared library
- The same core API shape as the pure C# package: constructors, single-bit operations, bulk ops, queries, and conversions
- Deterministic cleanup through `IDisposable`

## Example

```csharp
using CodingAdventures.BitsetNative;

using var left = Bitset.FromBinaryString("1100");
using var right = Bitset.FromBinaryString("1010");
using var intersection = left & right;

Console.WriteLine(intersection.ToBinaryString()); // 1000
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
