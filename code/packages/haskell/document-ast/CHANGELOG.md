# Changelog

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Add immutable TE00 block and inline node types.
- Add GFM task-list, strikethrough, and table node types.
- Add exhaustive algebraic unions and stable discriminator helpers.
