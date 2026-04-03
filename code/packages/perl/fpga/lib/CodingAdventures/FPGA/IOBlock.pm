package CodingAdventures::FPGA::IOBlock;

# IOBlock — Input/Output interface between the FPGA fabric and external pins.
#
# I/O blocks sit at the perimeter of the FPGA die and provide the interface
# between the internal logic fabric and the external package pins.
#
#     External Pin
#         │
#     ┌───┴───┐
#     │ Pad   │ ← Physical connection to package pin
#     ├───────┤
#     │ Input │ ← Input buffer (signal from pin into fabric)
#     │ Buffer│
#     ├───────┤
#     │Output │ ← Output buffer (signal from fabric to pin)
#     │Buffer │
#     ├───────┤
#     │Tri-St │ ← Output enable (for bidirectional I/O)
#     │Control│
#     └───────┘
#         │
#     To/From Internal Fabric
#
# Directions:
#   "input"         — external pin → fabric (read from outside world)
#   "output"        — fabric → external pin (drive outside world)
#   "bidirectional" — can switch between input and output (tri-state)

use strict;
use warnings;

our $VERSION = '0.1.0';

my %VALID_DIRECTIONS = (input => 1, output => 1, bidirectional => 1);

=head1 NAME

CodingAdventures::FPGA::IOBlock - External pin interface

=head1 SYNOPSIS

    use CodingAdventures::FPGA::IOBlock;

    my $io = CodingAdventures::FPGA::IOBlock->new('pin_0', 'input');
    $io->set_pin(1);
    my $v = $io->read_fabric();  # 1

=cut

=head2 new($name, $direction)

Creates a new I/O block. direction must be 'input', 'output', or 'bidirectional'.

=cut

sub new {
    my ($class, $name, $direction) = @_;
    die "direction must be 'input', 'output', or 'bidirectional', got: $direction\n"
        unless $VALID_DIRECTIONS{$direction};

    return bless {
        name          => $name,
        direction     => $direction,
        pin_value     => undef,
        fabric_value  => undef,
        # Output blocks are always enabled; others start as input
        output_enable => ($direction eq 'output') ? 1 : 0,
    }, $class;
}

=head2 set_pin($value)

Sets the external pin value (0 or 1). For input or bidirectional blocks.

=cut

sub set_pin {
    my ($self, $value) = @_;
    die "cannot set pin on output-only I/O block\n"
        if $self->{direction} eq 'output';
    die "pin value must be 0 or 1, got: $value\n"
        unless $value == 0 || $value == 1;
    $self->{pin_value} = $value;
    return $self;
}

=head2 set_fabric($value)

Sets the fabric-side value (0 or 1). For output or bidirectional blocks.

=cut

sub set_fabric {
    my ($self, $value) = @_;
    die "cannot set fabric value on input-only I/O block\n"
        if $self->{direction} eq 'input';
    die "fabric value must be 0 or 1, got: $value\n"
        unless $value == 0 || $value == 1;
    $self->{fabric_value} = $value;
    return $self;
}

=head2 set_output_enable($value)

Sets the output enable (0 or 1) for bidirectional blocks only.
OE=1: drive pin from fabric_value; OE=0: read from pin.

=cut

sub set_output_enable {
    my ($self, $value) = @_;
    die "output enable only applies to bidirectional I/O, got: $self->{direction}\n"
        unless $self->{direction} eq 'bidirectional';
    die "output_enable must be 0 or 1\n"
        unless $value == 0 || $value == 1;
    $self->{output_enable} = $value;
    return $self;
}

=head2 read_fabric()

Returns the value visible to the internal fabric:
  input:          pin_value
  output:         fabric_value
  bidirectional OE=0: pin_value
  bidirectional OE=1: fabric_value

=cut

sub read_fabric {
    my ($self) = @_;
    my $dir = $self->{direction};
    if ($dir eq 'input') {
        return $self->{pin_value};
    } elsif ($dir eq 'output') {
        return $self->{fabric_value};
    } else {  # bidirectional
        return $self->{output_enable} == 0
            ? $self->{pin_value}
            : $self->{fabric_value};
    }
}

=head2 read_pin()

Returns the value on the external pin:
  input:          pin_value (driven by external)
  output:         fabric_value (driven by FPGA)
  bidirectional OE=1: fabric_value (FPGA driving)
  bidirectional OE=0: pin_value (external driving)

=cut

sub read_pin {
    my ($self) = @_;
    my $dir = $self->{direction};
    if ($dir eq 'input') {
        return $self->{pin_value};
    } elsif ($dir eq 'output') {
        return $self->{fabric_value};
    } else {  # bidirectional
        return $self->{output_enable} == 1
            ? $self->{fabric_value}
            : $self->{pin_value};
    }
}

1;
