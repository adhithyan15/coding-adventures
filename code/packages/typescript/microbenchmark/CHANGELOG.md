# Changelog

- Reject inherited Object member names as CLI options; only explicitly
  declared flags may update benchmark configuration.
- Validate present workload metadata, including NUL and Unicode scalar
  boundaries, before projecting it from core report replay.

## 0.1.0

- Add paired observation analysis, deterministic collection, report replay, and indexed-array driver.
- Preserve signed-64-bit integer values through lossless JSON and BigInt collection.
