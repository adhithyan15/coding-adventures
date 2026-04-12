# @coding-adventures/lzss — LZSS Compression (CMP02)

LZSS (1982) refines LZ77 with flag bits: literals cost 1 byte, matches cost 3 bytes.

## Usage

```ts
import { compress, decompress } from "@coding-adventures/lzss";

const data = new TextEncoder().encode("hello hello hello");
const compressed = compress(data);
const original   = decompress(compressed);
```

## Development

```bash
npm ci && npx vitest run
```
