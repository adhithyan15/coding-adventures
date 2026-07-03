# vcd-writer

Streaming VCD (Value Change Dump) writer — IEEE 1364-2005 §18.

## What it does

`vcd-writer` produces VCD files from a sequence of time-stamped value changes.
VCD is the universal waveform format read by GTKWave, Surfer, ModelSim, and every
other waveform viewer.

The writer is **decoupled from any specific simulator**: you feed it events; it
produces text.  An `attach` helper integrates it with the `hardware-vm` event
callback interface.

## VCD file structure

```
$date 2026-06-13 00:00:00 UTC $end
$version Silicon-Stack VCD Writer 0.1.0 $end
$timescale 1ps $end
$scope module adder $end
  $var wire 4 ! a [3:0] $end
  $var wire 5 " sum [4:0] $end
$upscope $end
$enddefinitions $end
#0
b0000 !
b00000 "
#10
b0011 !
b01000 "
```

## Where it fits

```
hardware-vm → vcd-writer → *.vcd → GTKWave / Surfer
```

## Usage

```rust
use vcd_writer::VcdWriter;

let mut vcd = VcdWriter::new("1ps");
vcd.open_scope("adder");
let a_id   = vcd.declare("a",   4, "wire");
let sum_id = vcd.declare("sum", 5, "wire");
vcd.close_scope();
vcd.end_definitions();

vcd.time(0);
vcd.value_change(&a_id,   0);
vcd.value_change(&sum_id, 0);

vcd.time(10);
vcd.value_change(&a_id,   3);
vcd.value_change(&sum_id, 8);

let text = vcd.finish();
// text is a complete, valid VCD string.
```

### Integrating with hardware-vm

```rust
use std::collections::HashMap;
use vcd_writer::{VcdWriter, attach, SignalEvent};

let mut vcd = VcdWriter::new("1ps");
let clk_id = vcd.declare("clk", 1, "wire");
vcd.end_definitions();

let mut name_to_id = HashMap::new();
name_to_id.insert("clk".to_string(), clk_id);
let router = attach(name_to_id);

// Inside a hardware-vm subscriber:
// vm.subscribe(move |ev| {
//     let se = SignalEvent { time: ev.time, signal: ev.signal.clone(), new_value: ev.new_value };
//     if let Some((t, id, v)) = router(se) {
//         vcd.value_change_at(t, &id, v);
//     }
// });
```

## API

| Method | Description |
|--------|-------------|
| `new(timescale)` | Create writer; writes date/version/timescale preamble |
| `open_scope(name)` | `$scope module name $end` |
| `open_scope_kind(name, kind)` | `$scope <kind> name $end` |
| `close_scope()` | `$upscope $end` |
| `declare(name, width, kind)` | Declare a variable; returns its compact VCD ID |
| `end_definitions()` | `$enddefinitions $end`; called automatically if not done |
| `time(t)` | Emit `#t` timestamp |
| `dump_initial(map)` | Emit `$dumpvars` block with initial values |
| `value_change(id, val)` | Emit value change; no-op if value unchanged |
| `value_change_at(t, id, val)` | Advance time then emit change |
| `finish()` | Consume writer; return complete VCD text |
| `text()` | Borrow accumulated VCD text so far |

## Tests

```
cargo test -p vcd-writer
```

16 integration tests + 1 doc-test.
