# Haskell Catch-Up Survey

## Snapshot

- Haskell package count: `8`
- Rust package count: `283`
- Shared package names today: `arithmetic`, `block-ram`, `clock`, `fpga`, `logic-gates`
- Haskell-only package names today: `discrete-waveform`, `electronics`, `power-supply`

The largest gap is not in one vertical. Rust already has broad coverage across:

- tooling and repository automation
- parsers and lexers
- data structures and storage
- graphics/rendering
- crypto/compression
- simulators and execution runtimes

Haskell is currently concentrated around digital logic and electronics.

## Tooling Status

- `code/programs/haskell/build-tool` existed as a stub before this pass and now has a real implementation shape.
- `code/programs/haskell/scaffold-generator` did not exist before this pass and now exists as a Haskell-native scaffold generator package.
- Rust still has a wider tooling surface overall, especially around generator breadth and richer build metadata support.

## Recommended Backlog

### Tier 1

These unblock more Haskell package growth immediately:

- `directed-graph`
- `file-system`
- `cli-builder`
- `content_addressable_storage`
- `json-rpc`

### Tier 2

These make Haskell more competitive for language tooling and compiler work:

- `lexer`
- `parser`
- `state-machine`
- `compiler-source-map`
- `grammar-tools`

### Tier 3

These extend Haskell beyond the current hardware-focused cluster:

- `graph`
- `hash-map`
- `hash-set`
- `b-tree`
- `b-plus-tree`
- `rope`
- `trie`
- `wave`

## Proposed Strategy

1. Stabilize the Haskell `build-tool` and `scaffold-generator` so new packages are cheap to add.
2. Port Tier 1 packages first to support package creation, dependency modeling, file IO, and simple CLIs.
3. Port Tier 2 packages once Tier 1 exists, so generated Haskell packages can share the same parsing/tooling substrate Rust already has.
4. Use the scaffold generator to create the missing packages with consistent `cabal`, `BUILD`, docs, and tests before filling in implementation details.

## Near-Term Goal

The practical definition of “Rust-level” for Haskell should be:

- repo-native tooling exists in Haskell
- core foundation packages exist in Haskell
- new Haskell packages can be added with one command and built through the same repo workflow

That is the smallest milestone that turns Haskell from a niche island in the repo into a scalable lane.
