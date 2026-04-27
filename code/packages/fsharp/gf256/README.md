# gf256

Finite-field arithmetic over `GF(2^8)` for byte-oriented coding theory and
crypto building blocks such as Reed-Solomon, QR, and AES-style field math.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- Reed-Solomon style field operations over primitive polynomial `0x11D`
- Public `LOG` and `ALOG` tables for learning and debugging
- `add`, `subtract`, `multiply`, `divide`, `power`, `inverse`
- `createField` for alternate primitive polynomials such as AES `0x11B`

## Example

```fsharp
open CodingAdventures.Gf256

let product = Gf256.multiply 0x53uy 0x8cuy
printfn "%d" product // 1

let aes = Gf256.createField 0x11b
printfn "%d" (aes.Multiply(0x57uy, 0x83uy)) // 193
```

## Development

```bash
# Run tests
bash BUILD
```
