# md5

From-scratch MD5 implementation following RFC 1321, including both one-shot
and streaming APIs.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- `SumMd5` for the 16-byte digest
- `HexString` and `ToHex` for lowercase hexadecimal rendering
- `Md5Hasher` for incremental hashing across multiple chunks
- Literate comments on little-endian block parsing and Davies-Meyer feed-forward

## Example

```csharp
using System.Text;
using CodingAdventures.Md5;

var digest = Md5.HexString(Encoding.UTF8.GetBytes("abc"));
Console.WriteLine(digest); // 900150983cd24fb0d6963f7d28e17f72
```

## Development

```bash
# Run tests
bash BUILD
```
