# Changelog

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Add CMP01 LZ78 token encoding and checked decoding.
- Add an immutable byte-trie cursor for stepwise dictionary construction.
- Add strict fixed-width wire serialization and one-shot compression helpers.
