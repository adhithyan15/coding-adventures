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

Web targets that need CSS should compile the generated Lattice through the
Lattice transpiler. Mosaic's style-stage artifact is Lattice, not CSS.

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
  "style_map_json": "{\n  \"component\": \"Grid\",\n  \"parts\": [ ... ]\n}"
}
```

## Build

```sh
cargo build -p mosaic-driver
./target/debug/mosaic --help
```
