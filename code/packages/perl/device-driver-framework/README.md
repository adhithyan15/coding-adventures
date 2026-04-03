# device-driver-framework (Perl)

Device driver abstraction layer for the coding-adventures simulated OS.

## What It Does

- `SimulatedDisk` — In-memory block device with fixed-size sectors
- `SimulatedSerial` — Character device with TX/RX byte-stream buffers
- `SimulatedNIC` — Network device with TX/RX packet queues
- `Registry` — Manages device registration by name and major:minor number

## Device Families

| Type      | Example         | API         |
|-----------|-----------------|-------------|
| Block     | Disk, SSD       | read_block / write_block |
| Character | Serial, keyboard| read / write (byte stream) |
| Network   | Ethernet NIC    | send / receive (packets) |

## Usage

```perl
use CodingAdventures::DeviceDriverFramework;

# Block device
my $disk = CodingAdventures::DeviceDriverFramework::SimulatedDisk->new(
    block_size => 512, total_blocks => 4
);
$disk->initialize();
$disk->open();
my $data = "\x00" x 512;
$disk->write_block(0, $data);
my ($st, $self, $got) = $disk->read_block(0);

# Character device
my $serial = CodingAdventures::DeviceDriverFramework::SimulatedSerial->new();
$serial->initialize();
$serial->open();
$serial->inject_rx("hello\n");
my ($st2, $self2, $bytes) = $serial->read(6);

# Network device
my $nic = CodingAdventures::DeviceDriverFramework::SimulatedNIC->new();
$nic->initialize();
$nic->open();
$nic->send([0xAA, 0xBB]);
my ($st3, $self3, $pkt) = $nic->receive();

# Registry
my $reg = CodingAdventures::DeviceDriverFramework::Registry->new();
$reg->register($disk);
my ($st4, $dev) = $reg->get('disk0');
```
