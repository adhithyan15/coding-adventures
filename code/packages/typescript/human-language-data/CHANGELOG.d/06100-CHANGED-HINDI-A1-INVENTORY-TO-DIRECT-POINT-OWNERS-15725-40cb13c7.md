### Changed — Hindi A1 inventory to direct point owners (#15725)

- Migrate all 282 Hindi A1 exam points from one 2,881-line aggregate to stable
  canonical point owners under `core/exam-inventory-hindi-a1.d/`.
- Preserve the exact public inventory and ordered scope manifest while rejecting
  aggregate resurrection through the existing HL37 loader boundary.
