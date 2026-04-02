package CodingAdventures::FPGA::Bitstream;

# Bitstream — FPGA configuration data.
#
# A bitstream is the configuration file that programs an FPGA. It contains
# all the information needed to configure every LUT, flip-flop, routing
# switch, and I/O block.
#
# Our Format (Perl hashref):
#
#   {
#     clbs => {
#       "0_0" => {
#         slice_0 => {
#           lut_a        => [0, 0, 0, 1],
#           lut_b        => [0, 1, 1, 0],
#           use_ff_a     => 0,
#           use_ff_b     => 1,
#           carry_enable => 0,
#         },
#         slice_1 => { ... },
#       },
#     },
#     routing => {
#       "0_0" => { out_0 => "in_2", ... },
#     },
#     io => {
#       pin_0 => { direction => "input" },
#       pin_1 => { direction => "output" },
#     },
#   }

use strict;
use warnings;

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::FPGA::Bitstream - FPGA configuration parser

=head1 SYNOPSIS

    use CodingAdventures::FPGA::Bitstream;

    my $bs = CodingAdventures::FPGA::Bitstream->from_map({
        clbs    => { '0_0' => { slice_0 => { lut_a => [0,0,0,1] } } },
        routing => {},
        io      => {},
    });
    my $cfg = $bs->clb_config('0_0');

=cut

=head2 from_map(\%config)

Parses a bitstream from a plain Perl hashref.
Missing top-level keys default to empty hashrefs.

=cut

sub from_map {
    my ($class, $config) = @_;
    die "config must be a hashref\n" unless ref($config) eq 'HASH';

    return bless {
        clb_configs     => $config->{clbs}    // {},
        routing_configs => $config->{routing} // {},
        io_configs      => $config->{io}      // {},
    }, $class;
}

=head2 clb_config($key)

Returns the CLB configuration for the given position key (e.g., "0_0").
Returns undef if not found.

=cut

sub clb_config {
    my ($self, $key) = @_;
    return $self->{clb_configs}{$key};
}

=head2 routing_config($key)

Returns the routing configuration for the given position key.
Returns undef if not found.

=cut

sub routing_config {
    my ($self, $key) = @_;
    return $self->{routing_configs}{$key};
}

=head2 io_config($pin_name)

Returns the I/O configuration for the given pin name.
Returns undef if not found.

=cut

sub io_config {
    my ($self, $pin_name) = @_;
    return $self->{io_configs}{$pin_name};
}

1;
