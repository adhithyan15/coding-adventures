# Changelog

## Unreleased - 2026-08-26

- Add a generated native consumer for all 18 language-neutral Scytale fixture cases.
- Align grid cells with CR02 Unicode scalar values, including decomposed combining sequences.
- Define ragged-ciphertext decryption and remove only trailing U+0020 padding.
- Reject brute-force inputs above 4096 scalars before allocating candidate output.

## 0.1.0

- Initial F# Scytale cipher package.
