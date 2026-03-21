# GPU Core (Ruby)

A generic, pluggable GPU processing element simulator built on IEEE 754
floating-point arithmetic from logic gates.

## What is this?

This gem simulates a single GPU core -- the smallest independently
programmable compute unit on a GPU. It's like a tiny, simplified CPU that
does one thing well: floating-point math.

The core is **pluggable**: you can swap out the instruction set (ISA) to
simulate any vendor's GPU core -- NVIDIA CUDA cores, AMD Stream Processors,
Intel Vector Engines, or ARM Mali Execution Engines. The default `GenericISA`
provides a vendor-neutral set of 16 opcodes for education.

## Where it fits in the stack

```
Logic Gates (AND, OR, NOT, XOR)
  +-- Arithmetic (half_adder, full_adder, ripple_carry_adder)
      +-- FP Arithmetic (IEEE 754 encoding, add, mul, fma)
          +-- GPU Core (this package)  <-- you are here
```

## Architecture

- **FPRegisterFile** -- stores FloatBits values in 1-256 configurable registers
- **LocalMemory** -- byte-addressable scratchpad with FP load/store (default 4KB)
- **Instruction** -- immutable instruction representation (16 opcodes)
- **GenericISA** -- default educational ISA (pluggable via duck typing)
- **GPUCore** -- the fetch-execute loop that ties it all together
- **GPUCoreTrace** -- execution trace records for debugging and visualization

## Usage

```ruby
require "coding_adventures_gpu_core"
include CodingAdventures

# Create a core with the default GenericISA
core = GpuCore::GPUCore.new

# Write a program: compute 3.0 * 4.0 = 12.0
core.load_program([
  GpuCore.limm(0, 3.0),    # R0 = 3.0
  GpuCore.limm(1, 4.0),    # R1 = 4.0
  GpuCore.fmul(2, 0, 1),   # R2 = R0 * R1
  GpuCore.halt,             # stop
])

# Run and inspect
traces = core.run
puts core.registers.read_float(2)  # => 12.0

# Print execution trace
traces.each { |t| puts t.format }
```

## The 16 Opcodes

| Category   | Opcode | Description                    |
|------------|--------|--------------------------------|
| Arithmetic | FADD   | Rd = Rs1 + Rs2                 |
| Arithmetic | FSUB   | Rd = Rs1 - Rs2                 |
| Arithmetic | FMUL   | Rd = Rs1 * Rs2                 |
| Arithmetic | FFMA   | Rd = Rs1 * Rs2 + Rs3           |
| Arithmetic | FNEG   | Rd = -Rs1                      |
| Arithmetic | FABS   | Rd = \|Rs1\|                   |
| Memory     | LOAD   | Rd = Mem[Rs1 + offset]         |
| Memory     | STORE  | Mem[Rs1 + offset] = Rs2        |
| Data Move  | MOV    | Rd = Rs1                       |
| Data Move  | LIMM   | Rd = immediate float           |
| Control    | BEQ    | if Rs1 == Rs2: PC += offset    |
| Control    | BLT    | if Rs1 < Rs2: PC += offset     |
| Control    | BNE    | if Rs1 != Rs2: PC += offset    |
| Control    | JMP    | PC = target (absolute)         |
| Control    | NOP    | no operation                   |
| Control    | HALT   | stop execution                 |

## Pluggable ISA

The core uses duck typing -- any object with `#name` and
`#execute(instruction, registers, memory)` methods works as an ISA:

```ruby
class MyCustomISA
  def name
    "Custom"
  end

  def execute(instruction, registers, memory)
    # decode and execute, return an ExecuteResult
  end
end

core = GpuCore::GPUCore.new(isa: MyCustomISA.new)
```

## Running Tests

```bash
cd code/packages/ruby/gpu_core
bundle install
bundle exec rake test
```

## License

MIT
