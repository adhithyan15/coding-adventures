package CodingAdventures::FPGA::Fabric;

# Fabric — the complete FPGA top-level module.
#
# The fabric is the complete FPGA device, tying together all the components:
#   - A grid of CLBs (Configurable Logic Blocks) — the logic resources
#   - Switch matrices for routing signals between CLBs
#   - I/O blocks around the perimeter — external interface
#
# Configuration Flow:
#   1. Create a fabric with specified grid dimensions.
#   2. Load a bitstream (configuration hashref).
#   3. Set input pin values (external signals).
#   4. Evaluate (propagate signals through the fabric).
#   5. Read output pin values (results).
#
# Grid Layout:
#   IO   IO   IO   IO
#   IO  CLB  CLB   IO
#   IO  CLB  CLB   IO
#   IO   IO   IO   IO
#
# Perimeter I/O blocks:
#   top_0 ... top_{cols-1}       — input
#   bottom_0 ... bottom_{cols-1} — output
#   left_0 ... left_{rows-1}     — input
#   right_0 ... right_{rows-1}   — output

use strict;
use warnings;

use CodingAdventures::FPGA::CLB;
use CodingAdventures::FPGA::SwitchMatrix;
use CodingAdventures::FPGA::IOBlock;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::FPGA::Fabric - Complete FPGA top-level (CLB grid + routing + I/O)

=head1 SYNOPSIS

    use CodingAdventures::FPGA::Fabric;
    use CodingAdventures::FPGA::Bitstream;

    my $f = CodingAdventures::FPGA::Fabric->new(2, 2);
    $f->load_bitstream($bitstream);
    $f->set_input('top_0', 1);
    $f->evaluate(1);
    my $v = $f->read_output('bottom_0');

=cut

=head2 new($rows, $cols, %opts)

Creates a new FPGA fabric.

Options:
  lut_inputs   (default 4) — inputs per LUT
  switch_size  (default 8) — ports per switch matrix side

=cut

sub new {
    my ($class, $rows, $cols, %opts) = @_;
    die "rows must be positive\n"  unless $rows > 0;
    die "cols must be positive\n"  unless $cols > 0;

    my $lut_inputs  = $opts{lut_inputs}  // 4;
    my $switch_size = $opts{switch_size} // 8;

    # Create CLB grid (keyed by "row_col")
    my %clbs;
    my %switch_matrices;
    for my $r (0 .. $rows - 1) {
        for my $c (0 .. $cols - 1) {
            my $key = "${r}_${c}";
            $clbs{$key} = CodingAdventures::FPGA::CLB->new($r, $c, lut_inputs => $lut_inputs);
            $switch_matrices{$key} = CodingAdventures::FPGA::SwitchMatrix->new($switch_size, $switch_size);
        }
    }

    # Create perimeter I/O blocks
    my %io_blocks;
    for my $c (0 .. $cols - 1) {
        $io_blocks{"top_$c"}    = CodingAdventures::FPGA::IOBlock->new("top_$c",    'input');
        $io_blocks{"bottom_$c"} = CodingAdventures::FPGA::IOBlock->new("bottom_$c", 'output');
    }
    for my $r (0 .. $rows - 1) {
        $io_blocks{"left_$r"}  = CodingAdventures::FPGA::IOBlock->new("left_$r",  'input');
        $io_blocks{"right_$r"} = CodingAdventures::FPGA::IOBlock->new("right_$r", 'output');
    }

    return bless {
        rows            => $rows,
        cols            => $cols,
        clbs            => \%clbs,
        switch_matrices => \%switch_matrices,
        io_blocks       => \%io_blocks,
        lut_inputs      => $lut_inputs,
    }, $class;
}

=head2 load_bitstream($bitstream)

Loads a Bitstream configuration into the fabric.
Applies CLB, routing, and I/O configurations.
Returns self for chaining.

=cut

sub load_bitstream {
    my ($self, $bitstream) = @_;

    # Apply CLB configurations
    for my $key (keys %{ $self->{clbs} }) {
        my $config = $bitstream->clb_config($key);
        next unless defined $config;
        $self->{clbs}{$key}->configure(_parse_clb_config($config));
    }

    # Apply routing configurations
    for my $key (keys %{ $self->{switch_matrices} }) {
        my $config = $bitstream->routing_config($key);
        next unless defined $config;
        $self->{switch_matrices}{$key}->configure($config);
    }

    # Apply I/O configurations
    for my $name (keys %{ $self->{io_blocks} }) {
        my $config = $bitstream->io_config($name);
        next unless defined $config;
        my $direction = $config->{direction} // 'input';
        $self->{io_blocks}{$name} = CodingAdventures::FPGA::IOBlock->new($name, $direction);
    }

    return $self;
}

# Parse CLB config from bitstream format (all string keys, pass through)
sub _parse_clb_config {
    my ($config) = @_;
    my %result;
    $result{slice_0} = _parse_slice_config($config->{slice_0}) if exists $config->{slice_0};
    $result{slice_1} = _parse_slice_config($config->{slice_1}) if exists $config->{slice_1};
    return \%result;
}

sub _parse_slice_config {
    my ($config) = @_;
    my %result;
    $result{lut_a} = $config->{lut_a} if exists $config->{lut_a};
    $result{lut_b} = $config->{lut_b} if exists $config->{lut_b};
    return \%result;
}

=head2 set_input($pin_name, $value)

Sets an input pin value (0 or 1) on the fabric.
Returns self for chaining.

=cut

sub set_input {
    my ($self, $pin_name, $value) = @_;
    die "unknown pin: $pin_name\n" unless exists $self->{io_blocks}{$pin_name};
    $self->{io_blocks}{$pin_name}->set_pin($value);
    return $self;
}

=head2 read_output($pin_name)

Reads the value on an external output pin.
Returns 0, 1, or undef.

=cut

sub read_output {
    my ($self, $pin_name) = @_;
    die "unknown pin: $pin_name\n" unless exists $self->{io_blocks}{$pin_name};
    return $self->{io_blocks}{$pin_name}->read_pin();
}

=head2 evaluate($clock)

Evaluates one clock cycle of the FPGA fabric (simplified single-pass model).
Returns self for chaining.

=cut

sub evaluate {
    my ($self, $clock) = @_;
    my @zero_inputs = (0) x $self->{lut_inputs};

    for my $clb (values %{ $self->{clbs} }) {
        my $inputs = {
            s0_a => \@zero_inputs,
            s0_b => \@zero_inputs,
            s1_a => \@zero_inputs,
            s1_b => \@zero_inputs,
        };
        $clb->evaluate($inputs, $clock, 0);
    }

    return $self;
}

=head2 summary()

Returns a hashref with resource counts.

=cut

sub summary {
    my ($self) = @_;
    my $clb_count = scalar keys %{ $self->{clbs} };
    my $sm_count  = scalar keys %{ $self->{switch_matrices} };
    my $io_count  = scalar keys %{ $self->{io_blocks} };

    return {
        rows                => $self->{rows},
        cols                => $self->{cols},
        clb_count           => $clb_count,
        lut_count           => $clb_count * 4,
        ff_count            => $clb_count * 4,
        switch_matrix_count => $sm_count,
        io_block_count      => $io_count,
        lut_inputs          => $self->{lut_inputs},
    };
}

1;
