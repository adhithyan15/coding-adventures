# bitset

Compact boolean arrays packed into 64-bit words for space-efficient storage and fast bulk bitwise operations.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- Dynamic growth when `Set` or `Toggle` reach beyond the current logical length
- Bulk bitwise operations: AND, OR, XOR, NOT, and AND-NOT
- Counting and queries: `PopCount`, `Any`, `All`, `None`, `Length`, and `Capacity`
- Conversions to and from binary strings plus 64-bit integer conversion when the value fits
- Efficient iteration over set-bit indices using trailing-zero counting

## Example

```csharp
using CodingAdventures.Bitset;

var left = Bitset.FromBinaryString("1100");
var right = Bitset.FromBinaryString("1010");

var intersection = left & right;
Console.WriteLine(intersection.ToBinaryString()); // 1000
```

## Development

```bash
# Run tests
bash BUILD
```
