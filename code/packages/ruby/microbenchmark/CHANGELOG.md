# Changelog

- Exclude only StandardRB Layout/EndOfLine in the Windows build front to match the repository LF policy.

- Use the shared `arch` metadata field and omit private checksum metadata so
  native CLI reports replay in the other language implementations.
- Validate present workload metadata before projecting it from core report
  replay, including NUL and Unicode scalar boundaries.

## 0.1.0

- Add paired batch collection, ratio analysis, report replay and the indexed-array CLI.
- Preserve signed-64-bit integer fields in memory and through JSON.
