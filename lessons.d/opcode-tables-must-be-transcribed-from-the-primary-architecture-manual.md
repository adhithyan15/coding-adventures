---
category: Compiler / VM / language pipeline
---

# Opcode tables must be transcribed from the primary architecture manual

Six System/360 shift operations were assigned to `0xBA` through `0xBF` from a
mistyped secondary table. The architecture defines SRA, SLA, SRDL, SLDL, SRDA,
and SLDA at `0x8A` through `0x8F`; tests copied the same bad table and therefore
confirmed the mistake instead of detecting it.

Transcribe opcode maps directly from the primary Principles of Operation, keep
the source table beside the implementation checklist, and add negative tests
for the falsely tempting encodings as well as positive tests for every real
opcode. A test derived from the implementation's own table is not an
independent conformance witness.
