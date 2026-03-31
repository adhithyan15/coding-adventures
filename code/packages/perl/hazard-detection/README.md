# CodingAdventures::HazardDetection (Perl)

Pipeline hazard detection: RAW data hazards, control hazards (branch misprediction), and structural hazards.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```perl
use CodingAdventures::HazardDetection;

my $det = CodingAdventures::HazardDetection::DataHazardDetector->new();
my $id  = CodingAdventures::HazardDetection::PipelineSlot->new(
    valid       => 1,
    source_regs => [1],
    pc          => 0x8,
);
my $ex = CodingAdventures::HazardDetection::PipelineSlot->new(
    valid    => 1,
    dest_reg => 1,
    mem_read => 1,
    pc       => 0x4,
);
my $r = $det->detect($id, $ex,
    CodingAdventures::HazardDetection::PipelineSlot->empty());
print "$r->{action}\n";   # stall
```

## Dependencies

None.
