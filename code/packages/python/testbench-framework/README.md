# testbench-framework

Pythonic harness around HardwareVM. Decorate test functions, get value-based assertions on signal handles. v0.1.0 is sync (since hardware-vm is combinational); async lands in v0.2.0 alongside sequential simulation.

See [`code/specs/testbench-framework.md`](../../../specs/testbench-framework.md).

## Quick start

```python
from testbench_framework import test, run, exhaustive
from hdl_elaboration import elaborate_verilog

src = """
module adder4(input [3:0] a, input [3:0] b, input cin,
              output [3:0] sum, output cout);
  assign {cout, sum} = a + b + cin;
endmodule
"""
hir = elaborate_verilog(src, top="adder4")

@test
def addition_works(dut):
    dut.a.value = 5
    dut.b.value = 3
    dut.cin.value = 0
    assert dut.sum.value == 8
    assert dut.cout.value == 0

@test
def overflow_detected(dut):
    dut.a.value = 0xF
    dut.b.value = 1
    dut.cin.value = 0
    assert dut.cout.value == 1

@test
def adder_exhaustive(dut):
    def check(d):
        expected = (d.a.value + d.b.value + d.cin.value) & 0x1F
        actual = (d.cout.value << 4) | d.sum.value
        assert actual == expected

    exhaustive(dut, inputs={"a": 4, "b": 4, "cin": 1}, on_step=check)

report = run(hir)
print(report.summary())  # "3 passed, 0 failed, 0 skipped in 0.045s"
assert report.all_passed
```

## v0.1.0 scope

- `@test` decorator: register a function as a testbench test.
- `run(hir, tests=None)` runs all registered tests. Each test gets its own fresh VM so state never leaks.
- `DUTHandle`: attribute-access wrapper over an HardwareVM. `dut.signal.value` reads/writes.
- `TestReport`: passed/failed/skipped lists, durations, summary.
- Stimulus helpers:
  - `exhaustive(dut, inputs, on_step=None)`: drive every combination of input values (limited to ≤ 20 bits total).
  - `random_stimulus(dut, inputs, iterations, seed=42, on_step=None)`: random vectors with reproducible seed.
- `should_fail=True` on `@test` for negative tests.

## Out of scope (v0.2.0)

- Async / sequential testbenches with clocks (waits on `hardware-vm` v0.2 for clocked sim).
- Constrained-random stimulus.
- Reference-model + scoreboard pattern helpers.
- Native HDL `initial` blocks (handled via `hardware-vm` directly when it gains `initial` support).

MIT.
