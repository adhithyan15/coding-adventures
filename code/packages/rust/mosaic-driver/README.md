# mosaic-driver

The CLI entry point that stitches together the three Mosaic compiler stages into
a single end-to-end pipeline.

## Pipeline

```text
.mil -> mosmodel-compiler  -> interface descriptor JSON
                              |
.mll -> moslayout-compiler <-+-> part-map JSON
                              |
.msl -> mosstyle-compiler  <-+-> Lattice source
```

`mosstyle-compiler` still exposes compatibility CSS for callers that need it,
but new Mosaic style output should prefer the Lattice artifact.

## Usage

```sh
# Full three-stage compile from CWD (looks for Grid.mil, Grid.mll, Grid.msl)
mosaic Grid

# Individual stages
mosaic --interface  Grid.mil     # print interface descriptor JSON
mosaic --layout     Grid.mll     # print part-map JSON
mosaic --style      Grid.msl     # print Lattice
```

## Output (stdout JSON)

```json
{
  "component": "Grid",
  "interface": {
    "component": "Grid",
    "slots": [
      { "name": "column-headers", "type": { "List": "Text" }, "required": true },
      { "name": "viewport-rows", "type": { "List": "Text" }, "required": true }
    ],
    "emits": [
      { "name": "onRowClick", "params": [{ "name": "row", "type": "Number" }] }
    ]
  },
  "parts": {
    "component": "Grid",
    "parts": [
      { "name": "root", "primitive": "Column" },
      { "name": "cell-grid", "primitive": "Grid" }
    ]
  },
  "lattice": ".mos-Grid-root { ... }\n.mos-Grid-cell-grid { ... }",
  "css": ".mos-Grid-root { ... }\n.mos-Grid-cell-grid { ... }"
}
```

## Build

```sh
cargo build -p mosaic-driver
./target/debug/mosaic --help
```
