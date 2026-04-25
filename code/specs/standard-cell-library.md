# Standard Cell Library

## Overview

A standard cell library is the catalog of building blocks the ASIC backend places on the die. Each cell is:
1. A **schematic** in transistors (NAND2 = 2 NMOS in series, 2 PMOS in parallel).
2. A **layout** as a polygon set in GDSII (the actual mask).
3. A **characterization** — for every input transition, every output load, at every PVT corner: how long does the cell take to switch? How much power does it consume? What's the input pin capacitance?

This spec defines:
1. The cell-list ABI shared with `tech-mapping.md` and `asic-placement.md`.
2. The Liberty (`.lib`) format we read from Sky130 and write for our characterized cells.
3. The characterization methodology — SPICE simulation per cell per corner per stimulus.
4. The mapping from generic HNL cells to standard cells.
5. Drive-strength selection.

Sky130 ships *pre-characterized* Liberty files. We use those directly when available. We also provide the infrastructure to re-characterize from SPICE, both as validation and for cells we may add (e.g., cells with custom drive strengths or transistor sizing experiments).

## Layer Position

```
sky130-pdk.md     spice-engine.md     mosfet-models.md
       │                │                    │
       └────────────────┴────────────────────┘
                        │
                        ▼
        ┌─────────────────────────────────────┐
        │  standard-cell-library.md           │  ◀── THIS SPEC
        │  (catalog + characterization)       │
        └─────────────────────────────────────┘
                        │
                        ▼
                tech-mapping.md
                asic-placement.md
                asic-routing.md
                gdsii-writer.md
```

## Concepts

### What a cell is

A cell has:

| Aspect | Source | Used by |
|---|---|---|
| **Function** | Boolean expression: `Y = !(A & B)` | tech-mapping (pattern recognition) |
| **Schematic** | SPICE subcircuit: 2 NMOS + 2 PMOS | spice-engine (characterization) |
| **Layout** | GDS polygons: poly, diff, M1 traces | gdsii-writer, drc-lvs |
| **Abstract** | LEF: pin locations, obstructions, footprint | lef-def, asic-placement, asic-routing |
| **Characterization** | Liberty: timing/power per (input slew, output load, corner) | static-timing, asic-placement, asic-routing |
| **Behavioral** | Verilog model | hdl-elaboration, hardware-vm |

### The Liberty (`.lib`) format

Liberty is the ubiquitous timing/power format. Industrial Liberty files are huge (50+ MB per corner for a real PDK). The format:

```liberty
library (sky130_fd_sc_hd__tt_025C_1v80) {
  technology (cmos);
  delay_model : table_lookup;
  voltage_unit : "1V";
  capacitive_load_unit (1, ff);
  time_unit : "1ns";
  
  operating_conditions ("tt_025C_1v80") {
    voltage : 1.8; temperature : 25; process : 1;
  }
  default_operating_conditions : tt_025C_1v80;
  nom_voltage : 1.8;
  nom_temperature : 25;
  
  cell (sky130_fd_sc_hd__nand2_1) {
    area : 3.7536;
    cell_leakage_power : 0.001;
    
    pin (A) {
      direction : input;
      capacitance : 0.0036;
      max_transition : 0.5;
    }
    pin (B) {
      direction : input;
      capacitance : 0.0035;
    }
    pin (Y) {
      direction : output;
      function : "!(A * B)";
      
      timing () {
        related_pin : "A";
        timing_sense : negative_unate;
        cell_rise (delay_template_5x5) {
          index_1 ("0.01, 0.05, 0.10, 0.20, 0.50");
          index_2 ("0.01, 0.05, 0.10, 0.20, 0.50");
          values (
            "0.058, 0.075, 0.108, 0.190, 0.401",
            "0.062, 0.080, 0.113, 0.196, 0.408",
            "0.069, 0.087, 0.120, 0.204, 0.417",
            "0.085, 0.103, 0.137, 0.221, 0.436",
            "0.124, 0.143, 0.179, 0.265, 0.486"
          );
        }
        cell_fall (delay_template_5x5) { ... }
        rise_transition (...) { ... }
        fall_transition (...) { ... }
      }
      
      timing () {
        related_pin : "B"; ...
      }
    }
  }
  
  cell (sky130_fd_sc_hd__inv_1) { ... }
  ... ~250 cells total ...
}
```

