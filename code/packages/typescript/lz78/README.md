# lz78 — LZ78 Lossless Compression Algorithm (TypeScript)

LZ78 (Lempel & Ziv, 1978) explicit-dictionary compression. Part of the CMP series.

## Usage

```typescript
import { compress, decompress, encode, decode } from "@coding-adventures/lz78";

const data = new TextEncoder().encode("hello hello hello world");
const compressed = compress(data);
const original   = decompress(compressed);

// Token-level
const tokens = encode(data);
```

## Development

```bash
npm install
npx vitest run
```
