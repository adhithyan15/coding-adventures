# Ruby HDL DSL

## Overview

A Chisel-style Ruby DSL for hardware description. The third HDL front-end alongside Verilog and VHDL — one that puts the *full power of a host programming language* (Ruby, in this case) at the user's disposal for parameterization, abstraction, and metaprogramming, without losing the rigor of an HDL.

The argument: Verilog/VHDL parameterization is awkward (genvar gymnastics, `generate-for` blocks). Anyone writing parameterized hardware ends up reaching for templating systems (M4, Python preprocessors, Jinja2). A *host-language* DSL makes parameter passing trivial — it's just method arguments — while still producing IEEE-equivalent hardware semantics through tracing.

This spec defines the DSL's surface (the user-facing classes and operators), its elaboration semantics (how Ruby execution becomes HIR), and the relationship to the rest of the stack (HIR → simulation, synthesis, etc.).

### Design philosophy
- **Same target as Verilog/VHDL** — emits HIR; a Ruby-DSL design is indistinguishable downstream from a Verilog one.
- **Tracing-style elaboration** — Ruby code *runs*; operations on hardware-typed values *record* HIR nodes. Like Chisel, like Migen, like SpinalHDL.
- **Synthesizable subset** — HIR is full-IEEE-power, but the Ruby DSL only generates the synthesizable subset. No `wait`, no `$display`. (Use the testbench framework for those — `testbench-framework.md`.)
- **Type-safe-ish** — Ruby is dynamically typed, but the DSL types (`UInt(N)`, `SInt(N)`, `Bool`) carry width information; width-mismatch operations raise at elaboration time.
- **Idiomatically Ruby** — operator overloading, blocks, enumerators. A Ruby programmer should feel at home.

