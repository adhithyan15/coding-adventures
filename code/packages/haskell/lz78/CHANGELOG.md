# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0

- Add CMP01 LZ78 token encoding and checked decoding.
- Add an immutable byte-trie cursor for stepwise dictionary construction.
- Add strict fixed-width wire serialization and one-shot compression helpers.
