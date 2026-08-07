# huffman-tree

Pure Haskell implementation of DT27, the deterministic Huffman tree used by
the Huffman compression package.

## API

- `build` constructs a tree from `WeightPair` values through the existing
  Haskell `heap` package.
- `codeTable` and `codeFor` expose ordinary left-zero/right-one tree codes.
- `canonicalCodeTable` derives deterministic DEFLATE-style codes from the
  tree's code lengths.
- `decodeAll` decodes an exact symbol count and reports exhausted streams.
- `weight`, `depth`, `symbolCount`, `leaves`, and `isValid` expose structural
  inspection.

Construction follows the shared cross-language tie-break rules: lower weight,
leaf before internal, lower leaf symbol, then FIFO internal-node creation.

## Running the tests

```sh
cabal test all
```
