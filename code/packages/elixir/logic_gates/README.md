# Logic Gates (Elixir)

The foundation of all digital computing — implemented as pure functions in Elixir.

## What This Package Does

Implements the seven fundamental logic gates (NOT, AND, OR, XOR, NAND, NOR, XNOR), proves NAND is functionally complete by building all gates from NAND alone, and provides sequential logic elements (latches, flip-flops, registers, counters) built entirely from gates.

## Layer Position

```
[Logic Gates] → Arithmetic → CPU → ARM → Assembler → Lexer → Parser → Compiler → VM
```

This is the foundation layer — it has no dependencies.

## Usage

```elixir
alias CodingAdventures.LogicGates

# Fundamental gates
LogicGates.not_gate(1)       # => 0
LogicGates.and_gate(1, 1)    # => 1
LogicGates.or_gate(0, 1)     # => 1
LogicGates.xor_gate(1, 1)    # => 0

# Composite gates
LogicGates.nand_gate(1, 1)   # => 0
LogicGates.nor_gate(0, 0)    # => 1
LogicGates.xnor_gate(1, 1)   # => 1

# NAND-derived (proving functional completeness)
LogicGates.nand_not(1)       # => 0
LogicGates.nand_and(1, 1)    # => 1
LogicGates.nand_or(0, 1)     # => 1
LogicGates.nand_xor(1, 0)    # => 1

# Multi-input variants
LogicGates.and_n([1, 1, 1])  # => 1
LogicGates.or_n([0, 0, 1])   # => 1

# Sequential logic
{q, q_bar} = LogicGates.sr_latch(1, 0, 0, 1)  # Set the latch
{q, q_bar} = LogicGates.d_latch(1, 1, 0, 1)   # Transparent when enabled
```

## Public API

### Fundamental Gates

| Function | Inputs | Output | Description |
|----------|--------|--------|-------------|
| `not_gate(a)` | 1 | 1 | Inverts: 0→1, 1→0 |
| `and_gate(a, b)` | 2 | 1 | 1 only if BOTH are 1 |
| `or_gate(a, b)` | 2 | 1 | 1 if EITHER is 1 |
| `xor_gate(a, b)` | 2 | 1 | 1 if inputs DIFFER |

### Composite Gates

| Function | Description |
|----------|-------------|
| `nand_gate(a, b)` | NOT(AND) — functionally complete |
| `nor_gate(a, b)` | NOT(OR) — also functionally complete |
| `xnor_gate(a, b)` | NOT(XOR) — equality gate |

### NAND-Derived

| Function | Construction |
|----------|-------------|
| `nand_not(a)` | NAND(a, a) |
| `nand_and(a, b)` | NAND(NAND(a,b), NAND(a,b)) |
| `nand_or(a, b)` | NAND(NAND(a,a), NAND(b,b)) |
| `nand_xor(a, b)` | 4 NAND gates |

### Multi-Input

| Function | Description |
|----------|-------------|
| `and_n(inputs)` | AND across N inputs |
| `or_n(inputs)` | OR across N inputs |

### Sequential Logic

| Function | Description |
|----------|-------------|
| `sr_latch(set, reset, q, q_bar)` | Set-Reset latch |
| `d_latch(data, enable, q, q_bar)` | Data latch with enable |
| `d_flip_flop(data, clock, state)` | Edge-triggered flip-flop |
| `register(data, clock, state)` | N-bit word storage |
| `shift_register(serial_in, clock, state, opts)` | Serial-to-parallel |
| `counter(clock, reset, state)` | Binary counter |

## Running Tests

```bash
mix test --cover
```
