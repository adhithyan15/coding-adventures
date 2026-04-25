# sky130-pdk

SkyWater Sky130 PDK metadata loader. Provides process parameters, layer/datatype map for GDSII, and a teaching subset of standard cells. **Zero deps.**

See [`code/specs/sky130-pdk.md`](../../../specs/sky130-pdk.md).

## Quick start

```python
from sky130_pdk import load_sky130, PdkProfile

# Teaching profile: in-memory, no PDK install required
pdk = load_sky130(profile=PdkProfile.TEACHING)

print(pdk.process.feature_size_nm)         # 130
print(pdk.process.vdd_nominal)             # 1.8

print(len(pdk.cells))                       # ~33 teaching cells

cell = pdk.get_cell("sky130_fd_sc_hd__nand2_1")
print(cell.function)                        # "Y = !(A*B)"
print(cell.drive_strength)                  # 1

layer = pdk.get_layer("met1.drawing")
print(layer.layer_number, layer.datatype)  # 68, 20

# Full profile: requires the actual Sky130 install on disk
# pdk = load_sky130(root="/path/to/sky130A", profile=PdkProfile.FULL)
```

## v0.1.0 scope

- `PdkProfile`: TEACHING (in-memory subset) or FULL (requires install path).
- `ProcessMetadata`: 130nm, 1.8V V_DD, 4.2nm gate oxide, NMOS V_t ~0.42V, PMOS V_t ~-0.51V, μn·Cox ~220e-6, 6 metal layers, 2.72µm cell row height (sky130_fd_sc_hd).
- `LAYER_MAP`: 24 entries covering the GDSII layer/datatype pairs for nwell, pwell, diff, tap, poly, licon1, li1, mcon, met1-5, vias.
- `TEACHING_CELLS`: 33 entries covering INV/BUF (4 drive strengths each), NAND2/3, NOR2/3, AND2, OR2, XOR2, XNOR2, MUX2, AOI21, OAI21, DFXTP/DFRTP/DFSTP, DLXTP, CLKBUF, CONB, TAP, DECAP, FILL.
- `Pdk` accessor: `cell_names`, `get_cell(name)`, `get_layer('met1.drawing')`.

## Out of scope (v0.2.0)

- Full LEF parsing (per-cell pins, obstructions, sizes).
- BSIM3v3 .model card extraction.
- Liberty .lib characterization data loading.
- GDS layout file parsing.
- Other-process variants (sky130_fd_sc_hs, sky130_fd_sc_lp, sky130_fd_sc_ms, sky130_fd_sc_hdll, sky130_fd_sc_hvl).
- 5V I/O cells.
- Memory macros via OpenRAM.

MIT.
