package CodingAdventures::FPGA::LUT;
use strict;
use warnings;
our $VERSION = '0.01';

# LUT — Lookup Table: the fundamental logic element of an FPGA.
#
# A LUT stores a truth table in SRAM. N inputs form a binary address
# (MSB-first) that selects one of 2^N stored output values.
#
# Example: 2-input AND gate:
#   Address 00 → 0, Address 01 → 0, Address 10 → 0, Address 11 → 1
#   truth_table = [0, 0, 0, 1]

sub new {
    my ($class, $num_inputs) = @_;
    $num_inputs //= 4;
    die "num_inputs must be > 0" unless $num_inputs > 0;
    my $size = 1 << $num_inputs;
    return bless {
        num_inputs  => $num_inputs,
        truth_table => [(0) x $size],
    }, $class;
}

sub configure {
    my ($self, $truth_table) = @_;
    my $expected = 1 << $self->{num_inputs};
    die "truth table must have $expected entries, got " . scalar(@$truth_table)
        unless @$truth_table == $expected;
    $self->{truth_table} = [@$truth_table];
    return $self;
}

# Evaluates the LUT. inputs: arrayref of bits, MSB-first.
sub evaluate {
    my ($self, $inputs) = @_;
    die "expected $self->{num_inputs} inputs, got " . scalar(@$inputs)
        unless @$inputs == $self->{num_inputs};
    my $index = 0;
    for my $bit (@$inputs) {
        $index = ($index << 1) | $bit;
    }
    return $self->{truth_table}[$index];
}

1;
