# CodingAdventures::ARM1Simulator (Perl)

ARM1 (ARMv1) behavioral instruction set simulator — the complete ARMv1 instruction set in pure Perl.

## Installation

```bash
cpanm --installdeps .
```

## Usage

```perl
use CodingAdventures::ARM1Simulator;

my $cpu = CodingAdventures::ARM1Simulator->new(4096);
$cpu->load_instructions([
    $cpu->encode_mov_imm(CodingAdventures::ARM1Simulator::COND_AL, 0, 42),
    $cpu->encode_halt(),
]);
$cpu->run(100);
print $cpu->read_register(0);  # 42
```

## Running Tests

```bash
prove -l -v t/
```
