### Changed - retired tracked level snapshot rollups

- Removed the 23 deterministic per-language level snapshot files and their write/check
  CLI. Level coverage now stays source-derived in memory, with exact registered-track,
  histogram, unmapped, reach, and corpus arithmetic closure plus a resurrection guard.
