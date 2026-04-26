# gdsii-writer

GDSII Calma Stream Format binary writer. The format every fab speaks. **Zero deps.**

See [`code/specs/gdsii-writer.md`](../../../specs/gdsii-writer.md).

## Quick start

```python
from gdsii_writer import GdsWriter

with GdsWriter("adder4.gds", library_name="adder4") as gds:
    gds.begin_structure("nand2_1")
    gds.boundary(layer=66, datatype=20, points=[
        (0.0, 0.0), (0.5, 0.0), (0.5, 0.3), (0.0, 0.3)
    ])
    gds.end_structure()

    gds.begin_structure("adder4")
    gds.sref("nand2_1", x=10, y=0)
    gds.sref("nand2_1", x=12, y=0)
    gds.path(layer=68, datatype=20, width=0.14, points=[(10.5, 1.5), (12.5, 1.5)])
    gds.text(layer=68, text_type=0, x=0, y=0, text="adder4")
    gds.end_structure()
```

The output is a binary `.gds` that opens in KLayout.

## v0.1.0 scope

- HEADER, BGNLIB/ENDLIB, LIBNAME, UNITS
- BGNSTR/ENDSTR, STRNAME (cell hierarchy)
- BOUNDARY (polygons, auto-closes if last point != first)
- PATH (wires with width)
- SREF (cell instances; supports angle, mag, reflection via STRANS/MAG/ANGLE)
- TEXT (pin labels)
- LAYER, DATATYPE, XY, WIDTH, SNAME records
- 8-byte fixed-point real conversion per Calma spec
- Streaming write (in-memory buffer flushed on close)

## Out of scope (v0.2.0)

- AREF (array reference) — emit as multiple SREFs for v0.1.0.
- GDSII reader (parsing).
- Properties (PROPATTR/PROPVALUE).
- BOX records (deprecated).

MIT.
