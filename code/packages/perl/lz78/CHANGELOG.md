# Changelog

## [0.1.0] - 2026-04-11

### Added

- Exported TrieCursor functions: `new_cursor`, `cursor_step`, `cursor_insert`,
  `cursor_reset`, `cursor_dict_id`, `cursor_at_root`, `cursor_entries`
- `encode($data, $max_dict_size)` — encode binary string to LZ78 token list
- `decode(\@tokens, $original_length)` — decode token list to binary string
- `compress($data, $max_dict_size)` — one-shot compress with CMP01 wire format
- `decompress($bytes)` — one-shot decompress
- `serialise_tokens` / `deserialise_tokens`
- Test suite covering spec vectors, round-trips, TrieCursor, wire format
