# CodingAdventures::ARM1Gatelevel (Perl)

ARM1 gate-level processor simulation in pure Perl.

Where the behavioral simulator computes `$result = $a + $b` directly, this
simulator routes every bit through logic gate function calls from
`CodingAdventures::LogicGates`. Addition uses a ripple-carry adder from
`CodingAdventures::Arithmetic` (~160 gate calls for 32-bit). The barrel
shifter uses a 5-level Mux2 tree (~480 gate calls per shift).

## Dependencies

- `CodingAdventures::LogicGates` — AND, OR, NOT, XOR, XNOR primitives
- `CodingAdventures::Arithmetic` — `ripple_carry_adder`
- `CodingAdventures::ARM1Simulator` — instruction decode, memory, encoding

## Installation

```bash
cpanm --installdeps .
```

## Running Tests

```bash
prove -l -v t/
```

## Usage

```perl
use CodingAdventures::ARM1Gatelevel;
use CodingAdventures::ARM1Simulator;

my $cpu = CodingAdventures::ARM1Gatelevel->new(4096);
$cpu->load_instructions(0, [
    CodingAdventures::ARM1Simulator::encode_mov_imm(
        CodingAdventures::ARM1Gatelevel::COND_AL, 0, 42),
    CodingAdventures::ARM1Simulator::encode_halt(),
]);
$cpu->run(100);
print $cpu->read_register(0), "\n";  # 42
print $cpu->{gate_ops}, "\n";        # cumulative gate calls
```

## Gate Count

| Operation            | Approximate gate calls |
|----------------------|------------------------|
| Condition evaluation | 1                      |
| Logical op (AND/ORR) | 32                     |
| Arithmetic (ADD/SUB) | ~160 (ripple-carry)    |
| Barrel shift         | ~480 (5-level Mux2)    |
| Data processing instr| ~200 (combined)        |
| Load/store           | 50 (estimated)         |
| Block transfer       | 100 (estimated)        |

The `gate_ops` field on the CPU object accumulates these counts.
