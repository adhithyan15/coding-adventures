# UUID v1 Shared Fixture

Neutral conformance vectors for the uuid identity (`code/specs/uuid.md`).
`cases.json` carries every expectation literally — no aggregate hashes stand
in for per-case checks.

## Groups

- `namespaces`: the four RFC 4122 namespace UUIDs plus nil and max, as hex
  and canonical strings.
- `known_answers`: hand-pinned vectors — the RFC v3/v5 `python.org` vectors,
  the corrected v5 widgets.com vector (see uuid.md test 22), and a Unicode
  v5 vector.
- `parse_cases`: valid inputs across all four accepted forms (standard,
  compact, braced, URN), both hex cases, surrounding whitespace, and the
  reference hyphen leniency, each with expected bytes and decoded fields.
- `invalid_inputs`: strings every lane must reject (wrong length, non-hex,
  garbage, doubled braces, truncated URN).
- `v3_cases` / `v5_cases`: 128 seeded namespace+name pairs each with expected
  UUIDs. Names come from a URI-safe alphabet, lengths 0-47.
- `byte_patterns`: 84 sixteen-byte patterns (edges, singletons, counting,
  seeded) with expected string/URN/version/variant for round-trip checks.

Time-based versions (v1, v7) and random v4 have no fixture vectors: their
outputs are nondeterministic by design. Lanes test those natively (version,
variant, shape, uniqueness, timestamp decoding).

## Provenance

`oracle.py` is an independent pure-stdlib implementation written from the RFCs.
`generate.py` derives every expectation from it using only a SplitMix64 stream
seeded `0x555549445F7631` ("UUID_v1"). `validate.py` re-derives all
expectations and rejects any `cases.json` that differs from regeneration.

```bash
cd code/specs/fixtures/uuid-v1
python3 generate.py   # rewrites cases.json deterministically
python3 validate.py   # exits 0 only if cases.json matches regeneration
```

## Consumer contract

A consuming lane replays every group through its public API:

1. Assert `schema == "uuid-v1"` and the exact group counts (4 known answers,
   15 parse cases, 19 invalid inputs, 128 v3, 128 v5, 84 byte patterns), so a
   truncated file cannot pass silently.
2. Parse each `parse_cases` input and compare bytes, string, URN, version,
   variant, nil, and max flags.
3. Reject every `invalid_inputs` entry through both the throwing and boolean
   entry points, requiring agreement.
4. Compute every `v3_cases`/`v5_cases` entry and compare the full string.
5. Round-trip every `byte_patterns` entry through string, hex, and URN forms.

First consumer: `dart/uuid`. Rust, TypeScript, and Python adoption is logged
follow-up work.
