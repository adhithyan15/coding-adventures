# wasm/lzss

WebAssembly bindings for the Rust `lzss` crate (CMP02).

## What it does

This package compiles the pure-Rust LZSS implementation to WebAssembly and
exposes two functions to JavaScript/TypeScript via [wasm-bindgen]:

| Function | Description |
|---|---|
| `compress(data)` | Encode a `Uint8Array` into CMP02 wire-format bytes |
| `decompress(data)` | Recover original bytes from CMP02 wire-format bytes |

LZSS (Lempel-Ziv-Storer-Szymanski, 1982) scans input left-to-right. Wherever
a byte sequence already appeared in the last 4096 bytes, it emits a compact
`(offset, length)` back-reference instead of repeating the bytes. A per-block
flag byte distinguishes literals from back-references so no "next character"
byte is wasted after every token.

## Stack position

```
code/packages/wasm/lzss    ← this package (WebAssembly + JS glue)
        │
        └── depends on
code/packages/rust/lzss    ← pure-Rust CMP02 implementation
```

The `wasm/lzss` crate is a standalone Cargo workspace (`[workspace]` in its
`Cargo.toml`) so it does not pollute the Rust workspace lock-file.

## CMP series

```
CMP00 (LZ77,    1977) — Sliding-window backreferences
CMP01 (LZ78,    1978) — Explicit trie dictionary
CMP02 (LZSS,    1982) — LZ77 + flag bits  ← this package
CMP03 (LZW,     1984) — LZ78 + pre-seeded alphabet; GIF
CMP04 (Huffman, 1952) — Entropy coding
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP / gzip / PNG
```

## Wire format (CMP02)

```
Bytes 0–3  original_length   big-endian u32
Bytes 4–7  block_count       big-endian u32
Bytes 8+   blocks …

Each block:
  [1 byte flag]  bit i = 0 → next symbol is a literal byte
                 bit i = 1 → next symbol is a 3-byte back-reference
                             (offset high byte, offset low byte, length)
```

## Usage from JavaScript / TypeScript

```js
import init, { compress, decompress } from './lzss_wasm.js';

await init(); // loads and instantiates the .wasm binary

const enc = new TextEncoder();
const dec = new TextDecoder();

// Compress
const original   = enc.encode("hello hello hello world world world");
const compressed = compress(original);
console.log(`${original.length} → ${compressed.length} bytes`);

// Decompress
const recovered = decompress(compressed);
console.log(dec.decode(recovered)); // "hello hello hello world world world"
```

```ts
// TypeScript — wasm-pack generates types automatically
import init, { compress, decompress } from './lzss_wasm';

await init();

function roundtrip(data: Uint8Array): Uint8Array {
  return decompress(compress(data));
}
```

## Building with wasm-pack

```sh
# Install wasm-pack (once)
cargo install wasm-pack

# Build for the browser (outputs pkg/)
wasm-pack build --target web

# Build for Node.js
wasm-pack build --target nodejs

# Run wasm-bindgen tests under Node
wasm-pack test --node
```

## Running native tests

```sh
cargo test -- --nocapture
```

The native test suite runs on the host (no browser or wasm runtime needed)
because the `#[cfg(not(target_arch = "wasm32"))]` guard keeps native tests
separate from the `wasm_bindgen_test` browser/Node tests.

[wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen
