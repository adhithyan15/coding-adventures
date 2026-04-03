# CodingAdventures::Intel4004GateLevel

A gate-level simulator for the **Intel 4004** — every arithmetic and logical
operation routes through actual gate functions from
`CodingAdventures::LogicGates` and `CodingAdventures::Arithmetic`.

This is a Perl port of the Elixir `CodingAdventures.Intel4004GateLevel` package.

## Why gate-level vs behavioral?

The behavioral simulator (`CodingAdventures::Intel4004Simulator`) executes
instructions directly with host-language integers. This gate-level simulator
routes **every computation** through the gate abstraction layer:

| Operation        | Implementation                                      |
|-----------------|-----------------------------------------------------|
| ADD Rn          | `ripple_carry_adder(\@a_bits, \@b_bits, $carry_in)` |
| SUB Rn          | `gate_add(A, NOT(Rn), borrow_in)` via NOT gates     |
| INC Rn          | 4-bit half-adder chain                              |
| ISZ Rn          | 4-bit half-adder chain                              |
| IAC             | `gate_add(A, 0, 1)` — carry-in adds 1              |
| CMC             | `NOT(carry)` using the NOT gate                    |
| CMA             | 4× NOT gates on each accumulator bit               |
| PC increment    | 12-bit half-adder chain                             |
| Registers       | D flip-flop states via `Register()` function        |

This lets you:

1. **Count gates**: how many AND/OR/NOT ops does `ADD R3` actually require?
2. **Trace signals**: follow a single bit from register R3 through the ALU
3. **Understand timing**: a ripple-carry add takes 4 gate delays (one per bit)
4. **Appreciate constraints**: 2,300 transistors is incredibly few

## Gate budget

```perl
my $gc = CodingAdventures::Intel4004GateLevel::gate_count();
# {
#   alu       => 80,   # 4 full adders x ~20 gates each
#   registers => 256,  # 16 regs x 4 bits x 4 gates per flip-flop
#   acc       => 16,
#   carry     => 4,
#   decoder   => 120,  # AND/OR/NOT tree for opcode decode
#   pc        => 96,   # 12 half-adders for PC increment
#   stack     => 144,  # 3 x 12 bits x 4 gates
#   total     => 716,  # close to 4004's ~786 estimated gates
# }
```

## Flip-flop register model

Every register (accumulator, R0–R15, PC, stack slots) is stored as an array
of `new_flip_flop_state()` hashrefs from `CodingAdventures::LogicGates`.

Writes use the **two-phase clock** protocol:

```
Phase 1 (clock=0): data captured into master latch
Phase 2 (clock=1): master latches to slave (data appears at output)
```

This matches how real D flip-flops work in hardware.

## Usage

```perl
use CodingAdventures::Intel4004GateLevel;

my $cpu = CodingAdventures::Intel4004GateLevel->new();

# 1 + 2 = 3 — routes through the ripple-carry adder
my $traces = $cpu->run([0xD1, 0xB0, 0xD2, 0x80, 0x01]);
printf "Result: %d\n", $cpu->accumulator;  # 3

# Gate usage
my $gc = CodingAdventures::Intel4004GateLevel::gate_count();
printf "Total gates: %d\n", $gc->{total};  # 716
```

## Dependencies

- `CodingAdventures::LogicGates` — AND, OR, NOT, XOR, Register, new_flip_flop_state
- `CodingAdventures::Arithmetic` — half_adder, ripple_carry_adder

## Running tests

```
prove -l -v t/
```

## Installation

```
cpanm .
```
