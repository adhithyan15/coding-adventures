### Changed — sharded Tamil script inventory ownership

- Give each existing Tamil letter and mark an ASCII code-point data shard and
  matching evidence owner while preserving the exact folded inventory order and
  data.
- Route TypeScript and Python inventory readers through the shard fold so
  authoring tools keep one canonical source and stable `tamil.json` provenance.
- Reject duplicate, mismatched, and escaping owners without adding a mutable
  whole-inventory hash to a shared test file.
