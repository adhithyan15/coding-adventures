# Changelog

## [0.1.0] — Unreleased

### Added
- `PdkProfile`: TEACHING (in-memory) or FULL (Sky130 install required).
- `ProcessMetadata`: Sky130 process parameters (130nm, 1.8V V_DD, gate-oxide, V_t, mobility, metal-layer count, cell-row height).
- `LayerInfo` + `LAYER_MAP`: 24 GDSII layer/datatype entries for nwell, pwell, diff, tap, poly, licon1, li1, mcon, met1-5, vias.
- `CellInfo` + `TEACHING_CELLS`: 33 cells from sky130_fd_sc_hd including INV/BUF (1/2/4/8 drive), NAND2/3, NOR2/3, AND2, OR2, XOR2, XNOR2, MUX2, AOI21, OAI21, DFXTP, DFRTP, DFSTP, DLXTP, CLKBUF, CONB, TAP, DECAP, FILL.
- `Pdk` dataclass with `get_cell()` and `get_layer()` accessors.
- `load_sky130(root=None, profile=PdkProfile.TEACHING)`: TEACHING returns in-memory PDK; FULL validates install path exists.

### Out of scope (v0.2.0)
- Full LEF parsing (cell pins, obstructions, exact sizes).
- BSIM3v3 .model card extraction.
- Liberty .lib loading.
- GDS file parsing.
- Other Sky130 cell variants (hs, lp, ms, hdll, hvl).
- 5V I/O cells, OpenRAM memory macros.
