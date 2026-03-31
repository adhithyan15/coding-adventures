# CodingAdventures::CpuSimulator (Perl)

CPU simulator building blocks: byte-addressable Memory, SparseMemory, and RegisterFile.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```perl
use CodingAdventures::CpuSimulator;

my $m = CodingAdventures::CpuSimulator::Memory->new(65536);
$m->write_word(0, 0xDEADBEEF);
printf "0x%08X\n", $m->read_word(0);  # 0xDEADBEEF

my $rf = CodingAdventures::CpuSimulator::RegisterFile->new(16, 32);
$rf->write(1, 0xCAFE);
printf "%d\n", $rf->read(1);  # 51966
```

## Dependencies

None.
