# gpu-core (Perl)

Generic, pluggable accelerator processing element — GPU core simulator.

## Usage

```perl
use CodingAdventures::GpuCore;

my $isa  = CodingAdventures::GpuCore::GenericISA->new;
my $core = CodingAdventures::GpuCore::GPUCore->new(isa => $isa);

$core->load_program(CodingAdventures::GpuCore->saxpy_program(2.0, 3.0, 1.0));
$core->run;
print $core->registers->read(3);   # 7.0
```

## Dependencies

None — this package is self-contained.