The 5x5 lookup table indexed by `(input_slew, output_load)` returns delay or transition time. Linear interpolation between table points; extrapolation beyond is allowed but flagged.

### Characterization methodology

For each cell × each corner × each timing arc × each (input_slew, output_load) grid point:

1. **Setup**: build the cell schematic in `spice-engine.md`. Drive the input pin under test with a step transition having the chosen slew rate. Hold all other inputs at their non-controlling value (e.g., for NAND2, hold the *other* input at 1 to enable propagation through the input under test). Connect the output to a capacitive load matching the chosen value.
2. **Run transient**: simulate for enough time to capture the output transition.
3. **Measure**: compute delay (50% input crossing → 50% output crossing) and transition time (10% to 90% of output range).
4. **Record**: emit one entry in the `cell_rise` / `cell_fall` / `rise_transition` / `fall_transition` table.
5. **Repeat** for the (slew × load) grid (typically 5×5 = 25 simulations per arc).

For an N-input cell, there are N timing arcs (one per input). Plus a leakage-power simulation. Plus dynamic-power.

Total simulations: ~30 cells × 5 corners × ~3 arcs × 26 grid points = ~12,000 SPICE runs. Each ~10-100 ms. Total ~10-30 minutes on a modern laptop. Larger libraries (full Sky130, ~250 cells) are ~10× more.

### Drive-strength selection

Cells with multiple drive strengths (`inv_1` through `inv_8`) are nominally equivalent in function but differ in transistor sizing (W). Larger drive = faster switching of large loads but more area and input capacitance.

**Drive-strength sizing** is a post-mapping refinement:
1. After tech mapping, every cell is at minimum drive (`_1`).
2. Walk the netlist; for each cell, estimate its actual output load (sum of input pin capacitances of all sinks + estimated wire capacitance).
3. Pick the smallest drive that satisfies the timing budget for the cell's slack.

For initial mapping (where slack is unknown), default `_1`. Re-size after first place-and-route iteration.

### Inverted vs non-inverted output cells

A subtle reality: `nand2_1` is faster and smaller than `and2_1` (which is just `nand2 + inv`). Synthesis often produces ANDs and ORs; tech mapping must convert these to NAND/NOR with inverted contexts when beneficial.

This is **bubble-pushing**: a `NOT` followed by an `AND` is the same as a `NAND` followed by a `NOT`; pushing bubbles through the netlist often eliminates them. Tech mapping does this automatically.

## Public API

