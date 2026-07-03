# coverage-hdl

HDL coverage measurement for Rust — toggle coverage, functional coverpoints, and
cross-product coverage, all driven by a `HardwareVm` event subscription.

## Where it fits

```
hardware-vm  ──►  coverage-hdl
```

`CoverageRecorder` subscribes to a `HardwareVm`'s event stream and updates
coverage metrics on every signal transition.  It sits alongside
`testbench-framework` — attach a recorder to the DUT's VM before handing it
to the runner.

## Core concepts

### Bins

A *bin* is a named predicate on a signal value.

```rust
let b_hi  = bin_value("high", 255);         // matches exactly 255
let b_mid = bin_range("mid",  64, 192);     // matches 64..=192
let b_dflt = bin_default();                 // matches anything
```

Bins are `Clone` (via `Arc<dyn Fn>`) so the same spec can be shared.

### Coverpoints

A coverpoint samples one signal and increments the first matching bin.

```rust
let cp = Coverpoint::new("data_val", "data_bus", vec![
    bin_value("zero", 0),
    bin_range("low",  1,  127),
    bin_range("high", 128, 254),
    bin_value("max",  255),
]);
// coverage() = hit_bins / total_bins  (0.0 → 1.0)
```

### Cross coverage

A `CrossPoint` measures joint coverage across two or more coverpoints — the
Cartesian product of their bin spaces.

```rust
let cross = CrossPoint::new("op_x_data", vec![opcode_cp, data_cp]);
// sample() uses the last sampled value per coverpoint signal.
```

### CoverageRecorder

```rust
let mut recorder = CoverageRecorder::new(&mut vm);  // subscribes to vm
recorder.add_coverpoint(data_cp);
recorder.add_cross(cross);
recorder.enable_toggle_coverage(&["clk", "reset", "data_bus"]);

// ... run stimulus ...

let report = recorder.report();
println!("Overall: {:.1}%", recorder.overall_coverage() * 100.0);
```

The recorder uses `Arc<Mutex<RecorderInner>>` so the VM callback (`Fn + Send + 'static`)
can share state with the recorder handle.

### Toggle coverage

Counts rising (0→non-zero) and falling (non-zero→0) transitions per signal.

```rust
let ts = &report.toggle["clk"];
println!("clk: {} rising, {} falling", ts.rising, ts.falling);
```

### Overall coverage

`overall_coverage()` averages coverpoint, cross, and toggle coverage (each
category weighted equally; toggle is 1.0 when every enabled signal has seen
both edges).

## Usage example

```rust
use coverage_hdl::{bin_range, bin_value, Coverpoint, CoverageRecorder};
use hardware_vm::HardwareVm;

fn measure(hir: Hir) {
    let mut vm = HardwareVm::new(hir).unwrap();
    let mut rec = CoverageRecorder::new(&mut vm);
    rec.add_coverpoint(Coverpoint::new("out", "y", vec![
        bin_value("zero", 0),
        bin_value("one",  1),
    ]));
    rec.enable_toggle_coverage(&["y"]);
    vm.set_input("a", 0).unwrap();
    vm.set_input("a", 1).unwrap();
    let rpt = rec.report();
    println!("{:#?}", rpt);
}
```

## Test coverage

19 integration tests + 5 doctests.  Run with:

```
cargo test -p coverage-hdl
```
