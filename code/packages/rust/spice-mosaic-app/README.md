# spice-mosaic-app

`spice-mosaic-app` is the stateful host adapter for the Berkeley SPICE Mosaic
workbench. It owns the editable deck and selected analysis row, then delegates
inspection and simulation to `spice-netlist-parser`.

The workbench has table and waveform views while retaining raw JSON as the
canonical result artifact. Its initial schematic-capture API is a small,
editor-owned Rust model: R, C, DC voltage, and ground symbols connect through
exact grid endpoints, then lower deterministically to a Berkeley `.op` deck.
This is not a second simulator or parser format; hosts pass the emitted text to
the same parser and engine used by the deck editor. Interactive placement,
selection, and synchronization are the next UI phase.

```sh
cargo test -p spice-mosaic-app
cargo clippy -p spice-mosaic-app --all-targets -- -D warnings
```
