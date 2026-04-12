# Changelog

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
