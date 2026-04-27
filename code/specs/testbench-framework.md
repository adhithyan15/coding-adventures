# Testbench Framework

## Overview

A testbench is the design's stimulus + check harness: drives inputs, captures outputs, validates correctness. This spec defines a multi-surface testbench framework with three coexisting interfaces:

1. **Native HDL testbenches** — `initial begin ... end` Verilog or `process ... end process` VHDL, parsed by F05 + extensions, executed by `hardware-vm.md`. The traditional path. Required for compatibility with industry-standard testbenches.
2. **Ruby DSL `Tester`** — testbench-as-Ruby-code, in the same DSL family as `ruby-hdl-dsl.md`. Same elaboration philosophy.
3. **Python harness** (cocotb-style) — Python coroutines drive a Verilog/VHDL DUT. The most ergonomic for complex stimulus/checking.

All three feed the same `hardware-vm.md` and produce the same `.vcd` output. They coexist; pick what fits the test.

## Layer Position

```
hardware-vm.md
       │
       ▲ (testbench drives the VM)
       │
   ┌───┴────┬───────────────┬───────────────┐
Native HDL  Ruby DSL Tester  Python harness  ◀── three surfaces
   │         │               │
   ▼         ▼               ▼
       Same elaborated HIR + driver bridge
```

## Surfaces

### Native HDL

```verilog
module tb_adder4;
  reg [3:0] a, b;
  reg cin;
  wire [3:0] sum;
  wire cout;
  
  adder4 dut (.a(a), .b(b), .cin(cin), .sum(sum), .cout(cout));
  
  initial begin
    $dumpfile("tb.vcd"); $dumpvars(0, tb_adder4);
    a = 0; b = 0; cin = 0;
    #5;
    
    for (integer i = 0; i < 256; i = i + 1) begin
      a = i[3:0]; b = i[7:4]; cin = 0;
      #5;
      if ({cout, sum} !== a + b)
        $display("FAIL @ %t: a=%h b=%h sum=%h cout=%b", $time, a, b, sum, cout);
    end
    
    $display("PASS");
    $finish;
  end
endmodule
```

The HIR for this includes `Process(kind=INITIAL)` with delays and assertions. `hardware-vm` runs it natively.

### Ruby DSL Tester

```ruby
class TbAdder4 < TestBench
  dut = Adder4.new
  
  test "exhaustive 256 combinations" do
    256.times do |i|
      a = i & 0xF
      b = (i >> 4) & 0xF
      cin = 0
      expected = a + b + cin
      
      poke(dut.io.a, a)
      poke(dut.io.b, b)
      poke(dut.io.cin, cin)
      step()
      
      assert_equal expected & 0x1F, peek(dut.io.cout) << 4 | peek(dut.io.sum),
                   "a=#{a} b=#{b}"
    end
  end
end
```

`poke`, `peek`, `step` are DSL methods that drive the VM.

### Python harness (cocotb-style)

```python
import cocotb_lite as ct

@ct.test
async def adder4_exhaustive(dut):
    for i in range(256):
        a = i & 0xF
        b = (i >> 4) & 0xF
        cin = 0
        dut.a.value = a
        dut.b.value = b
        dut.cin.value = cin
        await ct.step()
        expected = (a + b + cin) & 0x1F
        actual = (dut.cout.value << 4) | dut.sum.value
        assert actual == expected, f"a={a} b={b} got={actual} expected={expected}"
```

The harness is a Python module that runs as the simulator's testbench.

## Concepts

### Test discovery

Tests are named via decorator (Python) or block (Ruby). The runner:
1. Loads the test file.
2. Discovers tests.
3. For each test: instantiates DUT, runs the test, collects results.

### Stimulus generation

| Pattern | Use case |
|---|---|
| **Directed** | A specific sequence of inputs probing a known case. |
| **Exhaustive** | All input combinations (only for small input spaces). |
| **Random** | Random vectors; useful with seeding for repro. |
| **Constrained-random** | Random vectors satisfying constraints (`a < b`, `a is even`). |
| **Sequence** | A high-level sequence object (e.g., "10 reads, 5 writes, then a flush") that compiles to low-level stimulus. |

Constrained-random is supported via a simple constraint solver (random sampling + reject) for v1; full SAT-backed solver in future.

### Self-checking

Three patterns:
1. **Inline assertion**: `assert(actual == expected)` — fails immediately on mismatch.
2. **Reference model**: a Python/Ruby function that computes expected output; compared each cycle.
3. **Scoreboard**: queue-based comparison for designs with latency (DUT outputs and reference outputs are queued and matched in order).

### Coverage hooks

Each test can declare cover points; the framework records them. After the test, a coverage report is generated. (See `coverage.md`.)

### Reset and clock generators

```python
async def adder4_test(dut):
    await ct.reset(dut.reset, cycles=5)   # async helper
    ct.clock(dut.clk, period_ns=10)
    # ... test body ...
```

`reset` and `clock` are utilities that run in parallel with the test.

## Public API

### Python

