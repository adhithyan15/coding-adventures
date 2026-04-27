# beam-bytes-decoder

Low-level BEAM bytes decoding for pipeline-oriented tooling.

This package focuses on reusable container and chunk parsing:

- `FOR1` / `BEAM` container parsing
- Chunk table extraction
- Atom table decoding
- Code chunk header decoding
- Import/export table decoding

It intentionally stops short of instruction disassembly so future tools can
reuse it without pulling in VM semantics.
