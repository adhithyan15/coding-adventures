# Haskell Catch-Up Survey

## Snapshot

- Haskell package count: `108`
- Rust package count: `287`
- Shared package names today: `101`
- Haskell-only package names today: `discrete-waveform`, `ecmascript-es1-lexer`, `ecmascript-es3-lexer`, `ecmascript-es5-lexer`, `electronics`, `power-supply`, `wasm-assembler`

The largest remaining gap is no longer generic data structures. Rust still has
broader coverage across:

- tooling and repository automation
- compiler IR and optimizer layers
- crypto and compression
- graphics/rendering
- simulators and execution runtimes

Haskell now has broad generic data-structure coverage plus meaningful compiler,
parser, and runtime foundations.

## Generic Data Structures Status

This pass closes the generic data-structure gap in practice. Newly added
Haskell packages:

- `avl-tree`
- `b-plus-tree`
- `b-tree`
- `binary-search-tree`
- `binary-tree`
- `bitset`
- `bloom-filter`
- `fenwick-tree`
- `immutable-list`
- `lsm-tree`
- `red-black-tree`
- `rope`
- `segment-tree`
- `suffix-tree`
- `tree`
- `tree-set`
- `treap`
- `trie`

Remaining Rust-only structure-shaped packages are down to:

- `bitset-c`, which is a C-specific variant rather than a separate Haskell need
- `huffman-tree`, which fits better in the compression lane than the generic
  data-structure lane

## Tooling Status

- `code/programs/haskell/build-tool` now has a real implementation and uses the
  Haskell graph stack instead of carrying its own graph logic.
- `code/programs/haskell/scaffold-generator` exists and is already useful for
  adding new Haskell packages quickly.
- The next tooling gap to close is operational support: `file-system`,
  `cli-builder`, and a shared `core` layer that other packages can lean on.

## Recommended Backlog

### Tier 1: Tooling and Compiler Foundation

These unlock faster Haskell package growth and make the crypto batch easier to
build cleanly:

- `file-system`
- `cli-builder`
- `core`
- `compiler-ir`
- `ir-optimizer`
- `json-value`
- `json-serializer`

### Tier 2: Crypto

These are the best next parity target after the tooling/core pass:

- `aes`
- `sha1`
- `sha256`
- `sha512`
- `x25519`
- `ed25519`
- `hkdf`
- `hmac`
- `chacha20-poly1305`

### Tier 3: Compression

These fit naturally after the crypto primitives:

- `brotli`
- `deflate`
- `huffman-tree`

## Proposed Strategy

1. Finish the tooling/core lane so new Haskell packages can share file IO,
   CLI parsing, base types, and IR infrastructure instead of each package
   re-inventing its own helpers.
2. Use that foundation to add the crypto packages as a coherent batch, with
   shared byte handling and serialization support.
3. Follow immediately with the compression lane, which can reuse the new
   crypto-friendly binary utilities and absorb `huffman-tree`.
4. Only after those are in place should the focus shift to heavier rendering,
   simulator, and runtime breadth.

## Near-Term Goal

The practical definition of “Rust-level” for Haskell should be:

- repo-native tooling exists in Haskell
- core foundation and compiler substrate packages exist in Haskell
- crypto and compression primitives exist in Haskell
- new Haskell packages can be added with one command and built through the same
  repo workflow

That is the smallest milestone that turns Haskell from a strong experimental
lane into a broadly self-sufficient one.
