## 0.372.0 - 2026-10-05 - Runtime-real array-formal procedure results

- Preserve formatter provenance on real procedure results when direct calls
  carry value or name array formals through the existing typed descriptor ABI.
  No dynamic array closure or thunk representation is introduced.
