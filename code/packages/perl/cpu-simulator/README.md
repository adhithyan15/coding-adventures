# CodingAdventures::CpuSimulator (Perl)

CPU simulator building blocks — Memory, SparseMemory, and RegisterFile.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```perl
use CodingAdventures::CpuSimulator;

my $mem = CodingAdventures::CpuSimulator::Memory->new(65536);
$mem->write_word(0, 0xDEADBEEF);
printf "0x%08X\n", $mem->read_word(0);  # 0xDEADBEEF

my $rf = CodingAdventures::CpuSimulator::RegisterFile->new(16, 32);
$rf->write(0, 42);
print $rf->read(0);  # 42
```

## Description

Provides the fundamental storage primitives for a CPU simulator.
See `code/specs/08-cpu-simulator.md` for the full specification.
