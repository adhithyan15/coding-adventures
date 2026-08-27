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
- `code/scripts/generate_scytale_fixture_consumers.py` renders the Scytale
  subset into dependency-free native tests for every established
  implementation lane. Its `--check` mode is the corpus-to-consumer drift
  gate.
- `code/scripts/generate_vigenere_fixture_consumers.py` renders all 26
  Vigenere cases into the same 15 native test lanes. The consumers execute
  complete expected objects, including normalized errors, analysis limits and
  ordering, deterministic ties, exact recovered keys, and break plaintexts.
- `code/scripts/generate_atbash_fixture_consumers.py` renders the exact six
  Atbash objects into all 15 native test lanes. The dependency-free consumers
  pin the corpus digest and case roster, construct strings from Unicode scalar
  values, and compare complete expected text objects.

The repository test
`code/scripts/tests/test_classical_cipher_fixtures.py` validates the schema,
case IDs, file and case-count bounds, Unicode scalar safety, stable errors, and
every expected result with a dependency-free semantic oracle. A schema-only
pass is not conformance: consumers must execute the operations and compare the
full expected object.

The generated Atbash, Scytale, and Vigenere consumers are derived only from the
bounded, strictly decoded fixture. Every output records the source SHA-256
digest and all selected case IDs, constructs repeat descriptors at test
runtime where applicable, normalizes public API results to the operation's
closed expected shape, and compares every field. Production packages do not
read fixture files and do not gain filesystem or JSON-parser authority. A
changed fixture, changed established-lane roster, missing output, or
hand-edited output makes `--check` fail closed.

Consumers must reject the encoded schema or fixture above 131,072 bytes and
scan both raw JSON inputs for bounded nesting before parsing. After the bounded
parse, reject duplicate object names, non-standard or non-finite numbers,
surrogates, and duplicate case IDs before recursive schema validation. Schema
references are fragment-local only. The validation errors
`fixture-size-limit`, `fixture-depth-limit`, `fixture-invalid-json`,
`fixture-invalid-scalar`, `fixture-duplicate-id`, and
`fixture-schema-invalid` are stable identifiers and never contain fixture
payloads.

## Operations

- `atbash-transform`: apply the fixed ASCII involution once.
- `scytale-encrypt`: transpose a Unicode-scalar sequence by columns.
- `scytale-decrypt`: reconstruct full and ragged column lengths, invert the
  grid, keep decomposed combining marks as independent scalar cells, and remove
  trailing `U+0020` only.
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

- at most 64 fixture cases, 131,072 bytes for either encoded JSON input,
  16 levels of schema JSON nesting, and eight levels of fixture JSON nesting;
- at most 8,193 scalars in a fixture string or fixed repeat descriptor;
- at most 4,096 scalars for Scytale brute force;
- at most 8,192 scalars and key length 40 for Vigenère analysis.

Cipher packages remain dependency-free and process-free. Loading this corpus
is test-only filesystem access; production code must not acquire filesystem,
network, subprocess, environment, clock, or randomness authority from it.
