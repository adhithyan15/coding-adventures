# gf256

Finite-field arithmetic over `GF(2^8)` for byte-oriented coding theory and
crypto building blocks such as Reed-Solomon, QR, and AES-style field math.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- Reed-Solomon style field operations over primitive polynomial `0x11D`
- Public `LOG` and `ALOG` tables for learning and debugging
- `Add`, `Subtract`, `Multiply`, `Divide`, `Power`, `Inverse`
- `CreateField` for alternate primitive polynomials such as AES `0x11B`

## Example

```csharp
using CodingAdventures.Gf256;

var product = Gf256.Multiply(0x53, 0x8c);
Console.WriteLine(product); // 1

var aes = Gf256.CreateField(0x11b);
Console.WriteLine(aes.Multiply(0x57, 0x83)); // 193 (0xC1)
```

## Development

```bash
# Run tests
bash BUILD
```
