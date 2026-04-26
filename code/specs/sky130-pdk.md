# Sky130 PDK Integration

## Overview

The **SkyWater 130 nm Open Source PDK** (Sky130) is the first fully open-source process design kit for a real foundry process. Released under Apache 2.0, it provides the technology files needed to design ASICs that can actually be manufactured — through Efabless's chipIgnite shuttle program, real silicon arrives in your hands. This spec defines how the silicon stack integrates with Sky130: which files we consume, how we interpret them, and what subset we support.

The user's commitment to **full Sky130 compatibility** raises this from "teaching PDK" to "real path to silicon." Every cell we characterize, every transistor parameter we use, every metal layer we route on, every design rule we check — must match the reality of the SkyWater fab. Designs that pass our DRC must (in principle) pass SkyWater's DRC. GDSII we emit must be acceptable on the chipIgnite shuttle.

What we provide:
1. A **PDK loader** that reads Sky130's open-source files (SPICE models, LEF, GDS, Liberty, technology files).
2. A **canonical mapping** of Sky130 technology objects to our internal representations (HNL cell types, MOSFET model parameters, layer stack).
3. **Tool integration** points for `standard-cell-library.md` (cell list + characterization), `lef-def.md` (LEF/DEF files), `gdsii-writer.md` (GDS layer/datatype map), `drc-lvs.md` (DRC rules), `tape-out.md` (chipIgnite-compatible bundle).
4. **Two PDK profiles**:
   - **Teaching subset**: ~30 cells, 2 metal layers, simplified DRC. Fast to characterize, complete enough for the 4-bit adder smoke test. Default for learning paths.
   - **Full Sky130**: ~250+ cells, 5 metal layers, full DRC. Real tape-out target. Used when the user is ready to ship.

The two profiles share the same loader infrastructure and conformance tests. Switching between them is a configuration knob.

## Layer Position

```
device-physics.md, mosfet-models.md, fab-process-simulation.md, spice-engine.md
                                          │
                                          ▼
                            ┌───────────────────────────────┐
                            │  sky130-pdk.md                │  ◀── THIS SPEC
                            │  (PDK files → internal types) │
                            └───────────────────────────────┘
                                          │
        ┌────────────────────┬────────────┼──────────┬─────────────────┐
        ▼                    ▼            ▼          ▼                 ▼
standard-cell-library  lef-def.md   gdsii-writer  drc-lvs        tape-out
                                                                  (chipIgnite)
```

## Sky130 — what's in the PDK

The open-source distribution is structured as a tree of files. Top-level directories:

```
sky130A/                              # the digital-friendly process variant
├── libs.ref/
│   ├── sky130_fd_sc_hd/              # high-density standard cell library
│   │   ├── lef/                      # LEF (cell abstracts)
│   │   │   ├── sky130_fd_sc_hd.lef   # tech LEF
│   │   │   └── sky130_fd_sc_hd.lef   # cells LEF
│   │   ├── gds/                      # GDS layouts of each cell
│   │   ├── lib/                      # Liberty timing/power
│   │   │   ├── sky130_fd_sc_hd__tt_025C_1v80.lib    # TT corner
│   │   │   ├── sky130_fd_sc_hd__ss_n40C_1v60.lib    # SS corner
│   │   │   └── sky130_fd_sc_hd__ff_100C_1v95.lib    # FF corner
│   │   ├── spice/                    # SPICE subcircuits per cell
│   │   ├── verilog/                  # behavioral Verilog per cell
│   │   └── techfile/                 # tech rules
│   ├── sky130_fd_sc_hs/              # high-speed (analog-friendly) library
│   ├── sky130_fd_sc_lp/              # low-power
│   ├── sky130_fd_sc_ms/              # medium-speed
│   ├── sky130_fd_sc_hdll/            # high-density low-leakage
│   └── sky130_fd_sc_hvl/             # high-voltage
├── libs.tech/                         # technology files
│   ├── magic/
│   ├── klayout/
│   ├── ngspice/
│   └── netgen/
└── libs.priv/                         # (in chipIgnite, sealed PDK files)
```

We focus on `sky130_fd_sc_hd` (high-density) for digital design. The other libraries are out of scope for v1 (used for analog and special-purpose designs).

### File formats consumed

| Format | What it provides | Parser spec |
|---|---|---|
| **`.lef`** (Library Exchange Format) | Cell abstracts: pin locations, footprint, obstructions; tech rules: layer thicknesses, vias, design rules | `lef-def.md` |
| **`.lib`** (Liberty) | Timing arcs, power, capacitance — characterized at PVT corners | `standard-cell-library.md` |
| **`.gds`** (GDSII Stream) | Polygon-level layout of each cell | `gdsii-writer.md` (consume side) |
| **`.spice`** (SPICE subcircuit) | Cell schematic in transistors | `spice-engine.md` |
| **`.v`** (Verilog) | Behavioral simulation model | `hdl-elaboration.md` consumes via the parser |
| **`.tlef`** (technology LEF) | Layer stack, via definitions | `lef-def.md` |
| **`.tech`** (Magic technology file) | Magic-format rules; optional to consume; we use LEF instead | — |

