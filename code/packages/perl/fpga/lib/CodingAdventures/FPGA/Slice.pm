package CodingAdventures::FPGA::Slice;

# Slice — the basic compute unit within a CLB.
#
# A slice contains:
#   - 2 LUTs (for combinational logic)
#   - 2 Flip-Flops (for sequential logic / state storage)
#   - 1 Carry Chain (for fast arithmetic)
#
# Each LUT computes a combinational function. The output can either:
#   1. Pass directly to the slice output (combinational path), or
#   2. Pass through a flip-flop first (registered path, captured on clock edge).
#
# Carry Chain
# -----------
# When carry_enable is true:
#   out_a     = lut_a_result XOR carry_in      (sum bit)
#   carry_mid = lut_a_result AND carry_in      (carry out of A)
#   out_b     = lut_b_result XOR carry_mid     (sum bit)
#   carry_out = lut_b_result AND carry_mid     (carry out of B)
#
# Flip-Flop Model
# ---------------
# A D flip-flop captures D on the rising clock edge (clock = 1).
# We model it as: if clock is 1, Q = D; otherwise Q holds its value.

use strict;
use warnings;

use CodingAdventures::FPGA::LUT;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::FPGA::Slice - FPGA slice (2 LUTs + 2 FFs + carry chain)

=head1 SYNOPSIS

    use CodingAdventures::FPGA::Slice;

    my $s = CodingAdventures::FPGA::Slice->new(lut_inputs => 2);
    $s->configure({ lut_a => [0,0,0,1], lut_b => [0,1,1,0] });
    my ($out_a, $out_b, $carry) = $s->evaluate([1,1], [0,1], 0, 0);

=cut

=head2 new(%opts)

Creates a new slice.

Options:
  lut_inputs    (default 4)     — inputs per LUT
  use_ff_a      (default false) — register LUT A output
  use_ff_b      (default false) — register LUT B output
  carry_enable  (default false) — enable carry chain arithmetic

=cut

sub new {
    my ($class, %opts) = @_;
    my $lut_inputs   = $opts{lut_inputs}   // 4;
    my $use_ff_a     = $opts{use_ff_a}     // 0;
    my $use_ff_b     = $opts{use_ff_b}     // 0;
    my $carry_enable = $opts{carry_enable} // 0;

    return bless {
        lut_a        => CodingAdventures::FPGA::LUT->new($lut_inputs),
        lut_b        => CodingAdventures::FPGA::LUT->new($lut_inputs),
        ff_a         => 0,   # flip-flop A state
        ff_b         => 0,   # flip-flop B state
        use_ff_a     => $use_ff_a,
        use_ff_b     => $use_ff_b,
        carry_enable => $carry_enable,
    }, $class;
}

=head2 configure(\%config)

Configures the LUTs. The config hashref may have keys 'lut_a' and/or
'lut_b', each pointing to a truth table arrayref.
Returns self for chaining.

=cut

sub configure {
    my ($self, $config) = @_;
    $self->{lut_a}->configure($config->{lut_a}) if exists $config->{lut_a};
    $self->{lut_b}->configure($config->{lut_b}) if exists $config->{lut_b};
    return $self;
}

=head2 evaluate(\@inputs_a, \@inputs_b, $clock, $carry_in)

Evaluates the slice.

  inputs_a  — arrayref of bits for LUT A
  inputs_b  — arrayref of bits for LUT B
  clock     — clock signal (0 or 1)
  carry_in  — carry input (0 or 1)

Returns ($output_a, $output_b, $carry_out).
Mutates flip-flop state in place.

=cut

sub evaluate {
    my ($self, $inputs_a, $inputs_b, $clock, $carry_in) = @_;

    # Evaluate both LUTs combinationally
    my $lut_a_result = $self->{lut_a}->evaluate($inputs_a);
    my $lut_b_result = $self->{lut_b}->evaluate($inputs_b);

    # Apply carry chain if enabled
    my ($out_a_comb, $carry_mid, $out_b_comb, $carry_out);

    if ($self->{carry_enable}) {
        $out_a_comb = $lut_a_result ^ $carry_in;        # XOR: sum bit
        $carry_mid  = $lut_a_result & $carry_in;        # AND: carry propagate
        $out_b_comb = $lut_b_result ^ $carry_mid;
        $carry_out  = $lut_b_result & $carry_mid;
    } else {
        $out_a_comb = $lut_a_result;
        $out_b_comb = $lut_b_result;
        $carry_out  = 0;
    }

    # Apply flip-flops if enabled (capture on rising edge: clock = 1)
    my ($output_a, $output_b);

    if ($self->{use_ff_a}) {
        $self->{ff_a} = $out_a_comb if $clock == 1;
        $output_a = $self->{ff_a};
    } else {
        $output_a = $out_a_comb;
    }

    if ($self->{use_ff_b}) {
        $self->{ff_b} = $out_b_comb if $clock == 1;
        $output_b = $self->{ff_b};
    } else {
        $output_b = $out_b_comb;
    }

    return ($output_a, $output_b, $carry_out);
}

1;
