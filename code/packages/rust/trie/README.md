# trie

A prefix tree for string keys with exact lookup, autocomplete, deletion, and longest-prefix matching.

## What It Provides

- `Trie<V>` keyed by `&str`
- exact lookup and deletion
- `starts_with`, `words_with_prefix`, and `longest_prefix_match`
- lexicographic key collection and structural validation helpers

## Building and Testing

```bash
cargo test -p trie -- --nocapture
```
