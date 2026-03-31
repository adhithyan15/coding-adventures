# CodingAdventures::ClrSimulator

A Pure Perl simulator for the .NET CLR Intermediate Language (IL/CIL/MSIL).

## Usage

```perl
use CodingAdventures::ClrSimulator;

my $sim = CodingAdventures::ClrSimulator->new();

# Assemble: x = 1 + 2
my $code = CodingAdventures::ClrSimulator::assemble([
    CodingAdventures::ClrSimulator::encode_ldc_i4(1),
    CodingAdventures::ClrSimulator::encode_ldc_i4(2),
    [CodingAdventures::ClrSimulator::ADD],
    [CodingAdventures::ClrSimulator::STLOC_0],
    [CodingAdventures::ClrSimulator::RET],
]);

$sim->load($code);
my $traces = $sim->run();
print $sim->{locals}[0];  # 3
```

## Installation

```sh
cpanm --installdeps .
```

## Testing

```sh
prove -l -v t/
```
