package CodingAdventures::FPGA::SwitchMatrix;

# SwitchMatrix — programmable routing crossbar in an FPGA.
#
# A switch matrix connects CLBs, I/O blocks, and other resources together.
# It sits at the intersection of horizontal and vertical routing channels.
#
# In a real FPGA, routing consumes 60-80% of chip area. The switch matrix
# contains programmable pass transistors that can be turned on or off to
# create connections between wires.
#
# Architecture:
#   Input 0 ──┐
#   Input 1 ──┤──── Switch ──── Output 0
#   Input 2 ──┤     Matrix ──── Output 1
#   Input 3 ──┘            ──── Output 2
#
# Configuration: a hash from output names to input names.
#   { "out_0" => "in_2", "out_1" => "in_0" }
# Unconnected outputs produce undef (high-Z).

use strict;
use warnings;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::FPGA::SwitchMatrix - Programmable routing crossbar

=head1 SYNOPSIS

    use CodingAdventures::FPGA::SwitchMatrix;

    my $sm = CodingAdventures::FPGA::SwitchMatrix->new(4, 4);
    $sm->configure({ out_0 => 'in_2', out_1 => 'in_0' });
    my $signals = $sm->route({ in_0 => 1, in_1 => 0, in_2 => 1, in_3 => 0 });
    # $signals->{out_0} == 1

=cut

=head2 new($num_inputs, $num_outputs)

Creates a new switch matrix. Port names are automatically generated
as "in_0", "in_1", ... and "out_0", "out_1", ...

=cut

sub new {
    my ($class, $num_inputs, $num_outputs) = @_;
    die "num_inputs must be positive\n"  unless $num_inputs  > 0;
    die "num_outputs must be positive\n" unless $num_outputs > 0;

    my @input_names  = map { "in_$_"  } 0 .. ($num_inputs  - 1);
    my @output_names = map { "out_$_" } 0 .. ($num_outputs - 1);

    my %input_set  = map { $_ => 1 } @input_names;
    my %output_set = map { $_ => 1 } @output_names;

    return bless {
        num_inputs   => $num_inputs,
        num_outputs  => $num_outputs,
        connections  => {},
        input_names  => \@input_names,
        output_names => \@output_names,
        _input_set   => \%input_set,
        _output_set  => \%output_set,
    }, $class;
}

=head2 configure(\%connections)

Configures the routing. connections maps output port names to input
port names. Raises an error if any port name is invalid.
Returns self for chaining.

=cut

sub configure {
    my ($self, $connections) = @_;

    for my $out_name (keys %$connections) {
        my $in_name = $connections->{$out_name};
        die "invalid output port: $out_name\n"
            unless $self->{_output_set}{$out_name};
        die "invalid input port: $in_name\n"
            unless $self->{_input_set}{$in_name};
    }

    $self->{connections} = { %$connections };
    return $self;
}

=head2 route(\%input_signals)

Routes signals through the switch matrix. Returns a hashref mapping
output port names to signal values. Unconnected outputs have undef values.

=cut

sub route {
    my ($self, $input_signals) = @_;
    my %result;

    for my $out_name (@{ $self->{output_names} }) {
        my $in_name = $self->{connections}{$out_name};
        if (defined $in_name) {
            $result{$out_name} = $input_signals->{$in_name};
        } else {
            $result{$out_name} = undef;  # high-Z / unconnected
        }
    }

    return \%result;
}

1;
