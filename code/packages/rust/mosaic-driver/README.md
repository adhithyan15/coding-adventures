# mosaic-driver

The CLI entry point that stitches together the three Mosaic compiler stages into a
single end-to-end pipeline.

## Pipeline

```
.mil  ──▶  mosmodel-compiler  ──▶  interface descriptor JSON
                                       │
.mll  ──▶  moslayout-compiler ◀────────┤  ──▶  part-map JSON
                                       │
.msl  ──▶  mosstyle-compiler  ◀────────┘  ──▶  CSS string
```

## Usage

```sh
# Full three-stage compile from CWD (looks for Grid.mil, Grid.mll, Grid.msl)
mosaic Grid

# Individual stages
mosaic --interface  Grid.mil     # print interface descriptor JSON
mosaic --layout     Grid.mll     # print part-map JSON
mosaic --style      Grid.msl     # print CSS
```

## Output (stdout JSON)

```json
{
  "component": "Grid",
  "interface": {
    "component": "Grid",
    "slots": [
      { "name": "column-headers", "type": { "List": "Text" }, "required": true },
      { "name": "viewport-rows",  "type": { "List": "Text" }, "required": true }
    ],
    "emits": [
      { "name": "onRowClick", "params": [{ "name": "row", "type": "Number" }] }
    ]
  },
  "parts": {
    "component": "Grid",
    "parts": [
      { "name": "root",      "primitive": "Column" },
      { "name": "cell-grid", "primitive": "Grid"   }
    ]
  },
  "css": ".mos-Grid-root { ... }\n.mos-Grid-cell-grid { ... }"
}
```

## Build

```sh
cargo build -p mosaic-driver
./target/debug/mosaic --help
```
