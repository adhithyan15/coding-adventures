# radix-tree

A compressed trie for string keys that collapses single-child chains into labeled edges.

## What It Provides

- `RadixTree<V>` for string-keyed prefix indexes
- compressed-edge insertion and merge-on-delete behavior
- exact lookup, prefix queries, and longest-prefix matching
- deterministic key export and simple structural inspection helpers

## Building and Testing

```bash
cargo test -p radix-tree -- --nocapture
```
