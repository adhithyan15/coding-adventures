package CodingAdventures::FPGA::Slice;
use strict;
use warnings;
use CodingAdventures::FPGA::LUT;
our $VERSION = '0.01';

# Slice — 2 LUTs + 2 flip-flops + carry chain.
#
# evaluate(\@a, \@b, $clock, $carry_in) returns ($out_a, $out_b, $carry_out).
# Flip-flops capture on clock=1 (rising edge).
#
# Carry chain (when carry_enable=1):
#   out_a = LUT_A XOR carry_in;  carry_mid = LUT_A AND carry_in
#   out_b = LUT_B XOR carry_mid; carry_out = LUT_B AND carry_mid

sub new {
    my ($class, %opts) = @_;
    my $n = $opts{lut_inputs} // 4;
    return bless {
        lut_a        => CodingAdventures::FPGA::LUT->new($n),
        lut_b        => CodingAdventures::FPGA::LUT->new($n),
        ff_a         => 0,
        ff_b         => 0,
        use_ff_a     => $opts{use_ff_a}     // 0,
        use_ff_b     => $opts{use_ff_b}     // 0,
        carry_enable => $opts{carry_enable} // 0,
    }, $class;
}

sub configure {
    my ($self, $config) = @_;
    $self->{lut_a}->configure($config->{lut_a}) if $config->{lut_a};
    $self->{lut_b}->configure($config->{lut_b}) if $config->{lut_b};
    return $self;
}

sub evaluate {
    my ($self, $inputs_a, $inputs_b, $clock, $carry_in) = @_;
    my $lut_a_result = $self->{lut_a}->evaluate($inputs_a);
    my $lut_b_result = $self->{lut_b}->evaluate($inputs_b);

    my ($out_a_comb, $out_b_comb, $carry_out);

    if ($self->{carry_enable}) {
        $out_a_comb = $lut_a_result ^ $carry_in;
        my $carry_mid = $lut_a_result & $carry_in;
        $out_b_comb = $lut_b_result ^ $carry_mid;
        $carry_out  = $lut_b_result & $carry_mid;
    } else {
        $out_a_comb = $lut_a_result;
        $out_b_comb = $lut_b_result;
        $carry_out  = 0;
    }

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
