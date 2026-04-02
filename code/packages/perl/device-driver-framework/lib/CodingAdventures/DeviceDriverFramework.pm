package CodingAdventures::DeviceDriverFramework;

# ============================================================================
# CodingAdventures::DeviceDriverFramework — Device Driver Abstraction
# ============================================================================
#
# A device driver provides a uniform interface over diverse hardware.
# Programs say "read 512 bytes from block 7" — the driver translates that
# into whatever specific commands the hardware needs.
#
# ## Analogy: The Universal Remote
#
# A universal remote has one "Volume Up" button that works on your Samsung TV,
# Sony soundbar, and LG projector.  Device drivers are the universal remote
# for your OS: one API (read/write/ioctl) works for keyboards, disks, and NICs.
#
# ## Three Device Families
#
#   Character devices — byte streams (keyboard, serial, display)
#   Block devices     — fixed-size chunks (disk, SSD)
#   Network devices   — packets (Ethernet NIC)
#
# ## Driver Lifecycle
#
#   Register → Initialize → Open → Read/Write/Ioctl → Close
#
# ## Major and Minor Numbers
#
#   major — device type (disk=3, serial=4, NIC=5, ...)
#   minor — instance (disk0=0, disk1=1, ...)
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use constant TYPE_CHARACTER => 0;
use constant TYPE_BLOCK     => 1;
use constant TYPE_NETWORK   => 2;

use constant DEFAULT_BLOCK_SIZE   => 512;
use constant DEFAULT_TOTAL_BLOCKS => 2048;
use constant DEFAULT_BUFFER_SIZE  => 4096;

# ============================================================================
# SimulatedDisk — Block Device
# ============================================================================
#
# An in-memory block device.  Divided into fixed-size sectors.
#
#   ┌──────────┬──────────┬──────────┬─────┐
#   │  Block 0 │  Block 1 │  Block 2 │ ... │   total_blocks sectors
#   │ 512 bytes│ 512 bytes│ 512 bytes│     │
#   └──────────┴──────────┴──────────┴─────┘
#
# read_block(n)        → bytes at offset n × block_size
# write_block(n, data) → replaces bytes at that offset

package CodingAdventures::DeviceDriverFramework::SimulatedDisk;

sub new {
    my ($class, %opts) = @_;
    my $bs    = $opts{block_size}   // CodingAdventures::DeviceDriverFramework::DEFAULT_BLOCK_SIZE;
    my $total = $opts{total_blocks} // CodingAdventures::DeviceDriverFramework::DEFAULT_TOTAL_BLOCKS;
    return bless {
        name             => $opts{name}             // 'disk0',
        device_type      => CodingAdventures::DeviceDriverFramework::TYPE_BLOCK,
        major            => $opts{major}            // 3,
        minor            => $opts{minor}            // 0,
        interrupt_number => $opts{interrupt_number} // 34,
        initialized      => 0,
        block_size        => $bs,
        total_blocks      => $total,
        storage           => "\x00" x ($bs * $total),
        open_count        => 0,
    }, $class;
}

sub initialize {
    my ($self) = @_;
    $self->{initialized} = 1;
    return ('ok', $self);
}

sub open {
    my ($self) = @_;
    return ('not_initialized', $self) unless $self->{initialized};
    $self->{open_count}++;
    return ('ok', $self);
}

sub close {
    my ($self) = @_;
    return ('not_open', $self) if $self->{open_count} <= 0;
    $self->{open_count}--;
    return ('ok', $self);
}

sub read_block {
    my ($self, $block_num) = @_;
    return ('out_of_bounds', $self, undef)
        if $block_num < 0 || $block_num >= $self->{total_blocks};
    my $offset = $block_num * $self->{block_size};
    my $data   = substr($self->{storage}, $offset, $self->{block_size});
    return ('ok', $self, $data);
}

sub write_block {
    my ($self, $block_num, $data) = @_;
    return ('out_of_bounds', $self)
        if $block_num < 0 || $block_num >= $self->{total_blocks};
    return ('wrong_size', $self)
        if length($data) != $self->{block_size};
    my $offset = $block_num * $self->{block_size};
    substr($self->{storage}, $offset, $self->{block_size}) = $data;
    return ('ok', $self);
}

sub ioctl {
    my ($self, $cmd, $arg) = @_;
    return ('ok', $self->{block_size})   if $cmd eq 'get_block_size';
    return ('ok', $self->{total_blocks}) if $cmd eq 'get_total_blocks';
    return ('unsupported', undef);
}

# ============================================================================
# SimulatedSerial — Character Device
# ============================================================================
#
# A character device produces or consumes a stream of bytes.  This simulates
# a UART serial port with separate TX (transmit) and RX (receive) buffers.
#
#   TX buffer — bytes written by the process (sent to hardware)
#   RX buffer — bytes produced by hardware (read by process)

package CodingAdventures::DeviceDriverFramework::SimulatedSerial;

sub new {
    my ($class, %opts) = @_;
    return bless {
        name             => $opts{name}             // 'serial0',
        device_type      => CodingAdventures::DeviceDriverFramework::TYPE_CHARACTER,
        major            => $opts{major}            // 4,
        minor            => $opts{minor}            // 0,
        interrupt_number => $opts{interrupt_number} // 33,
        initialized      => 0,
        baud_rate        => $opts{baud_rate}        // 9600,
        tx_buffer        => '',
        rx_buffer        => '',
        open_count       => 0,
    }, $class;
}

sub initialize {
    my ($self) = @_;
    $self->{initialized} = 1;
    return ('ok', $self);
}

