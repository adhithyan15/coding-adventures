# tape-out

Tape-out bundle assembly and validation for Efabless chipIgnite silicon submissions.

## Pipeline position

```
drc-lvs ──► tape-out ──► [submission bundle]
gdsii-writer ──────────► tape-out
lef-def ───────────────► tape-out
```

## What it does

Assembles the complete set of files required for an Efabless chipIgnite shuttle submission and validates them against acceptance criteria:

- **Required files**: GDS, LEF, DEF, Verilog, DRC report, LVS report
- **Required metadata**: project name, designer, email, top module name
- **Signoff**: DRC and LVS must both be "clean"
- **Pad locations**: warned (not errored) if missing for chipIgnite Open MPW

The library produces text content (manifest.yaml, README.md) but does **not** write files itself — callers own the I/O.

## Key types

| Type | Description |
|------|-------------|
| `TapeoutMetadata` | Project info: name, designer, email, shuttle, PDK, clock, VDD |
| `TapeoutBundle` | Metadata + files map + pad locations + signoff map |
| `PadLocation` | IO pad: name, direction, x/y coordinates |
| `Shuttle` | `ChipigniteOpenMpw`, `ChipignitePaidMpw`, `TinyTapeout` |
| `ValidationReport` | passed flag, errors list, warnings list |

## Usage

```rust
use tape_out::{TapeoutBundle, TapeoutMetadata, validate_for_chipignite, render_manifest};

let meta = TapeoutMetadata { project_name: "adder4".into(), ..TapeoutMetadata::default() };
let mut bundle = TapeoutBundle::new(meta);
bundle.files.insert("gds".into(), "adder4.gds".into());
bundle.signoff.insert("drc".into(), "clean".into());

let report = validate_for_chipignite(&bundle);
let manifest_yaml = render_manifest(&bundle);
```

## Testing

```
cargo test -p tape-out -- --nocapture
```

15 integration tests + 1 doc-test covering: full valid bundle, missing required fields, missing files, dirty DRC/LVS, pad location warnings, manifest/readme content.
