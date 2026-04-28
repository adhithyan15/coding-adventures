# tape-out

Bundle assembly for the Efabless chipIgnite Sky130 shuttle. **Zero deps.**

See [`code/specs/tape-out.md`](../../../specs/tape-out.md).

## Quick start

```python
from pathlib import Path
from tape_out import (
    TapeoutMetadata, TapeoutBundle, PadLocation, Shuttle,
    write_bundle, validate_for_chipignite,
)

metadata = TapeoutMetadata(
    project_name="adder4",
    designer="Adhithya Rajasekaran",
    email="me@example.com",
    shuttle=Shuttle.CHIPIGNITE_OPEN_MPW,
    pdk="sky130A",
    top_module="adder4",
    git_url="https://github.com/...",
    clock_frequency_mhz=50.0,
)

bundle = TapeoutBundle(metadata=metadata)
bundle.files = {
    "gds":          Path("build/adder4.gds"),
    "lef":          Path("build/adder4.lef"),
    "def":          Path("build/adder4.def"),
    "verilog":      Path("rtl/adder4.v"),
    "drc_report":   Path("build/drc.rpt"),
    "lvs_report":   Path("build/lvs.rpt"),
}
bundle.pad_locations = [
    PadLocation("a[0]", "input", x=0, y=100),
    PadLocation("cout", "output", x=1000, y=100),
]
bundle.signoff = {"drc": "clean", "lvs": "clean", "antenna": "clean"}

# Validate first
report = validate_for_chipignite(bundle)
assert report.passed, report.errors

# Write the bundle directory
out = write_bundle(bundle, Path("tapeout/adder4"))
print(f"Bundle written to {out}")
```

## v0.1.0 scope

- `TapeoutMetadata` + `TapeoutBundle` + `PadLocation` data classes
- Shuttle enum: `CHIPIGNITE_OPEN_MPW`, `CHIPIGNITE_PAID_MPW`, `TINY_TAPEOUT`
- `write_bundle(bundle, out_dir)`: copies files, emits manifest.yaml + README.md
- `validate_for_chipignite(bundle)`: checks required fields + files + signoff state
- Required files: gds, lef, def, verilog, drc_report, lvs_report

## Out of scope (v0.2.0)

- Caravel user-project wrapper integration (the standard chipIgnite flow)
- Automatic pad-ring generation
- TinyTapeout-specific bundling rules
- Other-PDK support (GF180MCU, ASAP7)

MIT.