sub open {
    my ($self) = @_;
    return ('not_initialized', $self) unless $self->{initialized};
    $self->{open_count}++;
    return ('ok', $self);
}

sub close {
    my ($self) = @_;
    return ('not_open', $self) if $self->{open_count} <= 0;
    $self->{open_count}--;
    return ('ok', $self);
}

sub write {
    my ($self, $data) = @_;
    $self->{tx_buffer} .= $data;
    return ('ok', $self, length($data));
}

sub read {
    my ($self, $max_bytes) = @_;
    return ('empty', $self, '') unless length($self->{rx_buffer});
    my $n    = $max_bytes < length($self->{rx_buffer}) ? $max_bytes : length($self->{rx_buffer});
    my $data = substr($self->{rx_buffer}, 0, $n, '');
    return ('ok', $self, $data);
}

sub inject_rx {
    my ($self, $data) = @_;
    $self->{rx_buffer} .= $data;
    return $self;
}

sub tx_contents { $_[0]->{tx_buffer} }

sub ioctl {
    my ($self, $cmd, $arg) = @_;
    return ('ok', $self->{baud_rate}) if $cmd eq 'get_baud_rate';
    if ($cmd eq 'set_baud_rate') {
        $self->{baud_rate} = $arg;
        return ('ok', $self);
    }
    return ('unsupported', undef);
}

# ============================================================================
# SimulatedNIC — Network Device
# ============================================================================
#
# A network device sends and receives packets.
# tx_queue — packets sent by the process
# rx_queue — packets received from the (simulated) network

package CodingAdventures::DeviceDriverFramework::SimulatedNIC;

sub new {
    my ($class, %opts) = @_;
    return bless {
        name             => $opts{name}             // 'eth0',
        device_type      => CodingAdventures::DeviceDriverFramework::TYPE_NETWORK,
        major            => $opts{major}            // 5,
        minor            => $opts{minor}            // 0,
        interrupt_number => $opts{interrupt_number} // 35,
        initialized      => 0,
        mac_address      => $opts{mac_address}      // [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
        tx_queue         => [],
        rx_queue         => [],
        open_count       => 0,
    }, $class;
}

sub initialize {
    my ($self) = @_;
    $self->{initialized} = 1;
    return ('ok', $self);
}

sub open {
    my ($self) = @_;
    return ('not_initialized', $self) unless $self->{initialized};
    $self->{open_count}++;
    return ('ok', $self);
}

sub close {
    my ($self) = @_;
    return ('not_open', $self) if $self->{open_count} <= 0;
    $self->{open_count}--;
    return ('ok', $self);
}

sub send {
    my ($self, $packet) = @_;
    push @{ $self->{tx_queue} }, $packet;
    return ('ok', $self);
}

sub receive {
    my ($self) = @_;
    return ('empty', $self, undef) unless @{ $self->{rx_queue} };
    my $pkt = shift @{ $self->{rx_queue} };
    return ('ok', $self, $pkt);
}

sub inject_rx {
    my ($self, $packet) = @_;
    push @{ $self->{rx_queue} }, $packet;
    return $self;
}

sub ioctl {
    my ($self, $cmd, $arg) = @_;
    return ('ok', $self->{mac_address}) if $cmd eq 'get_mac';
    return ('unsupported', undef);
}

# ============================================================================
# Registry — Device Registry
# ============================================================================

package CodingAdventures::DeviceDriverFramework::Registry;

sub new {
    my ($class) = @_;
    return bless {
        devices        => {},
        by_major_minor => {},
    }, $class;
}

sub register {
    my ($self, $device) = @_;
    my $key = "$device->{major}:$device->{minor}";
    return 'already_registered'
        if $self->{devices}{$device->{name}} || $self->{by_major_minor}{$key};
    $self->{devices}{$device->{name}} = $device;
    $self->{by_major_minor}{$key}     = $device->{name};
    return 'ok';
}

sub get {
    my ($self, $name) = @_;
    return ('ok', $self->{devices}{$name}) if $self->{devices}{$name};
    return ('not_found', undef);
}

sub get_by_major_minor {
    my ($self, $major, $minor) = @_;
    my $key  = "$major:$minor";
    my $name = $self->{by_major_minor}{$key};
    return ('not_found', undef) unless $name;
    return ('ok', $self->{devices}{$name});
}

sub update {
    my ($self, $name, $device) = @_;
    return 'not_found' unless $self->{devices}{$name};
    $self->{devices}{$name} = $device;
    return 'ok';
}

sub unregister {
    my ($self, $name) = @_;
    return 'not_found' unless $self->{devices}{$name};
    my $device = $self->{devices}{$name};
    my $key = "$device->{major}:$device->{minor}";
    delete $self->{devices}{$name};
    delete $self->{by_major_minor}{$key};
    return 'ok';
}

sub list {
    my ($self) = @_;
    return sort keys %{ $self->{devices} };
}

# ============================================================================
# Top-level package
# ============================================================================

package CodingAdventures::DeviceDriverFramework;

=head1 NAME

CodingAdventures::DeviceDriverFramework - Device driver abstraction framework

=head1 SYNOPSIS

  use CodingAdventures::DeviceDriverFramework;

  my $disk = CodingAdventures::DeviceDriverFramework::SimulatedDisk->new(
      block_size => 512, total_blocks => 4
  );
  $disk->initialize();
  $disk->open();
  my $data = "\x00" x 512;
  $disk->write_block(0, $data);
  my ($st, $self, $got) = $disk->read_block(0);

=cut

1;