## Process metadata

The Sky130 process: 130 nm CMOS, twin-well, 5 metal layers (li1, met1, met2, met3, met4, met5 — actually 6 if local-interconnect is counted), 1.8 V nominal V_DD. Some salient parameters:

| Parameter | Value |
|---|---|
| Minimum gate length (drawn) | 150 nm (effective ~130 nm after process bias) |
| V_DD nominal | 1.8 V |
| Gate oxide thickness | ~4.2 nm (1.8 V devices) |
| NMOS V_t (typical) | ~0.42 V |
| PMOS V_t (typical) | ~-0.51 V |
| NMOS μ_n × C_ox | ~220 µA/V² |
| Metal layers | 6 (li1, met1, met2, met3, met4, met5) |
| Min metal pitch | 0.34 µm (met1) up to 4.0 µm (met5) |
| Cell row height | 2.72 µm (sky130_fd_sc_hd) |

PVT corners we characterize:
- TT (typical-typical, 25°C, 1.80 V)
- SS (slow-slow, -40°C, 1.60 V)
- FF (fast-fast, 100°C, 1.95 V)
- SF (slow-NMOS, fast-PMOS, 25°C, 1.80 V)
- FS (fast-NMOS, slow-PMOS, 25°C, 1.80 V)

(Sky130 ships pre-characterized Liberty for these corners; for our internal characterization runs, we re-characterize using `spice-engine.md` + `mosfet-models.md` BSIM3v3 to validate.)

## Cell list (teaching subset, ~30 cells)

The teaching subset is enough to map the 4-bit adder, ALU, and small CPU designs. Sufficient for end-to-end demonstration without the months of characterization work for the full library.

| Cell | Function | Drive strengths |
|---|---|---|
| `inv` | Inverter | _1, _2, _4, _8 |
| `buf` | Buffer | _1, _2, _4, _8 |
| `nand2` | NAND2 | _1, _2, _4 |
| `nand3` | NAND3 | _1, _2 |
| `nor2` | NOR2 | _1, _2 |
| `nor3` | NOR3 | _1 |
| `and2` | AND2 (= NAND + INV) | _1, _2 |
| `or2` | OR2 | _1, _2 |
| `xor2` | XOR2 | _1 |
| `xnor2` | XNOR2 | _1 |
| `mux2` | 2:1 mux | _1, _2 |
| `aoi21` | (A·B) + C inverted | _1 |
| `aoi22` | (A·B) + (C·D) inverted | _1 |
| `oai21` | (A+B) · C inverted | _1 |
| `dfxtp` | D-flip-flop, async reset (active low) | _1, _2 |
| `dfrtp` | D-flip-flop, async reset (active high) | _1, _2 |
| `dfsrtp` | D-flip-flop, async set + reset | _1 |
| `dlxtp` | D-latch | _1 |
| `clkbuf` | Clock buffer | _1, _2, _4, _8, _16 |
| `clkinv` | Clock inverter | _1, _2, _4, _8 |
| `tap` | Well/substrate tap (no logic) | — |
| `decap` | Decoupling capacitor | _3, _12 |
| `fill` | Filler (no logic) | _1, _2, _4 |
| `conb` | Constant 0/1 generator | _1 |
| `tinv` | Tristate inverter | _1 |

Each cell has a Sky130 name like `sky130_fd_sc_hd__nand2_1`. The teaching subset uses short aliases (`nand2_X1`).

## Full Sky130 cell list (~250+)

The full `sky130_fd_sc_hd` library has cells for AOI/OAI gates with 3-4 inputs, scan flip-flops, multiplexers up to 4-1, full and half adders, and buffers/inverters at every drive strength up to 16x. The full library list is shipped with the PDK; we do not enumerate it here — the loader reads the LEF directory.

## Public API

