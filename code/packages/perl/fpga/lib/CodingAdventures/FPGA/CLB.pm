package CodingAdventures::FPGA::CLB;

# CLB — Configurable Logic Block, the primary logic resource in an FPGA.
#
# A CLB contains two Slices, giving it a total of:
#   - 4 LUTs  (2 per slice)
#   - 4 Flip-Flops (2 per slice)
#   - 2 Carry Chains (1 per slice)
#
# CLBs are arranged in a grid across the FPGA, connected by the
# programmable routing network (switch matrices).
#
#     ┌─────────────────────────────────────┐
#     │           CLB (row, col)            │
#     │  ┌──────────┐    ┌──────────┐      │
#     │  │ Slice 0  │    │ Slice 1  │      │
#     │  │ LUT_A    │    │ LUT_A    │      │
#     │  │ LUT_B    │    │ LUT_B    │      │
#     │  │ FF_A     │    │ FF_A     │      │
#     │  │ FF_B     │    │ FF_B     │      │
#     │  │ Carry ───┼────┤→ Carry   │      │
#     │  └──────────┘    └──────────┘      │
#     └─────────────────────────────────────┘
#
# The carry chain propagates from Slice 0 to Slice 1.

use strict;
use warnings;

use CodingAdventures::FPGA::Slice;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::FPGA::CLB - Configurable Logic Block (2 slices)

=head1 SYNOPSIS

    use CodingAdventures::FPGA::CLB;

    my $clb = CodingAdventures::FPGA::CLB->new(0, 0);
    $clb->configure({
        slice_0 => { lut_a => [0,0,0,1] },
        slice_1 => { lut_a => [0,1,1,0] },
    });
    my ($outputs, $carry) = $clb->evaluate(\%inputs, 0, 0);

=cut

=head2 new($row, $col, %opts)

Creates a new CLB at the given grid position.

Options:
  lut_inputs  (default 4) — inputs per LUT

=cut

sub new {
    my ($class, $row, $col, %opts) = @_;
    my $lut_inputs = $opts{lut_inputs} // 4;

    return bless {
        slice_0 => CodingAdventures::FPGA::Slice->new(lut_inputs => $lut_inputs),
        slice_1 => CodingAdventures::FPGA::Slice->new(lut_inputs => $lut_inputs),
        row     => $row,
        col     => $col,
    }, $class;
}

=head2 configure(\%config)

Configures both slices. The config hashref may have keys 'slice_0' and/or
'slice_1', each being a config hashref for Slice->configure().
Returns self for chaining.

=cut

sub configure {
    my ($self, $config) = @_;
    $self->{slice_0}->configure($config->{slice_0}) if exists $config->{slice_0};
    $self->{slice_1}->configure($config->{slice_1}) if exists $config->{slice_1};
    return $self;
}

=head2 evaluate(\%inputs, $clock, $carry_in)

Evaluates the CLB. inputs hashref keys:
  s0_a  — arrayref of bits for Slice 0 LUT A
  s0_b  — arrayref of bits for Slice 0 LUT B
  s1_a  — arrayref of bits for Slice 1 LUT A
  s1_b  — arrayref of bits for Slice 1 LUT B

Returns (\@outputs, $carry_out) where outputs = [s0_a, s0_b, s1_a, s1_b].

=cut

sub evaluate {
    my ($self, $inputs, $clock, $carry_in) = @_;

    # Evaluate Slice 0 — carry_in from external
    my ($s0_a, $s0_b, $carry_mid) = $self->{slice_0}->evaluate(
        $inputs->{s0_a}, $inputs->{s0_b}, $clock, $carry_in);

    # Evaluate Slice 1 — carry_mid from Slice 0
    my ($s1_a, $s1_b, $carry_out) = $self->{slice_1}->evaluate(
        $inputs->{s1_a}, $inputs->{s1_b}, $clock, $carry_mid);

    return ([$s0_a, $s0_b, $s1_a, $s1_b], $carry_out);
}

1;
