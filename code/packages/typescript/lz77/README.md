# lz77 — LZ77 Lossless Compression Algorithm (TypeScript)

LZ77 sliding-window compression algorithm (Lempel & Ziv, 1977). Part of the CMP compression series in the coding-adventures monorepo.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                              |
|-------|----------------|------|------------------------------------------|
| CMP00 | **LZ77**       | 1977 | Sliding-window backreferences ← you are here |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no window    |
| CMP02 | LZSS           | 1982 | LZ77 + flag bits, no wasted literals     |
| CMP03 | LZW            | 1984 | Pre-initialized dictionary; powers GIF  |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib        |

## Usage

```ts
import { compress, decompress, encode, decode } from "@coding-adventures/lz77";

// One-shot compression / decompression (Uint8Array API)
const data = new TextEncoder().encode("hello hello hello world");
const compressed = compress(data);
const original = decompress(compressed);

// Token-level API
const tokens = encode(data);
const decoded = decode(tokens);

// Custom parameters
const tokens2 = encode(data, 2048, 128, 3);
```

## API

| Function | Signature | Description |
|----------|-----------|-------------|
| `encode` | `(data: Uint8Array, windowSize?, maxMatch?, minMatch?) → Token[]` | Encode to token stream |
| `decode` | `(tokens: Token[], initialBuffer?) → Uint8Array` | Decode token stream |
| `compress` | `(data: Uint8Array, windowSize?, maxMatch?, minMatch?) → Uint8Array` | Encode + serialise |
| `decompress` | `(data: Uint8Array) → Uint8Array` | Deserialise + decode |
| `serialiseTokens` | `(tokens: Token[]) → Uint8Array` | Serialise token list |
| `deserialiseTokens` | `(data: Uint8Array) → Token[]` | Deserialise token list |
| `token` | `(offset, length, nextChar) → Token` | Token constructor |

### Token

```ts
interface Token {
  readonly offset: number;   // Distance back (1..windowSize), or 0
  readonly length: number;   // Match length (0 = literal)
  readonly nextChar: number; // Literal byte after match (0..255)
}
```

### Parameters

| Parameter  | Default | Meaning |
|------------|---------|---------|
| windowSize | 4096    | Maximum lookback distance. |
| maxMatch   | 255     | Maximum match length. |
| minMatch   | 3       | Minimum match length for backreference. |

## Development

```bash
npm install
npx vitest run
npx vitest run --coverage
```

Coverage target: 95%+ (currently passing all 33 tests).
