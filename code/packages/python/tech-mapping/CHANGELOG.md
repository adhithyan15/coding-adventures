# Changelog

## [0.1.0] — Unreleased

### Added
- `DEFAULT_MAP`: generic HNL cell type -> Sky130-style stdcell, with per-pin remap dict (e.g. HNL `Y` -> stdcell `X` for combinational outputs).
- `TechMapper(cell_map)`: rule-based mapper.
- `map_to_stdcell(netlist)`: convenience entrypoint with default mapping.
- `MappingReport`: cells_before/after, unmapped cell types list, bubbles_canceled, aoi_oai_folded counters.
- `push_bubbles(netlist)`: optimization that cancels adjacent INV-INV pairs and rewires upstream A directly to downstream Y.
- Maps the full HNL primitive set (BUF, NOT, AND2-4, OR2-4, NAND2-4, NOR2-4, XOR2-3, XNOR2-3, MUX2, DFF and variants, DLATCH, TBUF, CONST_0/1).

### Out of scope (v0.2.0)
- AOI/OAI folding (needs DAG covering).
- Drive-strength selection (needs load estimation).
- Multi-target mapping for different constraints.
- Sequential cell variants (scan, clock-gating).
