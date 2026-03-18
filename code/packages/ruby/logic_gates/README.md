# Logic Gates (Ruby)

**Layer 10 of the computing stack** -- the foundation of all digital logic.

## What is a logic gate?

A logic gate is the simplest possible computing element. It takes one or two binary inputs (each either 0 or 1) and produces one binary output (0 or 1). The output is completely determined by the inputs -- no state, no memory, no randomness.

In real hardware, logic gates are built from transistors -- tiny electronic switches. A modern CPU contains billions of transistors organized into billions of logic gates. But at the conceptual level, every computation reduces to these simple 0-or-1 operations.

## The gates

### NOT (Inverter)

The simplest gate -- it has one input and flips it.

```
Input -> Output
  0   ->   1
  1   ->   0
```

### AND

Takes two inputs. The output is 1 **only if both inputs are 1**.

```
A  B  -> Output
0  0  ->   0
0  1  ->   0
1  0  ->   0
1  1  ->   1
```

### OR

Takes two inputs. The output is 1 **if either input is 1** (or both).

```
A  B  -> Output
0  0  ->   0
0  1  ->   1
1  0  ->   1
1  1  ->   1
```

### XOR (Exclusive OR)

Takes two inputs. The output is 1 **if the inputs are different**.

```
A  B  -> Output
0  0  ->   0
0  1  ->   1
1  0  ->   1
1  1  ->   0
```

### NAND, NOR, XNOR

The inverted versions of AND, OR, and XOR respectively. NAND is special because it is **functionally complete** -- every other gate can be built from NAND alone.

## NAND-derived gates

This package includes implementations of all gates built exclusively from NAND operations, proving functional completeness:

- `nand_not(a)` -- NOT from NAND: `NAND(a, a)`
- `nand_and(a, b)` -- AND from NAND: `NOT(NAND(a, b))`
- `nand_or(a, b)` -- OR from NAND: `NAND(NOT(a), NOT(b))`
- `nand_xor(a, b)` -- XOR from NAND: built from 4 NAND gates

## Multi-input gates

- `and_n(*inputs)` -- AND with N inputs (requires at least 2)
- `or_n(*inputs)` -- OR with N inputs (requires at least 2)

## Where it fits

```
[Logic Gates] -> Arithmetic -> CPU -> ARM/RISC-V -> Assembler -> Lexer -> Parser -> Compiler -> VM
```

This package is used by the **arithmetic** package to build half adders, full adders, and the ALU.

## Installation

```bash
gem install coding_adventures_logic_gates
```

Or add to your Gemfile:

```ruby
gem "coding_adventures_logic_gates"
```

## Usage

```ruby
require "coding_adventures_logic_gates"

CodingAdventures::LogicGates.and_gate(1, 1)    # => 1 -- both inputs are 1
CodingAdventures::LogicGates.and_gate(1, 0)    # => 0 -- one input is 0
CodingAdventures::LogicGates.or_gate(0, 1)     # => 1 -- at least one input is 1
CodingAdventures::LogicGates.not_gate(1)       # => 0 -- inverted
CodingAdventures::LogicGates.xor_gate(1, 0)    # => 1 -- inputs are different
CodingAdventures::LogicGates.xor_gate(1, 1)    # => 0 -- inputs are the same
CodingAdventures::LogicGates.and_n(1, 1, 1, 0) # => 0 -- one input is 0
```

## Input validation

All gates validate their inputs:
- Non-Integer types (true, false, "1", 1.0, nil) raise `TypeError`
- Integers outside {0, 1} raise `ArgumentError`

## Spec

See [10-logic-gates.md](../../../specs/10-logic-gates.md) for the full specification.
