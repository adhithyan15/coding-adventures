# Tape-Out

## Overview

Tape-out is the act of shipping the design files to the foundry. After GDSII is emitted, DRC and LVS pass, signoff timing is acceptable, and density requirements are met, the designer assembles a **tape-out bundle**: the GDSII plus all supporting documentation that the fab needs to manufacture the chip.

For our flow, the target is **Efabless chipIgnite** — a multi-project wafer (MPW) shuttle that accepts hobbyist and academic designs on the open-source Sky130 PDK. Real silicon, real cost (~$10K-50K depending on shuttle), real timeline (~6 months from submission to packaged chips). This spec defines the bundle that chipIgnite expects.

## Layer Position

```
gdsii-writer.md   drc-lvs.md   asic-routing.md (timing)
       │              │              │
       └──────────────┴──────────────┘
                      ▼
        ┌─────────────────────────────┐
        │  tape-out.md                 │  ◀── THIS SPEC
        │  (assemble shuttle bundle)   │
        └─────────────────────────────┘
                      │
                      ▼
         Efabless chipIgnite shuttle
                      │
                      ▼
                 Real silicon
```

## What's in a chipIgnite tape-out bundle

```
adder4_chipignite/
├── README.md                    # design description
├── adder4.gds                   # the layout
├── adder4.lef                   # cell-level abstract for the macro
├── adder4.def                   # placed + routed
├── adder4.v                     # behavioral Verilog (for scan/test)
├── adder4_drc_report.txt        # signoff DRC results
├── adder4_lvs_report.txt        # signoff LVS results
├── adder4_timing_report.txt     # static timing analysis (worst-case)
├── adder4_density_report.txt    # per-layer density distribution
├── adder4_layer_map.csv         # our layer numbers ↔ Sky130's
├── adder4_pad_locations.csv     # pad name, x, y, direction
├── adder4_ip_statement.md       # IP licensing statement (open / proprietary)
└── manifest.yaml                # bundle metadata; required by chipIgnite
```

### manifest.yaml

```yaml
project_name: adder4
designer: <name>
email: <email>
shuttle: chipignite
pdk: sky130A
pdk_version: <git-hash>
design_kind: standalone
top_module: adder4
target_chipignite_program: open_mpw   # vs paid_mpw
caravel_user_project: false
license: Apache-2.0
git_url: https://github.com/<user>/adder4-chipignite

clock:
  primary: clk
  frequency_mhz: 50

power:
  vdd_voltage: 1.8
  vdd_pads: [VDD_pad]
  vss_pads: [VSS_pad]

pads:
  - {name: a[0], dir: input,  x: 0,    y: 100}
  - {name: a[1], dir: input,  x: 0,    y: 200}
  # ...
  - {name: cout, dir: output, x: 1000, y: 100}

signoff:
  drc: clean
  lvs: clean
  antenna: clean
  density:
    met1: 32%
    met2: 28%
    met3: 18%
  timing:
    worst_setup_ns: 1.2
    worst_hold_ns: 0.05
```

## Caravel integration (advanced)

Most chipIgnite designs aren't standalone — they're **user-projects** plugged into the **Caravel** harness. Caravel is Efabless's integration chip: a RISC-V management core, a 32-bit Wishbone bus, configurable IO, and a fixed area for the user project. The user submits only the user-project area; Caravel surrounds it.

For our 4-bit adder smoke-test path, standalone is fine. For a more substantial design (e.g., the existing arm1 / intel4004 in the repo), Caravel integration makes more sense and shares the wafer with other designs (cheaper).

