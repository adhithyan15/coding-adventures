# @coding-adventures/huffman-compression

Huffman (1952) lossless compression and decompression — CMP04.

Part of the [coding-adventures](https://github.com/adhithya/coding-adventures) monorepo.

## What it does

Compresses and decompresses byte arrays using Huffman entropy coding. Frequent bytes get short codes; rare bytes get long codes. The result is provably optimal — no other prefix-free code achieves smaller expected output for the same symbol distribution.

This package delegates all tree construction and canonical code derivation to [`@coding-adventures/huffman-tree`](../huffman-tree) (DT27), mirroring how LZ78 delegates to the trie package.

## Wire format (CMP04)

```
Bytes 0–3:    original_length  (big-endian uint32)
Bytes 4–7:    symbol_count     (big-endian uint32)
Bytes 8–8+2N: code-lengths table — N entries of [symbol (uint8), length (uint8)]
              sorted by (length, symbol) ascending
Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary
```

## Usage

```typescript
import { compress, decompress } from "@coding-adventures/huffman-compression";

const original = new TextEncoder().encode("AAABBC");
const compressed = compress(original);
const recovered = decompress(compressed);

console.log(new TextDecoder().decode(recovered)); // "AAABBC"
```

## Series context

| Package | Algorithm     | Year | Notes                              |
|---------|---------------|------|------------------------------------|
| CMP00   | LZ77          | 1977 | Sliding-window backreferences      |
| CMP01   | LZ78          | 1978 | Explicit trie dictionary           |
| CMP02   | LZSS          | 1982 | LZ77 + flag bits                   |
| CMP03   | LZW           | 1984 | Powers GIF                         |
| **CMP04** | **Huffman** | **1952** | **This package — entropy coding** |
| CMP05   | DEFLATE       | 1996 | LZ77 + Huffman; ZIP/gzip/PNG       |

## License

MIT
