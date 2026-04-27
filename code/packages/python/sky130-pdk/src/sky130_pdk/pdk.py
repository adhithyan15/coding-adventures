"""SkyWater Sky130 PDK metadata + loader.

The Sky130 PDK is the open-source 130 nm PDK from SkyWater, distributed by
Google. v0.1.0 of this package provides:
- Process metadata (V_DD, gate-oxide thickness, V_t, mobility, layer stack).
- Teaching subset of standard-cell names (~30 cells: NAND2, NOR2, INV, DFF,
  etc., each at multiple drive strengths).
- Layer/datatype map for GDSII conventions.
- Path-aware loader that points at a Sky130 install.

For real characterization data (Liberty .lib files, GDS layouts, SPICE
.model cards), the loader reads from the user's Sky130 install at the
provided path. v0.2.0 extends with full LEF parsing and BSIM3v3 .model
extraction.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path


class PdkProfile(Enum):
    TEACHING = "teaching"
    FULL = "full"


# ---------------------------------------------------------------------------
# Process metadata
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class ProcessMetadata:
    """Top-level Sky130 process parameters."""

    name: str = "sky130A"
    feature_size_nm: int = 130
    vdd_nominal: float = 1.8
    gate_oxide_thickness_nm: float = 4.2
    nmos_vt_typical: float = 0.42
    pmos_vt_typical: float = -0.51
    mun_cox: float = 220e-6  # NMOS μ_n × C_ox in A/V²
    mup_cox: float = 75e-6   # PMOS roughly 1/3 of NMOS
    metal_layers: int = 6
    cell_row_height_um: float = 2.72  # sky130_fd_sc_hd row height


# ---------------------------------------------------------------------------
# Layer/datatype map for GDSII
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class LayerInfo:
    name: str
    layer_number: int
    datatype: int
    purpose: str  # 'drawing', 'pin', 'label', etc.


# Sky130 GDS layer/datatype map (subset, simplified).
# Source: sky130_fd_pr/cells/sky130.layermap (open-source reference)
LAYER_MAP: dict[str, LayerInfo] = {
    "nwell.drawing":     LayerInfo("nwell", 64, 20, "drawing"),
    "pwell.drawing":     LayerInfo("pwell", 64, 16, "drawing"),
    "diff.drawing":      LayerInfo("diff", 65, 20, "drawing"),
    "tap.drawing":       LayerInfo("tap", 65, 44, "drawing"),
    "poly.drawing":      LayerInfo("poly", 66, 20, "drawing"),
    "licon1.drawing":    LayerInfo("licon1", 66, 44, "drawing"),
    "li1.drawing":       LayerInfo("li1", 67, 20, "drawing"),
    "li1.pin":           LayerInfo("li1", 67, 16, "pin"),
    "mcon.drawing":      LayerInfo("mcon", 67, 44, "drawing"),
    "met1.drawing":      LayerInfo("met1", 68, 20, "drawing"),
    "met1.pin":          LayerInfo("met1", 68, 16, "pin"),
    "via.drawing":       LayerInfo("via", 68, 44, "drawing"),
    "met2.drawing":      LayerInfo("met2", 69, 20, "drawing"),
    "met2.pin":          LayerInfo("met2", 69, 16, "pin"),
    "via2.drawing":      LayerInfo("via2", 69, 44, "drawing"),
    "met3.drawing":      LayerInfo("met3", 70, 20, "drawing"),
    "met3.pin":          LayerInfo("met3", 70, 16, "pin"),
    "via3.drawing":      LayerInfo("via3", 70, 44, "drawing"),
    "met4.drawing":      LayerInfo("met4", 71, 20, "drawing"),
    "met4.pin":          LayerInfo("met4", 71, 16, "pin"),
    "via4.drawing":      LayerInfo("via4", 71, 44, "drawing"),
    "met5.drawing":      LayerInfo("met5", 72, 20, "drawing"),
    "met5.pin":          LayerInfo("met5", 72, 16, "pin"),
}


# ---------------------------------------------------------------------------
# Cell list (teaching subset)
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class CellInfo:
    name: str
    function: str  # boolean expression, e.g. "Y = !(A * B)"
    drive_strength: int
    height_tracks: int = 9  # sky130_fd_sc_hd is 9 tracks tall


# Teaching subset: ~30 cells covering everything needed for a 4-bit adder
# smoke test through tape-out. Cell name format:
# sky130_fd_sc_hd__<function>_<drive>
TEACHING_CELLS: dict[str, CellInfo] = {
    "sky130_fd_sc_hd__inv_1":     CellInfo("sky130_fd_sc_hd__inv_1",     "Y = !A", 1),
    "sky130_fd_sc_hd__inv_2":     CellInfo("sky130_fd_sc_hd__inv_2",     "Y = !A", 2),
    "sky130_fd_sc_hd__inv_4":     CellInfo("sky130_fd_sc_hd__inv_4",     "Y = !A", 4),
    "sky130_fd_sc_hd__inv_8":     CellInfo("sky130_fd_sc_hd__inv_8",     "Y = !A", 8),
    "sky130_fd_sc_hd__buf_1":     CellInfo("sky130_fd_sc_hd__buf_1",     "X = A", 1),
    "sky130_fd_sc_hd__buf_2":     CellInfo("sky130_fd_sc_hd__buf_2",     "X = A", 2),
    "sky130_fd_sc_hd__buf_4":     CellInfo("sky130_fd_sc_hd__buf_4",     "X = A", 4),
    "sky130_fd_sc_hd__buf_8":     CellInfo("sky130_fd_sc_hd__buf_8",     "X = A", 8),
    "sky130_fd_sc_hd__nand2_1":   CellInfo("sky130_fd_sc_hd__nand2_1",   "Y = !(A*B)", 1),
    "sky130_fd_sc_hd__nand2_2":   CellInfo("sky130_fd_sc_hd__nand2_2",   "Y = !(A*B)", 2),
    "sky130_fd_sc_hd__nand3_1":   CellInfo("sky130_fd_sc_hd__nand3_1",   "Y = !(A*B*C)", 1),
    "sky130_fd_sc_hd__nor2_1":    CellInfo("sky130_fd_sc_hd__nor2_1",    "Y = !(A+B)", 1),
    "sky130_fd_sc_hd__nor2_2":    CellInfo("sky130_fd_sc_hd__nor2_2",    "Y = !(A+B)", 2),
    "sky130_fd_sc_hd__nor3_1":    CellInfo("sky130_fd_sc_hd__nor3_1",    "Y = !(A+B+C)", 1),
    "sky130_fd_sc_hd__and2_1":    CellInfo("sky130_fd_sc_hd__and2_1",    "X = A*B", 1),
    "sky130_fd_sc_hd__and2_2":    CellInfo("sky130_fd_sc_hd__and2_2",    "X = A*B", 2),
    "sky130_fd_sc_hd__or2_1":     CellInfo("sky130_fd_sc_hd__or2_1",     "X = A+B", 1),
    "sky130_fd_sc_hd__or2_2":     CellInfo("sky130_fd_sc_hd__or2_2",     "X = A+B", 2),
    "sky130_fd_sc_hd__xor2_1":    CellInfo("sky130_fd_sc_hd__xor2_1",    "X = A^B", 1),
    "sky130_fd_sc_hd__xnor2_1":   CellInfo("sky130_fd_sc_hd__xnor2_1",   "Y = !(A^B)", 1),
    "sky130_fd_sc_hd__mux2_1":    CellInfo("sky130_fd_sc_hd__mux2_1",    "X = S?A1:A0", 1),
    "sky130_fd_sc_hd__aoi21_1":   CellInfo("sky130_fd_sc_hd__aoi21_1",   "Y = !(A1*A2 + B1)", 1),
    "sky130_fd_sc_hd__oai21_1":   CellInfo("sky130_fd_sc_hd__oai21_1",   "Y = !((A1+A2)*B1)", 1),
    "sky130_fd_sc_hd__dfxtp_1":   CellInfo("sky130_fd_sc_hd__dfxtp_1",   "Q = D @ posedge CLK", 1),
    "sky130_fd_sc_hd__dfrtp_1":   CellInfo("sky130_fd_sc_hd__dfrtp_1",   "Q = D @ posedge CLK, async R", 1),
    "sky130_fd_sc_hd__dfstp_1":   CellInfo("sky130_fd_sc_hd__dfstp_1",   "Q = D @ posedge CLK, async S", 1),
    "sky130_fd_sc_hd__dlxtp_1":   CellInfo("sky130_fd_sc_hd__dlxtp_1",   "Q = D when GATE=1", 1),
    "sky130_fd_sc_hd__clkbuf_1":  CellInfo("sky130_fd_sc_hd__clkbuf_1",  "X = A (clock buf)", 1),
    "sky130_fd_sc_hd__clkbuf_4":  CellInfo("sky130_fd_sc_hd__clkbuf_4",  "X = A (clock buf)", 4),
    "sky130_fd_sc_hd__conb_1":    CellInfo("sky130_fd_sc_hd__conb_1",    "LO = 0; HI = 1", 1),
    "sky130_fd_sc_hd__tap_1":     CellInfo("sky130_fd_sc_hd__tap_1",     "(well/substrate tap)", 0),
    "sky130_fd_sc_hd__decap_3":   CellInfo("sky130_fd_sc_hd__decap_3",   "(decap)", 3),
    "sky130_fd_sc_hd__fill_1":    CellInfo("sky130_fd_sc_hd__fill_1",    "(filler)", 0),
}


# ---------------------------------------------------------------------------
# Pdk loader
# ---------------------------------------------------------------------------


@dataclass
class Pdk:
    """A loaded Sky130 PDK reference."""

    profile: PdkProfile
    root: Path | None  # None for in-memory teaching subset
    process: ProcessMetadata = field(default_factory=ProcessMetadata)
    cells: dict[str, CellInfo] = field(default_factory=dict)
    layers: dict[str, LayerInfo] = field(default_factory=dict)

    @property
    def cell_names(self) -> list[str]:
        return sorted(self.cells.keys())

    def get_cell(self, name: str) -> CellInfo:
        if name not in self.cells:
            raise KeyError(f"cell {name!r} not in PDK")
        return self.cells[name]

    def get_layer(self, key: str) -> LayerInfo:
        """Look up a layer by 'name.purpose' (e.g., 'met1.drawing')."""
        if key not in self.layers:
            raise KeyError(f"layer {key!r} not in PDK")
        return self.layers[key]


def load_sky130(
    *,
    root: Path | str | None = None,
    profile: PdkProfile = PdkProfile.TEACHING,
) -> Pdk:
    """Load the Sky130 PDK.

    With profile=TEACHING (default), returns an in-memory PDK with the
    teaching cell subset and standard layer map. No filesystem access.

    With profile=FULL and root=<sky130A install path>, loads cell metadata
    by walking the install. v0.1.0 only validates the path exists; full LEF
    parsing comes in v0.2.0.
    """
    pdk = Pdk(
        profile=profile,
        root=Path(root) if root is not None else None,
        cells=dict(TEACHING_CELLS),
        layers=dict(LAYER_MAP),
    )
    if profile == PdkProfile.FULL:
        if root is None:
            raise ValueError("profile=FULL requires root path to a Sky130 install")
        root_path = Path(root)
        if not root_path.exists():
            raise FileNotFoundError(f"Sky130 install not found: {root_path}")
        # v0.2.0: walk root_path/libs.ref/sky130_fd_sc_hd/lef/ and parse cells.
    return pdk