### What it's *not*
- Not Chisel (which is Scala-based; we adopt the philosophy, not the syntax).
- Not Verilog inside Ruby strings (we don't do string templating).
- Not a SystemVerilog clone (no classes, interfaces, assertions in the DSL).

## Layer Position

```
Verilog/VHDL source                 Ruby DSL source (`.rb` files)
       │                                    │
       ▼                                    ▼
  parser AST                       DSL elaboration trace
       │                                    │
       └─────────────┬──────────────────────┘
                     ▼
            hdl-elaboration.md
                     │
                     ▼
                    HIR
                     │
                     ▼
            (rest of the stack)
```

## Concepts

### Hardware values

The atom of the DSL is a **hardware value** — a Ruby object that wraps an HIR `Expr` plus type info. Operations on hardware values produce more hardware values; the operation graph *is* the HIR being built.

```ruby
a = Wire(UInt(4))            # creates Wire HIR node, value is its NetRef
b = Wire(UInt(4))
sum = a + b                  # creates BinaryOp("+", a.expr, b.expr); sum is a UInt(5)
```

`a + b` doesn't add anything — it builds a node. The actual computation happens at simulation/runtime.

### Width arithmetic

Most operations follow well-defined width rules:

| Op | Result width |
|---|---|
| `a + b` | `max(a.w, b.w)` (truncated; no carry-out) |
| `a +& b` | `max(a.w, b.w) + 1` (expanded; preserves carry) |
| `a - b` | `max(a.w, b.w)` |
| `a * b` | `a.w + b.w` |
| `a / b` | `a.w` (integer division) |
| `a & b` | `max(a.w, b.w)` (zero-padded short side) |
| `a << k` | `a.w + k` |
| `a[hi, lo]` | `hi - lo + 1` |
| `Cat(x, y)` | `x.w + y.w` |
| `a == b` | `1` (Bool) |

Mismatches: `UInt(4) + SInt(4)` is an error (don't mix sign); use explicit `as_signed` / `as_unsigned`.

### Module class

A user defines a hardware module by subclassing `Module`:

```ruby
class FullAdder < Module
  io = Bundle.new(
    a:    Input(UInt(1)),
    b:    Input(UInt(1)),
    cin:  Input(UInt(1)),
    sum:  Output(UInt(1)),
    cout: Output(UInt(1)),
  )

  io.sum  := io.a ^ io.b ^ io.cin
  io.cout := (io.a & io.b) | ((io.a ^ io.b) & io.cin)
end
```

When the class is elaborated:
1. Class body executes top-to-bottom.
2. The `io` Bundle defines ports.
3. The `:=` operator records `ContAssign` nodes.
4. The result is a Module HIR node.

### Hierarchy

```ruby
class Adder4 < Module
  io = Bundle.new(
    a:    Input(UInt(4)),
    b:    Input(UInt(4)),
    cin:  Input(UInt(1)),
    sum:  Output(UInt(4)),
    cout: Output(UInt(1)),
  )

  carries = Array.new(5) { Wire(UInt(1)) }
  carries[0] := io.cin

  4.times do |i|
    fa = Instantiate(FullAdder)
    fa.io.a   := io.a[i]
    fa.io.b   := io.b[i]
    fa.io.cin := carries[i]
    io.sum[i] := fa.io.sum
    carries[i+1] := fa.io.cout
  end

  io.cout := carries[4]
end
```

`Instantiate(FullAdder)` creates an Instance node. The `4.times do |i| ... end` is just Ruby — no `genvar` ceremony. Each iteration records one Instance and four ContAssigns.

### Sequential logic

Registers via `Reg`:

```ruby
counter = Reg(UInt(8), init: 0)
counter := counter + 1
```

`Reg` declares a register with optional `init:`; the elaborator records a `Net` (kind=`reg`) plus a synthesizable always-block stub that updates it on the implicit clock. Clock and reset come from the Module's implicit `clock` and `reset` (a deliberate Chisel convention; eliminates clock plumbing).

Optional explicit clock/reset:

```ruby
class MultiClock < Module
  io = Bundle.new(
    fast_clk: Input(Clock),
    slow_clk: Input(Clock),
    data:     Input(UInt(8)),
    out:      Output(UInt(8)),
  )

  with_clock(io.fast_clk) do
    fast_reg = Reg(UInt(8))
    fast_reg := io.data
  end
  
  with_clock(io.slow_clk) do
    slow_reg = Reg(UInt(8))
    slow_reg := io.data
    io.out  := slow_reg
  end
end
```

### Conditional logic

```ruby
out = Wire(UInt(8))
when_(sel == 0) { out := a }
.elsewhen(sel == 1) { out := b }
.elsewhen(sel == 2) { out := c }
.otherwise          { out := 0 }
```

`when_` (trailing underscore to avoid Ruby keyword `when`) starts a conditional; it's chainable. The DSL records this as a chain of `IfStmt`s in a synthesizable always-block.

### State machines

```ruby
class Traffic < Module
  io = Bundle.new(
    light: Output(UInt(2)),
  )
  
  state = Reg(Enum.new(:red, :green, :yellow), init: :red)
  
  switch_(state) do |sw|
    sw.is(:red)    { state := :green }
    sw.is(:green)  { state := :yellow }
    sw.is(:yellow) { state := :red }
  end
  
  io.light := state.as_uint
end
```

`Enum` types map to `TyEnum` in HIR; `switch_` records a `CaseStmt`.

### Memories

```ruby
mem = Mem(256, UInt(32))         # 256-entry × 32-bit memory
read_data = mem.read(read_addr)
mem.write(write_addr, write_data, write_enable)
```

Inferred to BRAM by FPGA backend, to register file by default in ASIC backend.

### Bundles and Vec

```ruby
PixelBundle = Bundle.new(r: UInt(8), g: UInt(8), b: UInt(8))
pixel = Wire(PixelBundle)
pixel.r := 0xFF
pixel.g := 0
pixel.b := 0xFF

framebuffer = Vec(640 * 480, PixelBundle)
framebuffer[0] := pixel
```

### Parameterization

```ruby
class ParametricAdder < Module
  def initialize(width)
    @width = width
    super()
  end
  
  io = Bundle.new(
    a:   Input(UInt(@width)),
    b:   Input(UInt(@width)),
    sum: Output(UInt(@width + 1)),
  )
  
  io.sum := io.a +& io.b
end

adder8  = ParametricAdder.new(8)
adder16 = ParametricAdder.new(16)
```

Trivial. No genvar, no generate. Just Ruby.

### Generators (vs modules)

Sometimes you want a piece of logic that's not a module but a reusable function:

```ruby
def carry_chain(a, b, cin)
  cout = a & b | (a ^ b) & cin
  sum  = a ^ b ^ cin
  [sum, cout]
end

s, c = carry_chain(io.a, io.b, io.cin)
```

This is just a Ruby method. It records HIR nodes in the current Module's scope. No instance is created.

### DontCare and explicit unconnected

```ruby
debug = Output(UInt(8))
debug := DontCare    # explicit unused output
```

Maps to `Lit("X", ...)` in HIR; tools may optimize away.

## Public API (Ruby)

```ruby
# Types
class HwType end
class UInt < HwType; def self.[](w); ... end end
class SInt < HwType; def self.[](w); ... end end
class Bool < HwType end
class Clock < HwType end
class Reset < HwType end
class Enum < HwType; def initialize(*vals); ... end end
class Bundle < HwType; def initialize(**fields); ... end end
class Vec < HwType; def initialize(n, t); ... end end

# Hardware values
class HwValue
  attr_reader :type, :expr, :provenance
  def +(other); ... end
  def +&(other); ... end       # expand-width add
  def -(other); ... end
  def *(other); ... end
  def &(other); ... end
  def |(other); ... end
  def ^(other); ... end
  def <<(other); ... end
  def >>(other); ... end
  def ==(other); ... end
  def !=(other); ... end
  def <(other); ... end
  def >(other); ... end
  def [](hi, lo=nil); ... end  # bit/range select
  def :=(other); ... end       # connect / assign
  def as_signed; ... end
  def as_unsigned; ... end
  def as_uint; ... end
end

# Constructors
def Wire(type) end
def Reg(type, init: nil, clock: nil, reset: nil) end
def Mem(size, type) end
def Input(type) end
def Output(type) end
def Inout(type) end

# Module base
class Module
  def self.elaborate; ... end
  def initialize; ... end
  def io; @io end
end

# Conditional
def when_(cond, &block); end
class WhenContext
  def elsewhen(cond, &block); end
  def otherwise(&block); end
end

# State machine
def switch_(expr, &block); end
class SwitchContext
  def is(value, &block); end
  def default(&block); end
end

# Hierarchy
def Instantiate(module_class, *args); end

# Special values
DontCare = ...
def Cat(*args); end          # concatenation
def Mux(sel, *cases); end    # multiplexer
def Fill(n, v); end          # replication
```

## Elaboration

`Module.elaborate` is the entry point. It:

1. Sets up an "elaboration context" — a thread-local scratchpad for the in-progress HIR Module.
2. Instantiates the class (calls `initialize`).
3. Runs the class body, which records ContAssigns/Processes/Instances into the context.
4. Returns the resulting `Module` HIR node.

Operator overloading is the magic. When a user writes `a + b`, the `+` method on `HwValue` returns a new `HwValue` whose `.expr` is `BinaryOp("+", a.expr, b.expr)`. Nothing is added; an HIR node is created.

`:=` (Ruby's setter convention) is overridden on hardware values. `out := expr` records a `ContAssign(target=out.expr, rhs=expr.expr)` in the elaboration context.

The `when_` block uses Ruby blocks (closures) to capture conditional bodies. Inside the block, ContAssigns are recorded into a temporary buffer; the buffer becomes the `then_branch` of an `IfStmt`.

## Worked Example 1 — 4-bit Adder

```ruby
class FullAdder < Module
  io = Bundle.new(
    a: Input(UInt(1)), b: Input(UInt(1)), cin: Input(UInt(1)),
    sum: Output(UInt(1)), cout: Output(UInt(1)),
  )
  io.sum  := io.a ^ io.b ^ io.cin
  io.cout := (io.a & io.b) | ((io.a ^ io.b) & io.cin)
end

class Adder4 < Module
  io = Bundle.new(
    a: Input(UInt(4)), b: Input(UInt(4)), cin: Input(UInt(1)),
    sum: Output(UInt(4)), cout: Output(UInt(1)),
  )
  c = Array.new(5) { Wire(UInt(1)) }
  c[0] := io.cin
  4.times do |i|
    fa = Instantiate(FullAdder)
    fa.io.a   := io.a[i]
    fa.io.b   := io.b[i]
    fa.io.cin := c[i]
    io.sum[i] := fa.io.sum
    c[i+1]    := fa.io.cout
  end
  io.cout := c[4]
end
```

After elaboration, this produces the same HIR as the Verilog/VHDL versions in `hdl-ir.md`.

## Worked Example 2 — Parameterized FSM (FIFO controller)

```ruby
class FifoController < Module
  def initialize(depth = 16)
    @depth = depth
    @ptr_w = Math.log2(depth).ceil
    super()
  end
  
  io = Bundle.new(
    enq:    Input(Bool),
    deq:    Input(Bool),
    full:   Output(Bool),
    empty:  Output(Bool),
    count:  Output(UInt(@ptr_w + 1)),
  )
  
  wptr = Reg(UInt(@ptr_w), init: 0)
  rptr = Reg(UInt(@ptr_w), init: 0)
  cnt  = Reg(UInt(@ptr_w + 1), init: 0)
  
  do_enq = io.enq & !io.full
  do_deq = io.deq & !io.empty
  
  when_(do_enq) { wptr := wptr + 1 }
  when_(do_deq) { rptr := rptr + 1 }
  
  when_(do_enq & !do_deq) { cnt := cnt + 1 }
  .elsewhen(!do_enq & do_deq) { cnt := cnt - 1 }
  
  io.full  := cnt == @depth
  io.empty := cnt == 0
  io.count := cnt
end
```

`@depth = 16` → 4-bit pointer. `@depth = 256` → 8-bit pointer. Same code; specialization is just a Ruby integer.

## Worked Example 3 — Reusable generator (priority encoder)

```ruby
def priority_encoder(bits)
  out = Wire(UInt(Math.log2(bits.length).ceil))
  bits.length.times do |i|
    when_(bits[i]) { out := i }
  end
  out
end

# Use it:
io.encoded := priority_encoder([io.req0, io.req1, io.req2, io.req3])
```

A Ruby method that produces hardware. Generates a chain of `when_`/`elsewhen` clauses; result is a single Wire.

## Edge Cases

| Scenario | Handling |
|---|---|
| Width mismatch in `+` | Auto-extend short side with zeros (UInt) or sign-extend (SInt). |
| Mixed UInt + SInt | Explicit `as_signed`/`as_unsigned` required. |
| Reading from output | Allowed within the same module (Verilog reg-style). |
| Multiple assignments to same wire | Last-assignment-wins per `when_` chain; multiple unconditional → error. |
| Reg without init | Uninitialized; sim treats as X. |
| Cyclic combinational dependency (a := b; b := a) | Detected at elaboration end; error. |
| Instantiate without arguments | Calls module's no-arg constructor. |
| `:=` used outside a Module elaboration | Error: "no current Module context." |
| Bundle field type mismatch on connect | Error. |
| Memory write without enable | Always writes (port becomes Read+Write). |
| Method-defined logic returning multiple values | Use `[a, b] = method(...)` (Ruby destructuring). |

## Test Strategy

### Unit (target 95%+)
- Each operator on each type (UInt+UInt, SInt-SInt, etc.).
- Width inference correctness on every op.
- Type errors raise.
- `Wire`, `Reg`, `Mem` create correct HIR nodes.
- `when_/elsewhen/otherwise` chains produce correct nested IfStmt.
- `switch_/is/default` produces correct CaseStmt.
- Module instantiation produces Instance with correct connections.

### Property
- Round-trip: same DSL code elaborates to same HIR every time (deterministic).
- Equivalence: hand-written Verilog and DSL of the same circuit produce structurally equivalent HIR.

### Integration
- Adder4, ALU32, FSM, Fifo, RegFile examples elaborate cleanly.
- Generated HIR simulates correctly.
- Generated HIR synthesizes correctly.

## Open Questions

1. **`init:` reset polarity** — synchronous vs asynchronous? Recommendation: implicit reset is sync for ASIC, async for FPGA; configurable.
2. **Implicit clock and reset** — name them `clock` and `reset`? Recommendation: yes (Chisel convention).
3. **Polyglot port: should other languages get analogous DSLs?** Recommendation: yes — Python `python-hdl-dsl.md`, Go, Rust eventually. Same target HIR.
4. **DSL for testbenches** — separate spec or extend? Recommendation: separate (testbench-framework.md).
5. **Strict `mypy --strict`-equivalent type checking** — Sorbet for Ruby? Recommendation: defer; runtime checks for v1.

## Future Work

- Python equivalent (`python-hdl-dsl.md`).
- Go and Rust equivalents.
- Class-based collateral (interfaces, generic modules).
- Direct simulation via the Ruby DSL (skip HIR for fast prototyping).
- Visualization of the elaboration trace as a graph.
- Static analysis for unconnected wires / dead code.
- Integration with cocotb-style testing.
