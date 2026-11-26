# coding_adventures_brotli (Dart)

Brotli lossless compression (CMP06). The Dart lane of the Rust
`brotli` crate.

```dart
import 'package:coding_adventures_brotli/brotli.dart';

final wire = compress('hello hello hello'.codeUnits);
final back = decompress(wire); // == original
```

## API

- `compress(data)` — LZ matching plus canonical Huffman trees
  into CMP06 wire bytes.
- `decompress(data)` — validates tables, decodes to the header
  length; throws [BrotliError] on any malformation.
- `literalContext(lastByte)` — the 0–3 context bucket.
- Errors: [BrotliError] with twelve subtypes. Wire-format
  messages mirror the reference texts exactly; the three
  compress guards are lane-defined (see below).

The Huffman trees build over the in-lane huffman-tree package
(deterministic construction; the reference iterates hash maps,
so wire bytes may differ run to run — both decoders accept any
valid table). The ICC fallback the reference carries is proven
unreachable (covering tiers nest, insert ranges chain
contiguously, pass 1 caps at the maximum), so lookup failures
throw loudly instead of degrading silently.

## Fail-closed divergences

The reference silently corrupts two cases; the lane refuses:

- A literal bucket with all 256 symbols truncates its 1-byte
  table count to 0 (proven against the reference on random 2KB+
  inputs) → [BrotliTooManyLiteralSymbols].
- Inputs past the 4 GiB header field truncate
  (`original_length as u32`) → [BrotliInputTooLarge].
- Out-of-range table symbols panic the reference on fixed-table
  indexing → [BrotliInvalidTableSymbol].

All three are logged as Rust follow-ups.

## Validation

- **Unit** (`test/brotli_test.dart`): mirrors all 15 reference
  tests (round-trips, wire format, contexts, matches, ratios),
  plus truncation/bit-flip matrices, the fail-closed guards,
  exact error texts, and value equality. 21 tests.
- **Property** (`test/properties_test.dart`): round-trips,
  determinism, context totals, header lengths, run
  monotonicity, and the empty wire form — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile inputs and wire mutations (truncations
  decode exactly or throw; flips never escape the error
  vocabulary); only [BrotliError] escapes. Ad-hoc 20,000-iteration
  runs over two more seeds: zero crashes.
- **Coverage**: 100% lines and functions.