Caravel-specific files:
- `user_project_wrapper.v` (top module that integrates with Caravel's harness).
- `user_project_wrapper.gds` (must be exactly 2920 µm × 3520 µm).
- Wishbone or LA (logic analyzer) interfaces.

Out of scope for v1 detailed coverage; documented as future work.

## Public API

```python
from dataclasses import dataclass, field
from pathlib import Path
from enum import Enum


class Shuttle(Enum):
    CHIPIGNITE_OPEN_MPW = "chipignite_open_mpw"
    CHIPIGNITE_PAID_MPW = "chipignite_paid_mpw"


@dataclass
class TapeoutMetadata:
    project_name: str
    designer: str
    email: str
    shuttle: Shuttle
    pdk: str = "sky130A"
    pdk_version: str | None = None
    license: str = "Apache-2.0"
    top_module: str = ""
    git_url: str | None = None


@dataclass
class TapeoutBundle:
    metadata: TapeoutMetadata
    files: dict[str, Path]
    
    def write(self, output_dir: Path) -> None: ...
    def validate(self) -> "ValidationReport": ...


def assemble(
    metadata: TapeoutMetadata,
    gds: Path, lef: Path, def_file: Path, verilog: Path,
    drc_report: "DrcReport", lvs_report: "LvsReport",
    timing_report: dict, density_report: dict,
    output_dir: Path,
) -> TapeoutBundle: ...


def validate_for_chipignite(bundle: TapeoutBundle) -> "ValidationReport":
    """Check that the bundle meets chipIgnite acceptance criteria."""
    ...


@dataclass
class ValidationReport:
    passed: bool
    errors: list[str]
    warnings: list[str]
```

## Worked Example — 4-bit Adder tape-out (smoke test)

Bundle ready when:
1. ✅ GDS clean (DRC + LVS + antenna).
2. ✅ Density per layer within Sky130 ranges.
3. ✅ Timing: max delay < clock period (we use 50 MHz clock = 20 ns; adder delay ~1 ns, plenty of slack).
4. ✅ All signal pads named in manifest.
5. ✅ IP statement.
6. ✅ Verilog model for testbench.

Run:
```python
from tape_out import assemble, validate_for_chipignite, TapeoutMetadata, Shuttle

metadata = TapeoutMetadata(
    project_name="adder4_smoke_test",
    designer="A. Lurie",
    email="<email>",
    shuttle=Shuttle.CHIPIGNITE_OPEN_MPW,
    pdk_version="6f6c4e5",
    top_module="adder4",
    git_url="https://github.com/.../adder4-chipignite",
)

bundle = assemble(
    metadata=metadata,
    gds=Path("build/adder4.gds"),
    lef=Path("build/adder4.lef"),
    def_file=Path("build/adder4.def"),
    verilog=Path("rtl/adder4.v"),
    drc_report=drc_report,
    lvs_report=lvs_report,
    timing_report={"worst_setup_ns": 1.2, "worst_hold_ns": 0.05},
    density_report={"met1": 0.32, "met2": 0.28},
    output_dir=Path("tapeout/"),
)

validation = validate_for_chipignite(bundle)
assert validation.passed
print("Ready for chipIgnite submission")
```

The bundle is then submitted to Efabless's portal (manual step, out of automation scope). They review, send back manufactured packaged chips ~6 months later.

## Worked Example — Reality check

For our v1 silicon stack, "tape-out" is more accurately "tape-out-format" — we generate files that *look like* a real tape-out. Whether the user actually pays Efabless and waits 6 months is their choice. For the teaching path:

1. Write the 4-bit adder.
2. Run through the full stack: HDL → HIR → synth → tech-map → place → route → GDS.
3. Run signoff: DRC, LVS, density, antenna.
4. Assemble a chipIgnite-format bundle.
5. ✅ Educational success: you understand every step.

For real fabricated silicon: same flow + Efabless submission + payment + 6 months.

## Pre-tape-out checklist

```
[ ] DRC clean (signoff with KLayout)
[ ] LVS clean (signoff with Netgen or our LVS)
[ ] Antenna check clean
[ ] Density: every layer within (5%, 80%) per any 100x100 µm² tile
[ ] Timing: setup slack > 0 in worst PVT corner; hold slack > 0
[ ] Power: estimated power < pad current × VDD
[ ] All pads in manifest match GDS pin labels
[ ] IP statement (open-source license or proprietary disclaimer)
[ ] Behavioral Verilog matches expected behavior in simulation
[ ] Pad ring matches selected packaging (typically TQFP-44 or QFN-32 for chipIgnite)
[ ] Reset signal pulled to a defined value at power-on
[ ] No floating gates
[ ] Decoupling capacitors on power pads (decap cells inserted)
[ ] Filler cells inserted (for density)
[ ] All clocks have CTS-style clock distribution (out of scope for trivial designs)
```

## Edge Cases

| Scenario | Handling |
|---|---|
| chipIgnite shuttle deadline missed | Wait for next shuttle (~2-4 months). |
| Design exceeds shuttle area | Split into multiple shuttles or use full wafer (paid). |
| User doesn't have an Efabless account | Document signup process; out of automation scope. |
| Foundry-specific NDA constraints | chipIgnite uses Sky130 (open); no NDA. For other PDKs, requires foundry sign-off. |
| GDS includes unknown cells | LVS catches; reject bundle. |
| Pad locations conflict with package | Validation catches. |
| Manifest missing required field | Validation catches; reports specific field. |

## Test Strategy

### Unit (95%+)
- Manifest YAML emission produces valid YAML.
- File-presence check enforces required artifacts.
- Validation rules each have positive + negative tests.

### Integration
- 4-bit adder generates a complete bundle.
- Bundle YAML parses (cross-validate against chipIgnite's schema).
- (If hardware) submit a real chipIgnite project; receive packaged chip ~6 months later.

## Conformance

| Reference | Coverage |
|---|---|
| **Efabless chipIgnite** open MPW | Full bundle format |
| **Caravel** harness integration | Documented; future spec for full automation |
| **OpenRoad** signoff flow | Cross-validate via shell-out |
| **TinyTapeout** (smaller-scale shuttles) | Future spec |
| **Other foundries** (TSMC, GlobalFoundries) | Out of scope |

## Open Questions

1. **Caravel integration automation** — generate `user_project_wrapper.v` automatically? Future work.
2. **Tape-out preview** — visual check of pad ring + IO layout? Future.
3. **Multi-project tape-out** (mini-shuttle with multiple designs) — future spec.

## Future Work

- Full Caravel integration spec.
- TinyTapeout shuttle support.
- Other PDKs (GF180MCU, ASAP7).
- Signoff timing automation.
- Yield modeling per shuttle.
- Post-silicon test pattern generation.
- Bring-up support after chips arrive (test programs, board design).
