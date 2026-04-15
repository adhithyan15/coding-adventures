# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-11

### Added

- `CodingAdventures.LZ78.TrieCursor` — generic arena-based step-by-step trie cursor.
  Exported for use in LZW (CMP03) and other streaming dictionary algorithms.
  - `new/0`, `step/2`, `insert/3`, `reset/1`, `dict_id/1`, `at_root?/1`
  - `to_list/1` for DFS enumeration of all dictionary entries
- `CodingAdventures.LZ78.encode/2` — encode binary to LZ78 token list
- `CodingAdventures.LZ78.decode/2` — decode token list to binary
- `CodingAdventures.LZ78.compress/2` — one-shot compress with CMP01 wire format
- `CodingAdventures.LZ78.decompress/1` — one-shot decompress
- `CodingAdventures.LZ78.serialise_tokens/2` and `deserialise_tokens/1`
- 40 tests covering: spec vectors, round-trips, TrieCursor, wire format, compression ratio
