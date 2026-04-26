# GDSII Writer

## Overview

GDSII (Calma Stream Format) is the binary mask layout format. Every fab speaks it; every layout viewer (KLayout, Magic) reads it. GDSII is what actually goes to the foundry — the polygons in this file become the masks that pattern the silicon.

This spec defines a writer that takes a routed DEF + cell-library GDS files and emits a top-level `.gds` for the design. Cells are inserted by reference (SREF — structure reference) so the GDS is hierarchical and compact; routing geometry becomes BOUNDARY or PATH records on the appropriate layer/datatype.

GDSII is a binary record format from 1978 (yes, really). The bit layouts are baroque — fixed-point reals, big-endian, length-prefixed records. We follow the spec exactly; round-trip with KLayout for confidence.

## Layer Position

```
asic-routing.md (routed DEF)        sky130-pdk.md (cell GDS files,
              │                                     layer/datatype map)
              └──────────────┬───────────────────┘
                             ▼
                ┌────────────────────────────┐
                │  gdsii-writer.md            │  ◀── THIS SPEC
                │  (DEF + cell GDS → top GDS) │
                └────────────────────────────┘
                             │
                             ▼ (.gds binary)
                       drc-lvs.md, tape-out.md
```

## GDSII Format

### Record structure

Every record:
- 2 bytes: length (big-endian)
- 1 byte: record type
- 1 byte: data type (e.g., 02 = 2-byte int, 03 = 4-byte int, 05 = 8-byte real, 06 = string)
- N bytes: data

Record types:
- `0x0002` HEADER — version
- `0x0102` BGNLIB — library start
- `0x0206` LIBNAME — library name
- `0x0305` UNITS — DB unit and user unit
- `0x0400` ENDLIB — library end
- `0x0502` BGNSTR — structure start (a "structure" = a cell)
- `0x0606` STRNAME — structure name
- `0x0700` ENDSTR — structure end
- `0x0800` BOUNDARY — polygon
- `0x0900` PATH — wire (polyline + width)
- `0x0a00` SREF — structure reference (instance)
- `0x0b00` AREF — array reference (repeated instance)
- `0x0c00` TEXT — text label
- `0x0d02` LAYER — layer number
- `0x0e02` DATATYPE — datatype
- `0x0f03` WIDTH — path width
- `0x1003` XY — coordinate list
- `0x1100` ENDEL — element end
- `0x1206` SNAME — referenced structure name
- `0x1305` STRANS — instance transformation
- `0x1505` MAG — magnification
- `0x1605` ANGLE — rotation
- ... others

### Bit layout sample (BOUNDARY record)

```
length type+datatype data
04 00 08 00          BOUNDARY (no payload)
06 00 0d 02 LL       LAYER (LL = layer number, 2 bytes)
06 00 0e 02 DD       DATATYPE
NN 00 10 03 X1 Y1 X2 Y2 X3 Y3 X1 Y1   XY (4-byte ints, must close polygon)
04 00 11 00          ENDEL
```

(Coordinates are in DBU — database units. UNITS sets DBU = 1 nm typically; user unit is 1 µm.)

### Hierarchy via SREF

A top-level cell containing 16 INV cells doesn't draw 16 inverter layouts; it has 16 SREFs, each pointing to a single INV definition. The cell is reused by reference. This is how GDSII gives huge designs reasonable file sizes.

### Sky130 layer/datatype map

| Layer | Datatype | Purpose |
|---|---|---|
| 64 (well) | 20 (NWELL drawing) | n-well |
| 64 | 16 (PWELL drawing) | p-well |
| 65 (DIFF) | 20 | active region |
| 66 (POLY) | 20 | poly gate |
| 67 (LICON1) | 44 | li1 contact |
| 67 | 20 | li1 metal |
| 68 (MCON) | 44 | met1 contact |
| 68 | 20 | met1 metal |
| 69 (VIA) | 44 | via to met2 |
| ... | ... | met2-5 etc. |

(Full Sky130 layer map is loaded from `sky130_fd_pr` PDK files; we do not enumerate.)

## Public API

