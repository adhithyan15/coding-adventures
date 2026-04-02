package CodingAdventures::FPGA::SwitchMatrix;
use strict;
use warnings;
our $VERSION = '0.01';

# SwitchMatrix — programmable routing crossbar.
#
# new($num_inputs, $num_outputs)
# configure(\%connections)  — {out_name => in_name}
# route(\%input_signals)    — {in_name => value} → {out_name => value}
# Unconnected outputs are undef.

sub new {
    my ($class, $num_inputs, $num_outputs) = @_;
    die "num_inputs must be > 0"  unless $num_inputs  > 0;
    die "num_outputs must be > 0" unless $num_outputs > 0;

    my @input_names  = map { "in_$_"  } (0 .. $num_inputs  - 1);
    my @output_names = map { "out_$_" } (0 .. $num_outputs - 1);

    return bless {
        num_inputs   => $num_inputs,
        num_outputs  => $num_outputs,
        connections  => {},
        input_names  => \@input_names,
        output_names => \@output_names,
    }, $class;
}

sub configure {
    my ($self, $connections) = @_;
    my %in_set  = map { $_ => 1 } @{$self->{input_names}};
    my %out_set = map { $_ => 1 } @{$self->{output_names}};

    for my $out_name (keys %$connections) {
        my $in_name = $connections->{$out_name};
        die "invalid output port: $out_name" unless $out_set{$out_name};
        die "invalid input port: $in_name"   unless $in_set{$in_name};
    }
    $self->{connections} = {%$connections};
    return $self;
}

sub route {
    my ($self, $input_signals) = @_;
    my %output;
    for my $out_name (@{$self->{output_names}}) {
        my $in_name = $self->{connections}{$out_name};
        $output{$out_name} = defined $in_name ? $input_signals->{$in_name} : undef;
    }
    return \%output;
}

1;
