# Changelog

## Unreleased - 2026-08-26

- Pin CR02 Unicode-scalar, combining-sequence, ragged-grid, and literal-padding behavior.
- Return checked brute-force results and reject inputs above 4096 scalars before allocation.
- Replace repeated list indexing with linear grid transposition and add a pure capability profile.

## 0.1.0 - 2026-07-18

- Add padded Scytale encryption and decryption with explicit key validation.
- Add brute-force candidate generation across the shared CR02 key range.
- Add tests for reference vectors, uneven candidates, round trips, character
  preservation, validation, and brute force.