```python
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path


class PdkProfile(Enum):
    TEACHING = "teaching"
    FULL     = "full"


@dataclass
class Pdk:
    profile: PdkProfile
    root: Path
    library_name: str    # e.g. "sky130_fd_sc_hd"
    
    # Loaded artifacts
    tech_lef: "TechLef"
    cells_lef: dict[str, "CellLef"]
    liberty: dict[str, "LibertyCell"]   # per corner, merged
    spice_subckts: dict[str, "SpiceSubckt"]
    verilog_models: dict[str, "VerilogModel"]
    gds_cells: dict[str, "GdsCell"]
    
    @classmethod
    def load(cls, root: Path, profile: PdkProfile = PdkProfile.TEACHING) -> "Pdk":
        ...
    
    def cell_names(self) -> list[str]: ...
    def get_cell(self, name: str) -> "PdkCell": ...
    def corners(self) -> list[str]: ...
    
    def to_hnl_cell_types(self) -> dict[str, "CellTypeSig"]:
        """For tech-mapping consumers."""
        ...


@dataclass
class PdkCell:
    """Aggregate view of a Sky130 cell across all the file formats."""
    name: str
    function: str              # boolean expression like "Y = !A | (B & C)"
    pins: list["Pin"]
    drive_strength: int
    height_tracks: int          # 9 for sky130_fd_sc_hd
    width: float                # in micrometers
    area: float                 # square micrometers
    
    timing: "LibertyTiming"
    power: "LibertyPower"
    spice: "SpiceSubckt"
    gds: "GdsCell"
    abstract: "CellLef"
    behavioral_verilog: "VerilogModel"


@dataclass
class TechLef:
    units: float                # internal units to micrometers
    layers: list["LayerDef"]
    vias: list["ViaDef"]
    via_rules: list["ViaRule"]
    sites: list["SiteDef"]
    rules: dict[str, float]    # min-width, min-spacing, etc.


@dataclass
class LayerDef:
    name: str             # e.g. "met1"
    purpose: str          # e.g. "ROUTING"
    direction: str        # "HORIZONTAL" | "VERTICAL"
    pitch: float
    width_min: float
    spacing_min: float


def load_sky130(root: Path, profile: PdkProfile = PdkProfile.TEACHING) -> Pdk:
    return Pdk.load(root, profile)
```

## Mapping to internal types

### Cell types → HNL builtins

The teaching subset maps to HNL primitive cell types via a fixed table:

| Sky130 cell | HNL cell type |
|---|---|
| `nand2_1`/`_2`/`_4` | `NAND2` (with drive metadata) |
| `nor2_1`/`_2` | `NOR2` |
| `inv_1`/`_2`/`_4`/`_8` | `NOT` |
| `buf_1`/...`_16` | `BUF` |
| `xor2_1` | `XOR2` |
| `dfxtp_1` | `DFF_R` (async reset, active low — inverted at use site) |
| `dfrtp_1` | `DFF_R` (async reset, active high) |
| `mux2_1` | `MUX2` |
| `aoi21_1` | composite — synthesized as `OR2(AND2(A,B), C)` then NOTed; tech-mapping recognizes via pattern matching |

### Layer stack → HNL parameters

The Sky130 layer stack feeds `lef-def.md` (consumed by `asic-routing.md`):

| Layer | Direction | Pitch | Used for |
|---|---|---|---|
| li1 | (any) | 0.34 µm | local interconnect (cell internal) |
| met1 | horizontal | 0.34 µm | intra-cell + short routing |
| met2 | vertical | 0.46 µm | row routing |
| met3 | horizontal | 0.68 µm | block routing |
| met4 | vertical | 0.92 µm | global routing |
| met5 | horizontal | 1.60 µm | power grid + global routing |

(Numbers are illustrative; loaded from the actual LEF.)

### MOSFET parameters → mosfet-models

Sky130 ships BSIM3v3 model cards for the NFET/PFET. Our loader reads these and constructs `BSIM3v3Params` instances per corner.

Key cards:
- `sky130_fd_pr__nfet_01v8` — NMOS, 1.8 V devices.
- `sky130_fd_pr__pfet_01v8` — PMOS, 1.8 V devices.
- `sky130_fd_pr__nfet_g5v0d10v5` — NMOS, 5 V devices (for I/O).
- `sky130_fd_pr__pfet_g5v0d10v5` — PMOS, 5 V devices.

