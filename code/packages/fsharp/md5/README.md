# md5

From-scratch MD5 implementation following RFC 1321, including both one-shot
and streaming APIs.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- `Md5.sumMd5` for the 16-byte digest
- `Md5.hexString` and `Md5.toHex` for lowercase hexadecimal rendering
- `Md5.Md5Hasher` for incremental hashing across multiple chunks
- Literate comments on little-endian block parsing and Davies-Meyer feed-forward

## Example

```fsharp
open System.Text
open CodingAdventures.Md5

let digest = Md5.hexString (Encoding.UTF8.GetBytes("abc"))
printfn "%s" digest // 900150983cd24fb0d6963f7d28e17f72
```

## Development

```bash
# Run tests
bash BUILD
```
