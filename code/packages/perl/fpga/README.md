# CodingAdventures::FPGA (Perl)

FPGA (Field-Programmable Gate Array) simulation — programmable hardware from the ground up.

## What is an FPGA?

An FPGA is a chip full of logic gates, memory, and wires — but unlike a CPU or GPU where the circuits are permanently etched in silicon, an FPGA's circuits are **programmable**. You upload a configuration file (called a **bitstream**) and the chip reconfigures itself to implement whatever digital circuit you described.

**The key insight:** a Lookup Table (LUT) storing a truth table is functionally identical to a logic gate — but which gate it implements is determined by the truth table contents, not physical structure. A truth table is a program.

## Package Structure

```
lib/CodingAdventures/
  FPGA.pm                 -- top-level module
  FPGA/
    LUT.pm                -- truth-table lookup element
    Slice.pm              -- 2 LUTs + 2 FFs + carry chain
    CLB.pm                -- 2 slices (Configurable Logic Block)
    SwitchMatrix.pm       -- programmable routing crossbar
    IOBlock.pm            -- external pin interface
    Fabric.pm             -- complete FPGA (CLB grid + routing + I/O)
    Bitstream.pm          -- configuration parser
t/
  00-load.t               -- all modules load cleanly
  01-fpga.t               -- comprehensive tests
Makefile.PL
cpanfile
BUILD
BUILD_windows
```

## Installation

```bash
cpanm --installdeps .
```

## Usage

```perl
use CodingAdventures::FPGA::LUT;
use CodingAdventures::FPGA::Fabric;
use CodingAdventures::FPGA::Bitstream;

# Simple LUT test
my $lut = CodingAdventures::FPGA::LUT->new(2);
$lut->configure([0, 0, 0, 1]);    # AND gate
print $lut->evaluate([1, 1]);     # 1

# Full fabric example
my $f = CodingAdventures::FPGA::Fabric->new(2, 2);
my $bs = CodingAdventures::FPGA::Bitstream->from_map({
    clbs => {
        '0_0' => { slice_0 => { lut_a => [0,0,0,1] } }
    },
    routing => {},
    io => {},
});
$f->load_bitstream($bs);
$f->evaluate(1);
my $s = $f->summary();
printf "CLBs: %d, LUTs: %d\n", $s->{clb_count}, $s->{lut_count};
```

## API Summary

### LUT

```perl
my $lut = CodingAdventures::FPGA::LUT->new($num_inputs);
$lut->configure(\@truth_table);            # 2^n bits
my $out = $lut->evaluate(\@inputs);        # 0 or 1
```

### Slice

```perl
my $s = CodingAdventures::FPGA::Slice->new(%opts);
# opts: lut_inputs, use_ff_a, use_ff_b, carry_enable
$s->configure({ lut_a => \@tt, lut_b => \@tt });
my ($out_a, $out_b, $carry_out) = $s->evaluate(\@a, \@b, $clock, $carry_in);
```

### CLB

```perl
my $clb = CodingAdventures::FPGA::CLB->new($row, $col, %opts);
$clb->configure({ slice_0 => { lut_a => \@tt }, slice_1 => {...} });
my ($outputs_aref, $carry_out) = $clb->evaluate(\%inputs, $clock, $carry_in);
# inputs: { s0_a, s0_b, s1_a, s1_b } each an arrayref of bits
```

### SwitchMatrix

```perl
my $sm = CodingAdventures::FPGA::SwitchMatrix->new($n_in, $n_out);
$sm->configure({ out_0 => 'in_2', out_1 => 'in_0' });
my $result = $sm->route({ in_0 => 1, in_1 => 0, in_2 => 1, in_3 => 0 });
# $result->{out_0} == 1
```

### IOBlock

```perl
my $io = CodingAdventures::FPGA::IOBlock->new($name, $direction);
# direction: 'input', 'output', 'bidirectional'
$io->set_pin($value);            # for input/bidirectional
$io->set_fabric($value);         # for output/bidirectional
$io->set_output_enable($value);  # for bidirectional
my $v = $io->read_fabric();
my $v = $io->read_pin();
```

### Fabric

```perl
my $f = CodingAdventures::FPGA::Fabric->new($rows, $cols, %opts);
$f->load_bitstream($bs);
$f->set_input($pin_name, $value);
my $v = $f->read_output($pin_name);
$f->evaluate($clock);
my $s = $f->summary();
```

### Bitstream

```perl
my $bs = CodingAdventures::FPGA::Bitstream->from_map({
    clbs    => { '0_0' => { slice_0 => { lut_a => [0,0,0,1] } } },
    routing => { '0_0' => { out_0 => 'in_2' } },
    io      => { top_0 => { direction => 'input' } },
});
my $cfg   = $bs->clb_config('0_0');
my $rtr   = $bs->routing_config('0_0');
my $iocfg = $bs->io_config('top_0');
```

## Running Tests

```bash
prove -l -v t/
```
