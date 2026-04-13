# CodingAdventures::Intel8008GateLevel

A gate-level simulator for the **Intel 8008** (April 1972) — the world's first
commercial 8-bit microprocessor. Every arithmetic and logical operation routes
through actual gate functions from `CodingAdventures::LogicGates` and
`CodingAdventures::Arithmetic`.

This package is part of the layered stack:

```
CodingAdventures::Transistors     — CMOS switch model
CodingAdventures::LogicGates      — AND/OR/NOT/XOR/flip-flops from transistors
CodingAdventures::Arithmetic      — adders from logic gates
CodingAdventures::Intel8008GateLevel — 8008 CPU from gates and adders
```

## Why gate-level vs behavioral?

The behavioral simulator (`CodingAdventures::Intel8008Simulator`) executes
instructions directly with host-language integers. This gate-level simulator
routes **every computation** through the gate abstraction layer:

| Operation        | Implementation                                                   |
|-----------------|------------------------------------------------------------------|
| ADD r           | `alu_add` → `ripple_carry_adder(\@a_bits, \@b_bits, $carry_in)` |
| SUB r           | `alu_sub` → NOT each B bit, `ripple_carry_adder(..., 1)`         |
| ANA / ORA / XRA | 8 AND / OR / XOR gates (one per bit)                            |
| INR / DCR       | `alu_inr` / `alu_dcr` — adder with carry preserved              |
| RLC / RRC       | Bit shift via wire routing + one gate for the wrapped bit        |
| RAL / RAR       | Rotate through carry — one gate per bit for the mux             |
| Parity flag P   | `NOT(XORn(@bits))` — 7-input XOR tree via `XORn`                |
| Registers       | D flip-flop states via `Register()` from LogicGates              |

## Architecture

### 8-bit ALU

All 8 ALU operations (ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP) route through:

- **Addition**: `ripple_carry_adder` from `CodingAdventures::Arithmetic`
- **Subtraction**: Two's complement (NOT all bits of B, add with carry_in=1)
- **Carry convention**: CY=1 means borrow occurred (unsigned A < operand)

### Register file

7 × 8-bit registers (A, B, C, D, E, H, L) stored as arrays of D flip-flop
state hashrefs. Index mapping matches 8008 hardware encoding:
B=0, C=1, D=2, E=3, H=4, L=5, M=6 (pseudo, not physical), A=7.

Two-phase clock write protocol:
```
Phase 1 (clock=0): data captured into master latch
Phase 2 (clock=1): master latches to slave (data appears at output)
```

### 8-level push-down stack

The 8008 has a hardware stack of 8 × 14-bit registers. Entry 0 **is** the
program counter — there is no separate PC register. CALL rotates all entries
down (entry 7 lost), RETURN rotates all entries up.

### Gate budget (approximate)

| Component        | Gates  | Notes                                           |
|-----------------|--------|-------------------------------------------------|
| Register file    | 224    | 7 regs × 8 bits × 4 gates/flip-flop            |
| Stack            | 448    | 8 × 14 bits × 4 gates/flip-flop                 |
| ALU              | 160    | 8 full adders × ~20 gates each                  |
| Flags            | 32     | carry, zero, sign, parity (8-bit XOR tree)      |
| Decoder          | ~200   | AND/OR/NOT tree for opcode decode               |
| **Total**        | **~1064** | (compare: real 8008 ~3,500 transistors)      |

## Usage

```perl
use CodingAdventures::Intel8008GateLevel;

my $cpu = CodingAdventures::Intel8008GateLevel->new();

# MVI B,1 ; MVI A,2 ; ADD B ; HLT
my $program = pack('C*', 0x06, 0x01, 0x3E, 0x02, 0x80, 0x76);
my @traces  = $cpu->run($program, 100);

printf "A = %d\n", $cpu->register_a;   # 3
printf "CY = %d\n", $cpu->carry;       # 0
printf "Z  = %d\n", $cpu->zero;        # 0

# Cross-validate against behavioral simulator
use CodingAdventures::Intel8008Simulator;
my $bsim = CodingAdventures::Intel8008Simulator->new();
my @btraces = $bsim->run($program, 100);
# @traces and @btraces should be identical
```

## Submodules

| Module                                    | Purpose                                 |
|-------------------------------------------|-----------------------------------------|
| `Intel8008GateLevel::Bits`                | int↔bit-array conversion, parity        |
| `Intel8008GateLevel::ALU`                 | All 8 ALU ops + rotates + flag compute  |
| `Intel8008GateLevel::Registers`           | Flip-flop register file                 |
| `Intel8008GateLevel::Decoder`             | Combinational opcode decoder            |
| `Intel8008GateLevel::Stack`               | 8-level push-down PC stack              |

## Dependencies

- `CodingAdventures::LogicGates` — AND, OR, NOT, XOR, XORn, Register, new_flip_flop_state
- `CodingAdventures::Arithmetic` — ripple_carry_adder

## Running tests

```
prove -l -v t/
```

## Installation

```
cpanm .
```
