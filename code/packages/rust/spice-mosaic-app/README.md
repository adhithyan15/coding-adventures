# spice-mosaic-app

`spice-mosaic-app` is the stateful host adapter for the first usable Berkeley
SPICE Mosaic workbench. It owns the editable deck and selected analysis row,
then delegates inspection and simulation to `spice-netlist-parser`.

The app deliberately presents raw JSON output in this first UI slice. The next
UI phase can replace that text pane with table and waveform renderers without
changing the host event or snapshot boundary.

```sh
cargo test -p spice-mosaic-app
cargo clippy -p spice-mosaic-app --all-targets -- -D warnings
```