```python
from dataclasses import dataclass, field
from enum import Enum


@dataclass(frozen=True)
class TimingArc:
    related_pin: str             # e.g. "A" — the input being toggled
    sense: str                   # "negative_unate" | "positive_unate" | "non_unate"
    cell_rise:    list[list[float]]      # 2-D table indexed [slew][load]
    cell_fall:    list[list[float]]
    rise_transition: list[list[float]]
    fall_transition: list[list[float]]
    slew_index:  list[float]
    load_index:  list[float]


@dataclass(frozen=True)
class PinTiming:
    direction: str
    capacitance: float           # input pin cap (ff for inputs); irrelevant for outputs
    max_transition: float = 0.5
    function: str | None = None  # boolean function for outputs


@dataclass(frozen=True)
class CharacterizedCell:
    name: str                    # e.g. "sky130_fd_sc_hd__nand2_1"
    short_name: str              # e.g. "nand2"
    drive_strength: int
    pins: dict[str, PinTiming]
    timing_arcs: list[TimingArc]
    leakage_power: float
    dynamic_power_per_event: dict[str, float]   # per output transition
    area: float                  # square micrometers


@dataclass(frozen=True)
class Corner:
    name: str                    # "tt_025C_1v80" etc.
    voltage: float
    temperature: float
    process_factor: float        # nominal 1.0


@dataclass
class Library:
    name: str
    corners: list[Corner]
    cells: dict[tuple[str, str], CharacterizedCell]   # (cell_name, corner) → cell
    
    @classmethod
    def from_liberty(cls, paths: list[Path]) -> "Library":
        """Load + parse Liberty files for a given technology, one per corner."""
        ...
    
    def to_liberty(self, path: Path, corner: str) -> None:
        """Write Liberty for a given corner."""
        ...
    
    def get_cell(self, name: str, corner: str = "tt_025C_1v80") -> CharacterizedCell: ...
    def list_drive_strengths(self, base: str) -> list[int]: ...


@dataclass
class Characterizer:
    """Drive SPICE characterization for cells without pre-shipped Liberty."""
    pdk: "Pdk"
    spice_engine: "SpiceEngine"
    
    def characterize_cell(self, cell_name: str, corner: Corner) -> CharacterizedCell: ...
    def characterize_library(self, cell_names: list[str], corners: list[Corner]) -> Library: ...
```

## Worked Example 1 — Loading Sky130 teaching subset

```python
from sky130_pdk import load_sky130, PdkProfile
from standard_cell_library import Library

pdk = load_sky130("/path/to/sky130A", profile=PdkProfile.TEACHING)
lib = Library.from_liberty(pdk.liberty_paths())

nand2 = lib.get_cell("sky130_fd_sc_hd__nand2_1", corner="tt_025C_1v80")
print(nand2.area)                                    # 3.7536 µm²
print(nand2.pins["A"].capacitance)                   # ~0.004 ff
print(nand2.timing_arcs[0].cell_rise[0][0])          # delay at min(slew, load) ~58 ps
```

## Worked Example 2 — Characterizing a custom cell