```python
import asyncio
from typing import Callable, AsyncIterator


class Test:
    name: str
    func: Callable[["DUTHandle"], "asyncio.Future"]


class DUTHandle:
    """Wraps an HIR Module; signal accessors via attribute."""
    def __getattr__(self, name: str) -> "SignalHandle": ...


class SignalHandle:
    @property
    def value(self) -> int | None: ...
    @value.setter
    def value(self, v: int) -> None: ...


# decorators / helpers

def test(func: Callable) -> Test: ...

async def step(n: int = 1) -> None:
    """Advance simulation by n delta cycles."""
    ...

async def time(t: int) -> None:
    """Advance simulation to absolute time t."""
    ...

async def edge(signal: SignalHandle, kind: str = "rising") -> None: ...

async def reset(rst: SignalHandle, cycles: int = 5, active: int = 1) -> None: ...

async def clock(clk: SignalHandle, period_ns: float = 10.0) -> None: ...


# runner

class TestRunner:
    def __init__(self, hir: "HIR", top: str): ...
    def discover(self, module: str) -> list[Test]: ...
    def run(self, tests: list[Test]) -> "TestReport": ...


@dataclass
class TestReport:
    passed: list[str]
    failed: list[tuple[str, str]]   # (name, message)
    duration_ms: float
    coverage: dict[str, float]
```

### Ruby DSL Tester

```ruby
class TestBench
  def self.dut(module_class); ...; end
  
  def test(name, &block); ...; end
  
  def poke(signal, value); ...; end
  def peek(signal); ...; end
  def step(n = 1); ...; end
  def reset(signal, cycles: 5); ...; end
  def assert_equal(expected, actual, msg = nil); ...; end
end
```

## Worked Examples

### Example 1 — 4-bit Adder exhaustive (Python)

```python
@ct.test
async def adder_exhaustive(dut):
    for a in range(16):
        for b in range(16):
            for cin in (0, 1):
                dut.a.value = a
                dut.b.value = b
                dut.cin.value = cin
                await ct.step()
                got = (dut.cout.value << 4) | dut.sum.value
                expected = (a + b + cin) & 0x1F
                assert got == expected
```

512 stimulus, 512 checks. Runs in <1 sec.

### Example 2 — Pipelined ALU with scoreboard (Python)

```python
@ct.test
async def alu_pipelined(dut):
    ct.clock(dut.clk, period_ns=10)
    await ct.reset(dut.reset, cycles=3)
    
    scoreboard = []   # FIFO of expected results
    
    for _ in range(1000):
        a = random.randint(0, 0xFFFFFFFF)
        b = random.randint(0, 0xFFFFFFFF)
        op = random.randint(0, 7)
        dut.a.value = a; dut.b.value = b; dut.op.value = op
        await ct.edge(dut.clk, "rising")
        scoreboard.append(reference_alu(a, b, op))
    
    # Drain pipeline (4 stages)
    for _ in range(4):
        await ct.edge(dut.clk, "rising")
    
    # Now check outputs
    for expected in scoreboard:
        await ct.edge(dut.clk, "rising")
        assert dut.y.value == expected
```

### Example 3 — Constrained-random for FIFO (Ruby)

```ruby
class TbFifo < TestBench
  dut Fifo.new(depth: 16)
  
  test "stress with constrained random" do
    1000.times do
      enq = rand_bool weight: 0.6   # bias toward enqueue
      deq = rand_bool weight: 0.4
      data = rand_bits 8
      
      poke dut.io.enq, enq
      poke dut.io.deq, deq
      poke dut.io.din, data
      step
      
      assert !peek(dut.io.full) || !enq, "enq while full"
      assert !peek(dut.io.empty) || !deq, "deq while empty"
    end
  end
end
```

## Edge Cases

| Scenario | Handling |
|---|---|
| Test deadlock (await never satisfied) | Timeout: each test has a default 1-sec sim-time budget; configurable. |
| Multiple tests in same file | Each runs in fresh VM; no state leakage. |
| Signal driven from both testbench and DUT | Error; testbench is "outside" the DUT's drivers. |
| Reading an X or Z value | Returns `None`; tests can check. |
| Negative test (expecting failure) | `@ct.test(should_fail=True)`. |
| Test seeds | Configurable via env var or test arg for repro. |
| Cross-test parallelism | Allowed; each test is an independent VM instance. |

## Test Strategy

### Unit (95%+)
- Each helper (`step`, `time`, `edge`, `reset`, `clock`) drives the VM correctly.
- Constrained-random reject works.
- Scoreboard FIFO matches.

### Integration
- 4-bit adder exhaustive test: 512 checks in <1 sec.
- ALU stress test: 1M random vectors with reference model.
- ARM1 reference: existing testbench runs through the framework.

## Conformance

| Standard | Coverage |
|---|---|
| **IEEE 1364-2005** initial blocks, system tasks ($display, $finish, $monitor, $dumpfile, $dumpvars, $assert) | Full |
| **IEEE 1076-2008** assert/report/severity, textio | Full |
| **cocotb** API compatibility | Subset (covers essentials; full cocotb is its own product) |
| **UVM** (Universal Verification Methodology) | Out of scope |

## Open Questions

1. UVM-style component layering (sequencer/driver/monitor/scoreboard) — defer to future spec.
2. Constrained-random with full SAT solver — defer.
3. Coverage hooks integration — see `coverage.md`.

## Future Work

- UVM-lite component framework.
- Full SAT-backed constraint solver.
- Property-based fuzzing.
- VPI/VHPI compatibility (cocotb proper).
- Distributed test runner.
