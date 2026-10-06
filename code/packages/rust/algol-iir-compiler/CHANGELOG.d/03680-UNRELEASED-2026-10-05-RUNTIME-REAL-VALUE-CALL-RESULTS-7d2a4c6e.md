## 0.368.0 - 2026-10-05 - Runtime-real value-scalar call results

- Preserve runtime-real formatter provenance for direct real procedure results
  when every formal parameter is a value-mode scalar. Name parameters, arrays,
  and formal procedures remain outside this bounded proof, so this adds no
  dynamic closure or thunk ABI.
