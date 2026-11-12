# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

### Security

- Hardening (CWE-789): decode stops once original_length is reached (quadratic expansion before truncation) and on a dangling dict_index (previously died with 'undefined value as an ARRAY reference'). The token count was already capped.
- Added t/02-hostile-headers.t (Test::More): hostile headers checked for bounded time and sane results, plus round trips over every byte string of length <= 2 and a seeded random sample. (The pre-existing t/*.t need Test2::V0, which is not installed in the local review environment.)

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
