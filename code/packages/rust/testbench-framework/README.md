# testbench-framework

HDL testbench harness for Rust — run named tests against a `HardwareVm` DUT with
exhaustive or random stimulus, a global test registry, and per-test panic capture.

## Where it fits

```
hdl-ir  ──►  hardware-vm  ──►  testbench-framework
                               coverage-hdl
```

`testbench-framework` sits above `hardware-vm` and `hdl-ir`.  You write test
closures that drive inputs and assert outputs; the harness wires each closure to
a fresh simulator instance and records pass/fail.

## Core concepts

### DutHandle

A thin wrapper around `HardwareVm` exposing `get(name) -> i64` and
`set(name, value)`.  Think of it as the oscilloscope probe and signal generator
on a lab bench.

```rust
tc = TestCase::new("buffer_high", |dut: &mut DutHandle| {
    dut.set("a", 1);
    assert_eq!(dut.get("y"), 1);
});
```

### TestCase

Builder-style:

```rust
TestCase::new("name", closure)
    .with_timeout(10.0)   // wall-clock budget (informational in v0.1.0)
    .expect_fail()        // pass iff the closure panics
```

### run()

```rust
let report = run(hir, Some(vec![tc1, tc2, tc3]));
// or: run(hir, None)  — drains the global registry
assert!(report.all_passed(), "{}", report.summary());
```

Each test gets a *brand-new* `HardwareVm`.  Panics are caught with
`std::panic::catch_unwind`; one failing test never aborts the suite.

### Global registry

```rust
register_test("buf_high", |dut| { ... });
let report = run(hir, None);   // discovers from registry
```

Useful for `#[test]` integration suites where each test module registers its
cases at startup.

### Stimulus helpers

```rust
// Drive every 2^9 = 512 combinations of a 4-bit + 4-bit + 1-bit input:
exhaustive(dut, &[("a", 4), ("b", 4), ("cin", 1)], Some(&mut |d| {
    // assert combinational output
})).unwrap();

// Drive 1000 random vectors, seed=42 for reproducibility:
random_stimulus(dut, &[("a", 8), ("b", 8)], 1000, 42, Some(&mut |d| {
    // check output
}));
```

`exhaustive` returns `Err` if total input bits exceed 20 (> 1M iterations).

## Usage example

```rust
use testbench_framework::{run, TestCase, DutHandle};

fn my_tests(hir: Hir) {
    let tc = TestCase::new("all_zeros", |dut: &mut DutHandle| {
        dut.set("a", 0);
        assert_eq!(dut.get("y"), 0);
    });
    let report = run(hir, Some(vec![tc]));
    println!("{}", report.summary());
    assert!(report.all_passed());
}
```

## Test coverage

18 integration tests + 5 doctests.  Run with:

```
cargo test -p testbench-framework
```
