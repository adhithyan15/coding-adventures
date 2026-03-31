# CodingAdventures::Intel4004Simulator

A complete behavioral simulator for the **Intel 4004** — the world's first
commercial single-chip microprocessor — implemented in Perl.

This is a Perl port of the Elixir `CodingAdventures.Intel4004Simulator` package.

## Historical context

The Intel 4004 was released in November 1971, designed by Federico Faggin, Ted Hoff,
and Stanley Mazor for the Busicom 141-PF calculator. It contained just **2,300
transistors** and ran at **740 kHz** — roughly one million times slower than a
modern CPU core. Yet it proved that a general-purpose processor could be built on a
single chip, launching the microprocessor revolution.

## Architecture

| Feature         | Value                                    |
|----------------|------------------------------------------|
| Data width     | 4 bits (values 0–15)                     |
| Instruction    | 8 bits (some are 2 bytes)                |
| Registers      | 16 × 4-bit (R0–R15), 8 pairs (P0–P7)   |
| Accumulator    | 4-bit (A)                                |
| Carry flag     | 1 bit                                    |
| Program counter| 12 bits (4096 bytes ROM)                 |
| Stack          | 3-level hardware (wraps on overflow)     |
| RAM            | 4 banks × 4 regs × 16 chars + 4 status  |
| Clock          | 740 kHz (original hardware)              |

## Instruction set (46 + HLT)

| Opcode     | Mnemonic    | Description                               |
|-----------|-------------|-------------------------------------------|
| 0x00      | NOP         | No operation                              |
| 0x01      | HLT         | Halt (simulator-only)                     |
| 0x1_      | JCN c,a *   | Conditional jump                          |
| 0x2_ even | FIM Pp,d *  | Fetch immediate to register pair          |
| 0x2_ odd  | SRC Pp      | Send register control (set RAM address)   |
| 0x3_ even | FIN Pp      | Fetch indirect from ROM via P0            |
| 0x3_ odd  | JIN Pp      | Jump indirect via register pair           |
| 0x4_      | JUN a *     | Unconditional jump (12-bit)               |
| 0x5_      | JMS a *     | Jump to subroutine                        |
| 0x6_      | INC Rn      | Increment register (mod 16, no carry)     |
| 0x7_      | ISZ Rn,a *  | Increment register, skip if zero          |
| 0x8_      | ADD Rn      | Add register + carry to accumulator       |
| 0x9_      | SUB Rn      | Subtract register (complement-add)        |
| 0xA_      | LD Rn       | Load register into accumulator            |
| 0xB_      | XCH Rn      | Exchange accumulator and register         |
| 0xC_      | BBL n       | Branch back (return) and load immediate   |
| 0xD_      | LDM n       | Load immediate nibble                     |
| 0xE0–0xEF | I/O ops     | RAM read/write, ROM port                  |
| 0xF0–0xFD | Accum ops   | CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC,  |
|           |             | DAC, TCS, STC, DAA, KBP, DCL             |

`*` = 2-byte instruction

## Usage

```perl
use CodingAdventures::Intel4004Simulator;

my $cpu = CodingAdventures::Intel4004Simulator->new();

# Compute 1 + 2 = 3
# LDM 1, XCH R0, LDM 2, ADD R0, HLT
my $traces = $cpu->run([0xD1, 0xB0, 0xD2, 0x80, 0x01]);

printf "Result: %d\n", $cpu->accumulator;  # 3

# Inspect execution trace
for my $t (@$traces) {
    printf "addr=0x%03X  %-12s  A: %d->%d  C: %d->%d\n",
        $t->{address}, $t->{mnemonic},
        $t->{accumulator_before}, $t->{accumulator_after},
        $t->{carry_before}, $t->{carry_after};
}
```

## Key design notes

### SUB uses complement-add

The 4004 lacks a dedicated subtractor. Instead, SUB computes:

```
A = A + (~Rn) + borrow_in
```

where `borrow_in = carry ? 0 : 1`. After subtraction, carry=1 means "no borrow"
(result ≥ 0). This is the MCS-4 carry convention — the inverse of what you might
expect.

### BCD arithmetic

The 4004 was designed for calculators using decimal (BCD) arithmetic. The `DAA`
(Decimal Adjust Accumulator) instruction corrects the result of a binary addition
to produce a valid BCD nibble (0–9). Combined with `TCS` (Transfer Carry Subtract),
multi-digit decimal arithmetic is possible.

### 3-level hardware stack

The stack has exactly 3 slots. A 4th push silently overwrites the oldest entry.
This is a genuine hardware limitation of the 4004.

## Running tests

```
prove -l -v t/
```

## Installation

```
cpanm .
```
