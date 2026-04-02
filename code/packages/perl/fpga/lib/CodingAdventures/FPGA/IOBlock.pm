package CodingAdventures::FPGA::IOBlock;
use strict;
use warnings;
our $VERSION = '0.01';

# IOBlock — I/O interface between FPGA fabric and external pins.
#
# direction: 'input', 'output', or 'bidirectional'
# Input:  set_pin($v)    → read_fabric()
# Output: set_fabric($v) → read_pin()

sub new {
    my ($class, $name, $direction) = @_;
    die "direction must be 'input', 'output', or 'bidirectional'"
        unless $direction =~ /^(input|output|bidirectional)$/;
    return bless {
        name          => $name,
        direction     => $direction,
        pin_value     => undef,
        fabric_value  => undef,
        output_enable => ($direction eq 'output') ? 1 : 0,
    }, $class;
}

sub set_pin {
    my ($self, $v) = @_;
    die "cannot set pin on output-only I/O block" if $self->{direction} eq 'output';
    $self->{pin_value} = $v;
}

sub set_fabric {
    my ($self, $v) = @_;
    die "cannot set fabric on input-only I/O block" if $self->{direction} eq 'input';
    $self->{fabric_value} = $v;
}

sub set_output_enable {
    my ($self, $v) = @_;
    $self->{output_enable} = $v;
}

sub read_fabric {
    my ($self) = @_;
    return $self->{fabric_value} if $self->{direction} eq 'output';
    if ($self->{direction} eq 'bidirectional') {
        return $self->{output_enable} ? $self->{fabric_value} : $self->{pin_value};
    }
    return $self->{pin_value};
}

sub read_pin {
    my ($self) = @_;
    return $self->{pin_value} if $self->{direction} eq 'input';
    if ($self->{direction} eq 'bidirectional') {
        return $self->{output_enable} ? $self->{fabric_value} : $self->{pin_value};
    }
    return $self->{fabric_value};
}

1;
