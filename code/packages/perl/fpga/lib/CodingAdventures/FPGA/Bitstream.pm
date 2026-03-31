package CodingAdventures::FPGA::Bitstream;
use strict;
use warnings;
our $VERSION = '0.01';

# Bitstream — FPGA configuration data structure.
#
# from_map(\%config):
#   config keys: clbs, routing, io
#   clbs:    { "row,col" => { slice_0 => {...}, slice_1 => {...} } }
#   routing: { "row,col" => { "out_0" => "in_1", ... } }
#   io:      { pin_name => { direction => ... } }

sub from_map {
    my ($class, $config) = @_;
    return bless {
        _clbs    => $config->{clbs}    // {},
        _routing => $config->{routing} // {},
        _io      => $config->{io}      // {},
    }, $class;
}

sub clb_config     { return $_[0]->{_clbs}{$_[1]}    }
sub routing_config { return $_[0]->{_routing}{$_[1]} }
sub io_config      { return $_[0]->{_io}{$_[1]}      }

1;
