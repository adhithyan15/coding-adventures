package CodingAdventures::FPGA::CLB;
use strict;
use warnings;
use CodingAdventures::FPGA::Slice;
our $VERSION = '0.01';

# CLB — Configurable Logic Block: 2 slices with carry propagation.
#
# evaluate(\%inputs, $clock, $carry_in) returns (\@outputs, $carry_out).
# inputs keys: s0_a, s0_b, s1_a, s1_b (each an arrayref of bits).
# outputs: [s0_out_a, s0_out_b, s1_out_a, s1_out_b]

sub new {
    my ($class, $row, $col, %opts) = @_;
    return bless {
        slice_0 => CodingAdventures::FPGA::Slice->new(%opts),
        slice_1 => CodingAdventures::FPGA::Slice->new(%opts),
        row     => $row,
        col     => $col,
    }, $class;
}

sub configure {
    my ($self, $config) = @_;
    $self->{slice_0}->configure($config->{slice_0}) if $config->{slice_0};
    $self->{slice_1}->configure($config->{slice_1}) if $config->{slice_1};
    return $self;
}

sub evaluate {
    my ($self, $inputs, $clock, $carry_in) = @_;
    my ($s0_a, $s0_b, $carry_mid) = $self->{slice_0}->evaluate(
        $inputs->{s0_a}, $inputs->{s0_b}, $clock, $carry_in
    );
    my ($s1_a, $s1_b, $carry_out) = $self->{slice_1}->evaluate(
        $inputs->{s1_a}, $inputs->{s1_b}, $clock, $carry_mid
    );
    return ([$s0_a, $s0_b, $s1_a, $s1_b], $carry_out);
}

1;
