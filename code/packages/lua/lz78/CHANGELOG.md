# Changelog

## [Unreleased]

### Security

- Hardening (CWE-789): deserialise_tokens clamps the token count to the data present; decode stops once original_length is reached (quadratic expansion before truncation) and on a dangling dict_index (previously 'attempt to index a nil value'). NOT RUN locally (toolchain unavailable in the review environment).
- Added tests/test_hostile_headers.lua (busted): hostile headers with a wall-clock bound, plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (toolchain unavailable in the review environment).

## [0.1.0] - 2026-04-11

### Added

- `TrieCursor` — arena-based step-by-step trie cursor (exported, reusable for LZW)
  - `new()`, `step()`, `insert()`, `reset()`, `dict_id()`, `at_root()`, `entries()`
- `encode(data, max_dict_size)` — encode string to LZ78 token array
- `decode(tokens, original_length)` — decode token array to string
- `compress(data, max_dict_size)` — one-shot compress with CMP01 wire format
- `decompress(data)` — one-shot decompress
- `serialise_tokens(tokens, original_length)` / `deserialise_tokens(data)`
- 33 tests covering spec vectors, round-trips, TrieCursor, wire format