```python
from dataclasses import dataclass
from pathlib import Path
from enum import Enum


@dataclass
class GdsLibrary:
    name: str
    user_unit: float = 1e-6      # 1 µm
    db_unit: float = 1e-9        # 1 nm
    cells: dict[str, "GdsCell"]


@dataclass
class GdsCell:
    name: str
    boundaries: list["GdsBoundary"]
    paths: list["GdsPath"]
    refs: list["GdsRef"]
    arefs: list["GdsAref"]
    texts: list["GdsText"]


@dataclass
class GdsBoundary:
    layer: int
    datatype: int
    points: list[tuple[int, int]]    # in DBU; first == last (closed)


@dataclass
class GdsPath:
    layer: int
    datatype: int
    points: list[tuple[int, int]]
    width: int                       # in DBU


@dataclass
class GdsRef:
    sname: str                       # referenced cell name
    location: tuple[int, int]
    angle: float = 0.0
    mag: float = 1.0
    reflect: bool = False


@dataclass
class GdsAref(GdsRef):
    cols: int = 1
    rows: int = 1
    pitch_col: int = 0
    pitch_row: int = 0


@dataclass
class GdsText:
    layer: int
    text_type: int
    location: tuple[int, int]
    text: str
    angle: float = 0.0
    mag: float = 1.0


def read_gds(path: Path) -> GdsLibrary: ...
def write_gds(lib: GdsLibrary, path: Path) -> None: ...


@dataclass
class GdsBuilder:
    """Builds a top-level GDS from DEF + cell GDS files + tech LEF."""
    pdk: "Pdk"
    
    def build(self, routed_def: "Def", design_name: str) -> GdsLibrary: ...
    def merge_cell_libs(self, base: GdsLibrary, cell_libs: list[GdsLibrary]) -> GdsLibrary: ...
```

## Worked Example — 4-bit Adder

Inputs:
- `adder4_routed.def` (placed + routed, ~6 KB DEF).
- Sky130 cell GDS files (one per cell type used).

Building:
1. Load `sky130_fd_sc_hd__nand2_1.gds` (etc.) into a base library.
2. Create a top-level cell `adder4`.
3. For each placed component in DEF: emit an SREF pointing to the cell + transform.
4. For each routed segment in DEF: emit a PATH on the appropriate layer/datatype.
5. For each top-level pin: emit a TEXT label (optional).
6. Serialize to binary.

Output: `adder4.gds` ~5-10 KB. Opens in KLayout: shows the placed cells (each with their internal layout from Sky130 reference) connected by routed metal segments. Layer colors match Sky130 conventions.

## Worked Example — 32-bit ALU

~430 placed cells, ~5000 routed segments. GDS file ~80-150 KB. Loads in KLayout in <1 sec.

## Edge Cases

| Scenario | Handling |
|---|---|
| Polygon not closed (first ≠ last) | Auto-close. |
| Polygon with self-intersection | KLayout/DRC will flag; we don't validate at write time. |
| Path with zero width | Reject. |
| Angle not 0/90/180/270 | Allowed; emit ANGLE record. |
| Cell name with special chars | Reject; GDSII allows only A-Z, 0-9, _, $. |
| Cell name longer than 32 chars | GDSII limit; truncate or reject. |
| File > 4 GB | Reject; GDSII has 4 GB limit (offset is 32-bit). |
| Big-endian vs little-endian system | Always emit big-endian per spec. |
| Real numbers (MAG, ANGLE) | 8-byte fixed-point; not IEEE 754. |

## Test Strategy

### Unit (95%+)
- Each record type writes correct bytes.
- Real-number conversion (8-byte fixed-point) round-trips.
- Big-endian encoding is correct on little-endian hosts.
- Reading our output: round-trip identity (modulo header date).

### Integration
- 4-bit adder: GDS opens in KLayout; expected layers visible.
- 32-bit ALU: GDS opens in KLayout; ~430 cell instances visible.
- (Cross-validation) Klayout's `gds_inspect` or Calibre `calibrebatch` reads our file without errors.
- Round-trip via klayout: read our GDS, save, re-read; identical.

## Conformance

| Standard | Coverage |
|---|---|
| **GDSII Stream Format** (Calma 1978) | Full subset: HEADER, BGNLIB, UNITS, BGNSTR, BOUNDARY, PATH, SREF, AREF, TEXT, ENDEL, ENDSTR, ENDLIB |
| **OASIS** (newer; XML-based) | Out of scope; future spec |
| **Properties / element flags** | Subset (UNAME, ATTR not in v1) |
| **Box records** (deprecated in modern flows) | Skip |

## Open Questions

1. **OASIS as primary format** — defer; GDSII is universal.
2. **Pcell evaluation** — parametric cell expansion is out of scope (Sky130 ships flat).
3. **Compression** — gzip post-write? Defer.

## Future Work

- OASIS writer.
- Streaming write for very large designs.
- Pcell evaluation engine.
- GDSII reader hardening (handle malformed files gracefully).
- Cell flatten / hierarchy preserve options.
- Layer-by-layer GDS export for inspection.
