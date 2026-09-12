# spice-mosaic-app

`spice-mosaic-app` is the stateful host adapter for the Berkeley SPICE Mosaic
workbench. It owns the editable deck and selected analysis row, then delegates
inspection and simulation to `spice-netlist-parser`.

The workbench has table and waveform views while retaining raw JSON as the
canonical result artifact. Its initial schematic-capture API is a small,
editor-owned Rust model: R, C, L, DC voltage/current, AC voltage, and ground
symbols connect through exact, unique grid-terminal endpoints. The document persists an
operating-point, DC sweep, AC sweep, or transient choice and lowers it
deterministically to a Berkeley deck.
This is not a second simulator or parser format; hosts pass the emitted text to
the same parser and engine used by the deck editor. The workbench exposes a
palette for deterministic grid placement, selected-component routing, and a
selected-component value inspector. Non-ground values are validated as one
SPICE token before synchronization; orthogonal wire/terminal geometry retains
endpoint-only netlist semantics.

The selected analysis card is also canonical schematic state. DC sweeps choose
from independent voltage/current symbols, and DC, AC, and transient cards each
expose their runnable Berkeley parameters before synchronization. Those values
remain one-token inputs and lower directly to `.dc`, `.ac dec`, or `.tran` in
the same deck consumed by the parser and engine.

Hosts load, place, wire, select, edit component values, route, and synchronize that document through
the Mosaic app event contract. They can also select a DC sweep source and edit
the active analysis card. Synchronization replaces the text deck with the
canonical emitted deck and refreshes its runnable Berkeley analysis plan.

```sh
cargo test -p spice-mosaic-app
cargo clippy -p spice-mosaic-app --all-targets -- -D warnings
```