We use the 1.8 V cards for the digital core; 5 V for I/O pads (out of scope for the teaching subset; v1 doesn't drive 5 V signals).

## Worked Example 1 — Loading the teaching PDK

```python
from sky130_pdk import load_sky130, PdkProfile

pdk = load_sky130(Path("~/skywater-pdk/sky130A"), profile=PdkProfile.TEACHING)

print(pdk.cell_names())           # ['inv_1', 'inv_2', ..., 'nand2_1', ...]
print(len(pdk.cell_names()))      # ~30

nand2 = pdk.get_cell("nand2_1")
print(nand2.height_tracks)        # 9
print(nand2.width)                # ~1.04 µm
print(nand2.timing.input_pin_capacitance("A"))  # ~4 fF
```

## Worked Example 2 — Tech mapping the 4-bit adder using Sky130

`tech-mapping.md` consumes `pdk.to_hnl_cell_types()` and the generic 20-cell HNL adder. Mapping:

| Generic cell | Sky130 cell | Drive |
|---|---|---|
| 8 × XOR2 → | 8 × `xor2_1` | _1 |
| 8 × AND2 → | 8 × `and2_1` | _1 |
| 4 × OR2 → | 4 × `or2_1` | _1 |

Result: 20-cell stdcell HNL using Sky130 cells. Each cell's footprint and pin locations come from `nand2.abstract` (LEF data); `asic-floorplan.md` and `asic-placement.md` use these to determine where in the die area each instance lands.

## Worked Example 3 — Tape-out validation

After GDSII is emitted, we run:
1. **DRC**: against Sky130 rules (~100 design rules: minimum width, minimum spacing, well enclosure, etc.).
2. **LVS**: extract netlist from GDS using parasitic-aware extraction; compare to gate-level netlist via graph isomorphism.
3. **Antenna**: check that polysilicon edges aren't connected to large metal antennas during fabrication.
4. **Density**: each layer has minimum/maximum density requirements; we add fill cells if needed.

These checks must pass for chipIgnite acceptance. `drc-lvs.md` and `tape-out.md` orchestrate them.

## Edge Cases

| Scenario | Handling |
|---|---|
| PDK file missing or corrupt | Loader reports specific file; refuses to construct Pdk. |
| Cell present in LEF but not in Liberty (or vice versa) | Warn; cell unusable for timing-aware flows. |
| Liberty corner not present | Use closest available; warn. |
| Cell name doesn't match Sky130 convention | Reject. |
| Teaching subset references a cell not in full library | Compile-time error (loader checks). |
| GDS layer not in tech LEF | Warn; ignore. |
| Different versions of Sky130 (sky130A vs sky130B) | Loader detects from path; warns if mixed. |
| chipIgnite shuttle deadlines | Out of scope for tooling; mention in tape-out. |

## Test Strategy

### Unit (target 95%+)
- Loader parses each Sky130 file format without error.
- Cell-name → HNL cell-type mapping is correct for every teaching cell.
- BSIM3v3 parameter loading produces reasonable V_t and Idsat for NMOS/PMOS.
- Liberty parser extracts timing arcs correctly for `inv_1`, `nand2_1`, `dfrtp_1`.
- LEF parser extracts pin positions and obstructions.

### Integration
- Full teaching subset loads in < 1 sec.
- Full Sky130 hd library loads in < 10 sec.
- Round-trip: `pdk.to_hnl_cell_types()` is consistent with characterization output from `standard-cell-library.md`.
- Re-characterize 5 cells using our SPICE; compare to Sky130 reference Liberty within 10%.
- 4-bit adder: synth → tech-map (Sky130 teaching) → place → route → GDS → DRC clean.

### Property
- Determinism: loading Sky130 twice gives identical Pdk.
- Idempotence: re-running characterization gives identical results.

## Conformance Matrix

| Sky130 artifact | Coverage |
|---|---|
| **sky130_fd_sc_hd**: LEF | Full subset for digital flow |
| **sky130_fd_sc_hd**: Liberty (TT/SS/FF + intermediate corners) | Full |
| **sky130_fd_sc_hd**: SPICE subcircuits | Full |
| **sky130_fd_sc_hd**: GDS | Full (consume side) |
| **sky130_fd_sc_hd**: behavioral Verilog | Full |
| **sky130_fd_sc_hs/lp/ms/hdll/hvl** | Out of scope; future spec |
| **sky130_fd_pr** primitives (transistor models) | Full for `nfet_01v8`, `pfet_01v8`; out of scope for 5 V devices |
| **Tech files** | LEF only (Magic / KLayout tech files reference future) |
| **Magic, KLayout, OpenLane integration** | Documented but optional; we do not depend on these tools at runtime |

## Open Questions

1. **PDK version pinning** — Sky130 evolves. Pin to a known commit hash or release tag? Recommendation: yes; default to a tested version, allow override.
2. **Mismatched re-characterization** — what if our SPICE characterization disagrees with Sky130's reference Liberty by > 10%? Recommendation: report the deviation; allow the user to choose which to trust.
3. **Memory macros** — Sky130 has SRAM compilers (OpenRAM). Use OpenRAM-generated macros or roll our own? Recommendation: integrate OpenRAM as a future spec.
4. **5 V I/O cells** — needed for actual chipIgnite tape-out (ESD pads). Future scope.

## Future Work

- Other Sky130 cell libraries (hs, lp, ms, hdll, hvl).
- 5 V I/O cells and ESD pad ring.
- Sky130 SRAM macros via OpenRAM.
- Sky130 analog blocks (op-amps, ADCs, bandgap references) — for mixed-signal tape-outs.
- Process variation modeling (Monte Carlo).
- Sky130-B variant.
- Cross-PDK abstraction (`pdk-abstraction.md`) that lets the same flow target Sky130, GF180MCU, ASAP7, etc.
- Aging models (NBTI/PBTI) for the Sky130 process.
