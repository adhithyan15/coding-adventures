# Classical ciphers v1 conformance fixture

This directory is the language-neutral executable corpus for CR01 Atbash,
CR02 Scytale, and CR03 Vigenère. The JSON source owns portable string units,
exact vectors, deterministic cryptanalysis choices, bounded resource limits,
and stable payload-blind error identifiers. It grants no runtime capability to
any cipher implementation.

## Files

- `schema.json` defines the closed fixture envelope and operation shapes.
- `cases.json` is the normative corpus.
- `CHANGELOG.md` records contract changes.

The repository test
`code/scripts/tests/test_classical_cipher_fixtures.py` validates the schema,
case IDs, file and case-count bounds, Unicode scalar safety, stable errors, and
every expected result with a dependency-free semantic oracle. A schema-only
pass is not conformance: consumers must execute the operations and compare the
full expected object.

## Operations

- `atbash-transform`: apply the fixed ASCII involution once.
- `scytale-encrypt`: transpose a Unicode-scalar sequence by columns.
- `scytale-decrypt`: invert the grid and remove trailing `U+0020` only.
- `scytale-brute-force`: return candidates in ascending-key order.
- `vigenere-encrypt` and `vigenere-decrypt`: transform ASCII letters while
  preserving all other scalars without advancing the key.
- `vigenere-find-key-length`: run the exact shortest-near-maximum IC rule.
- `vigenere-find-key`: run ascending-shift chi-squared without period
  shortening.
- `vigenere-break`: recover the key and plaintext with the two pinned stages.

The bounded `repeat_scalar` plus `repeat_count` input is data, not code. It is
available only on explicit resource-boundary cases and expands to exactly one
repeated Unicode scalar. Consumers must not evaluate expressions, follow
paths, interpolate environment values, or accept extension operations.

## Portable limits

- at most 64 fixture cases and 131,072 encoded fixture bytes;
- at most 8,193 scalars in a fixture string or fixed repeat descriptor;
- at most 4,096 scalars for Scytale brute force;
- at most 8,192 scalars and key length 40 for Vigenère analysis.

Cipher packages remain dependency-free and process-free. Loading this corpus
is test-only filesystem access; production code must not acquire filesystem,
network, subprocess, environment, clock, or randomness authority from it.