Suppose we add a custom `nand2_X3` cell (drive strength 3 — between Sky130's `_2` and `_4`) for a teaching exercise. We design the schematic + layout, then run characterization:

```python
from spice_engine import SpiceEngine
from standard_cell_library import Characterizer

custom_spice = '''
.subckt nand2_X3 A B Y VDD VSS
  Mp1 Y A VDD VDD pfet_01v8 W=1.5u L=130n
  Mp2 Y B VDD VDD pfet_01v8 W=1.5u L=130n
  Mn1 Y A n1  VSS nfet_01v8 W=0.6u L=130n
  Mn2 n1 B VSS VSS nfet_01v8 W=0.6u L=130n
.ends
'''

engine = SpiceEngine(...)
char = Characterizer(pdk=pdk, spice_engine=engine)

corners = [
    Corner("tt_025C_1v80", 1.80, 25.0, 1.0),
    Corner("ss_n40C_1v60", 1.60, -40.0, 0.85),
    Corner("ff_100C_1v95", 1.95, 100.0, 1.15),
]

# Characterize this cell across the 3 corners
for corner in corners:
    cell = char.characterize_cell("nand2_X3", corner)
    # ... save cell to library, write Liberty, etc.
```

The characterizer:
1. Parses the SPICE subckt, finds `A`, `B`, `Y`, `VDD`, `VSS`.
2. For each of 5 slews × 5 loads = 25 grid points:
   - Builds a SPICE deck: subckt instance + driving voltage source on `A` + `B` tied high + load capacitor on `Y`.
   - Runs `engine.transient()`.
   - Extracts delay and transition time.
3. Same for `B`-arc.
4. Power: drives random vectors; integrates I_VDD × V_DD over time.
5. Leakage: DC operating point with all inputs at non-switching values.
6. Returns a `CharacterizedCell`.

## Worked Example 3 — Library validation

For each cell in the teaching subset, we run our characterization and compare to Sky130's reference Liberty. The comparison metric: max relative error across the timing table.

| Cell | Max delay error (TT) | Max delay error (SS) | Max delay error (FF) |
|---|---|---|---|
| `inv_1` | 4% | 7% | 5% |
| `nand2_1` | 6% | 9% | 6% |
| `dfrtp_1` | 12% | 15% | 14% |
| ... | ... | ... | ... |

Typical errors are 5-10% for combinational cells; 10-15% for sequential cells (clock-to-Q timing is more sensitive to BSIM model fit). Acceptable for teaching; for tape-out, use Sky130's official Liberty.

## Edge Cases

| Scenario | Handling |
|---|---|
| Liberty file missing for a corner | Use closest corner; warn. |
| Cell in PDK but not in our cell list | Loaded; flagged as "not in teaching subset." Tech-mapping won't use it but it's available. |
| Slew/load index out of grid range | Linear extrapolation; warn if > 50% beyond max. |
| Cell with internal feedback (latch, FF) | Special-case characterization: clock arc, recovery/removal arcs. |
| Tristate cell (TBUF) | Output-enable arc characterized separately. |
| Cell with `function` containing operators we don't parse | Fall back to behavioral simulation; warn. |
| Power tables incomplete | Estimate dynamic power as `(C_load × V_DD²) / 2 × switching_activity`; warn. |
| Multiple drives of the same function (e.g., `nand2_1`, `_2`, `_4`) | Treated as separate `CharacterizedCell` entries with same `short_name`. |

## Test Strategy

### Unit (target 95%+)
- Liberty parser handles each token type.
- Timing-arc lookup: bilinear interpolation matches reference.
- Characterizer runs SPICE for `inv_1` and produces a delay value within 10% of reference.
- Drive-strength selection picks smallest cell satisfying load.

### Integration
- Load Sky130 teaching subset; verify cell count.
- Re-characterize 5 cells; verify within 15% of reference.
- Tech-mapping uses the library to map a 4-bit adder; result has expected cell mix.
- Static timing analysis on the mapped adder; report critical path.

### Property
- Monotonicity: larger drive strength → smaller delay (for the same cell function).
- Library is closed under tech-map: every input HNL cell type has a matching mapping target.

## Conformance Matrix

| Standard | Coverage |
|---|---|
| **Liberty** (Synopsys / OpenSourceLiberty) | Subset: cell, pin, timing (table_lookup model), power tables, area, leakage |
| **Liberty CCS** (current-source model) | Out of scope; future spec |
| **Liberty NLDM** (non-linear delay model) | Full |
| **Liberty composite cells** | Reading: yes; writing: limited |
| Sky130 cells | Teaching subset (~30) + full library (~250) supported |
| Custom cells via SPICE characterization | Full |

## Open Questions

1. **CCS vs NLDM** — CCS models are more accurate for advanced nodes; NLDM is sufficient for 130 nm. Recommendation: NLDM only for v1.
2. **Power format granularity** — `dynamic_power_per_event` per output transition is one model; full Liberty supports per-arc per-direction. Recommendation: use full per-arc/dir for tape-out flow.
3. **Variation models** (process Monte Carlo) — sample BSIM3v3 parameters per simulation? Future work.
4. **Layout-aware characterization** (extracted parasitics from cell layout) — Sky130 ships with `sky130_fd_sc_hd_*.spice` that has parasitics. Use them. Yes.

## Future Work

- CCS current-source models for high-accuracy timing.
- Full statistical (Monte Carlo) variation.
- Aging-aware characterization (NBTI/PBTI degradation over years).
- Cell layout generator (cell schematic → cell GDS) for parametric cells.
- ML-based characterization (train a network on a few simulations to predict the rest).
- Drive-strength interpolation (allow continuous sizing, not just discrete `_1`/`_2`/...).
