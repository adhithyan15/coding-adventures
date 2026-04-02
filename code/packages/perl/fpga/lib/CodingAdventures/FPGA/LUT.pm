package CodingAdventures::FPGA::LUT;

# LUT — Lookup Table, the fundamental logic element of an FPGA.
#
# A Lookup Table (LUT) is a small SRAM-based truth table that can implement
# ANY Boolean function of N inputs. In modern FPGAs, the most common size
# is a 4-input LUT (LUT4), which has 2^4 = 16 bits of configuration memory.
#
# How a LUT Works
# ---------------
# The key insight is that any Boolean function can be represented by its
# truth table. We store the 2^N output values in SRAM. When inputs arrive,
# they form an N-bit address that selects one of the stored values.
#
# Example: implementing AND(a, b) with a 2-input LUT:
#
#     Address (inputs) │ Stored Value (output)
#     ─────────────────┼──────────────────────
#        00 (a=0,b=0)  │         0
#        01 (a=0,b=1)  │         0
#        10 (a=1,b=0)  │         0
#        11 (a=1,b=1)  │         1
#
# The LUT stores [0, 0, 0, 1] — the truth table for AND.

use strict;
use warnings;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::FPGA::LUT - Lookup Table (truth table logic element)

=head1 SYNOPSIS

    use CodingAdventures::FPGA::LUT;

    my $lut = CodingAdventures::FPGA::LUT->new(2);
    $lut->configure([0, 0, 0, 1]);    # AND gate
    my $out = $lut->evaluate([1, 1]); # 1

=cut

=head2 new($num_inputs)

Creates a new unconfigured LUT with the given number of inputs.
The truth table defaults to all zeros.

=cut

sub new {
    my ($class, $num_inputs) = @_;
    die "num_inputs must be a positive integer\n"
        unless defined($num_inputs) && $num_inputs > 0;

    my $table_size = 1 << $num_inputs;
    my @truth_table = (0) x $table_size;

    return bless {
        num_inputs  => $num_inputs,
        truth_table => \@truth_table,
    }, $class;
}

=head2 configure(\@truth_table)

Configures the LUT with a truth table.
The array must have exactly 2^num_inputs entries (each 0 or 1).
Returns self for chaining.

=cut

sub configure {
    my ($self, $truth_table) = @_;
    my $expected = 1 << $self->{num_inputs};

    die sprintf(
        "truth table must have %d entries for %d-input LUT, got %d\n",
        $expected, $self->{num_inputs}, scalar(@$truth_table)
    ) unless scalar(@$truth_table) == $expected;

    for my $bit (@$truth_table) {
        die "truth table entries must be 0 or 1, got $bit\n"
            unless $bit == 0 || $bit == 1;
    }

    $self->{truth_table} = [@$truth_table];
    return $self;
}

=head2 evaluate(\@inputs)

Evaluates the LUT for the given inputs.
Inputs are treated as MSB-first: the first element is the most
significant bit of the address into the truth table.
Returns 0 or 1.

=cut

sub evaluate {
    my ($self, $inputs) = @_;
    my $n = $self->{num_inputs};

    die sprintf("expected %d inputs, got %d\n", $n, scalar(@$inputs))
        unless scalar(@$inputs) == $n;

    # Convert input bits to an address index (MSB first)
    my $index = 0;
    for my $bit (@$inputs) {
        die "inputs must be 0 or 1, got $bit\n"
            unless $bit == 0 || $bit == 1;
        $index = ($index << 1) | $bit;
    }

    return $self->{truth_table}[$index];
}

1;
