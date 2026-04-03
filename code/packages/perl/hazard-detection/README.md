# CodingAdventures::HazardDetection (Perl)

Pipeline hazard detection for pipelined CPUs.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```perl
use CodingAdventures::HazardDetection;

my $Slot = 'CodingAdventures::HazardDetection::PipelineSlot';
my $det  = CodingAdventures::HazardDetection::DataHazardDetector->new();

my $ex_slot = $Slot->new(valid => 1, dest_reg => 1, dest_value => 42);
my $id_slot = $Slot->new(valid => 1, source_regs => [1]);
my $result  = $det->detect($id_slot, $ex_slot, $Slot->empty());

print $result->{action};           # forward_ex
print $result->{forwarded_value};  # 42
```

## Description

Provides hazard detection for data hazards (RAW), control hazards (branch
misprediction), and structural hazards (resource conflicts).

See `code/specs/D03-hazard-detection.md` for the full specification.
