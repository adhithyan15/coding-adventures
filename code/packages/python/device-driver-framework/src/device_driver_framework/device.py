"""Core device abstractions: DeviceType, Device base class, and the three
device family classes (CharacterDevice, BlockDevice, NetworkDevice).

==========================================================================
Why Three Device Families?
==========================================================================

Not all hardware behaves the same way. A keyboard produces one byte at a time
whenever the user presses a key. A disk reads and writes fixed-size chunks
called "blocks" or "sectors." A network card sends and receives variable-length
packets. Trying to force all three into a single interface would be awkward,
so operating systems classify devices into families:

  Device Type       Data Model            Examples
  --------------------------------------------------------------------------
  Character         Stream of bytes       Keyboard, serial port, display
                    (one at a time)       terminal, mouse

  Block             Fixed-size chunks     Hard disk, SSD, USB drive,
                    (random access)       CD-ROM

  Network           Variable-length       Ethernet NIC, WiFi adapter
                    packets

Each family gets an interface that matches how the hardware naturally operates.

==========================================================================
Major and Minor Numbers
==========================================================================

In Unix, every device is identified by two numbers:

  Major number: identifies the DRIVER (which software module handles this)
  Minor number: identifies the INSTANCE (which specific device of that type)

  Example:
    Major 3 = disk driver
    Minor 0 = first disk (disk0)
    Minor 1 = second disk (disk1)

This lets the kernel route I/O requests to the correct driver without knowing
anything about the hardware itself. The kernel just looks up the major number
to find the driver, and passes the minor number so the driver knows which
physical device to talk to.
"""

from enum import IntEnum


# =========================================================================
# DeviceType Enum
# =========================================================================
# We use IntEnum so device types can be compared with integers and used
# as dictionary keys. The values (0, 1, 2) are arbitrary but conventional.

class DeviceType(IntEnum):
    """Classification of device into one of three families.

    Each family has a different interface reflecting how the underlying
    hardware naturally operates:
      - CHARACTER (0): byte streams (read/write one byte at a time)
      - BLOCK (1): fixed-size chunks (random access by block number)
      - NETWORK (2): variable-length packets (send/receive)
    """

    CHARACTER = 0
    BLOCK = 1
    NETWORK = 2


# =========================================================================
# Device Base Class
# =========================================================================
# Every device in the system -- regardless of whether it's a keyboard, disk,
# or network card -- shares these core attributes. This is the "common
# denominator" that lets the DeviceRegistry store and manage all devices
# uniformly.

class Device:
    """Base class for all devices in the system.

    Attributes:
        name: Human-readable identifier, e.g. "disk0", "keyboard0".
              Used for lookup in the DeviceRegistry.
        device_type: Which family this device belongs to (CHARACTER,
                     BLOCK, or NETWORK).
        major: Driver identifier. All devices handled by the same driver
               share a major number.
        minor: Instance identifier within the driver. First disk = 0,
               second disk = 1, etc.
        interrupt_number: Which interrupt this device raises when it needs
                         attention. -1 if the device does not use interrupts.
                         For example, a keyboard raises interrupt 33 when a
                         key is pressed; a disk raises interrupt 34 when a
                         block read completes.
        initialized: Whether init() has been called. Prevents
                     double-initialization and ensures the device is ready
                     before use.
    """

    def __init__(
        self,
        name: str,
        device_type: DeviceType,
        major: int,
        minor: int,
        interrupt_number: int = -1,
    ) -> None:
        self.name = name
        self.device_type = device_type
        self.major = major
        self.minor = minor
        self.interrupt_number = interrupt_number
        self.initialized = False

    def init(self) -> None:
        """Initialize the device. Called once at boot time.

        Subclasses override this to perform hardware-specific setup (clear
        buffers, allocate storage, etc.). The base implementation just sets
        the initialized flag.
        """
        self.initialized = True

    def __repr__(self) -> str:
        return (
            f"{self.__class__.__name__}("
            f"name={self.name!r}, "
            f"type={self.device_type.name}, "
            f"major={self.major}, minor={self.minor}, "
            f"irq={self.interrupt_number})"
        )


# =========================================================================
# CharacterDevice
# =========================================================================
# Character devices produce or consume a stream of bytes, one at a time.
# You cannot "seek" to a specific position -- data arrives when it arrives
# (like a keyboard) or is consumed in order (like a display).
#
# Real-world examples:
#   /dev/ttyS0  -- serial port (read/write bytes over a cable)
#   /dev/stdin  -- keyboard input
#   /dev/null   -- discards everything written to it, reads return EOF
#   /dev/random -- generates random bytes

class CharacterDevice(Device):
    """A byte-stream device (keyboard, serial port, display terminal).

    Character devices are sequential: you read bytes in order, and you
    cannot "seek" to byte 47. Data arrives when it arrives (for input
    devices like keyboards) or is consumed in order (for output devices
    like displays).

    Subclasses must implement read() and write().
    """

    def __init__(
        self,
        name: str,
        major: int,
        minor: int,
        interrupt_number: int = -1,
    ) -> None:
        super().__init__(name, DeviceType.CHARACTER, major, minor, interrupt_number)

    def read(self, count: int) -> bytes:
        """Read up to `count` bytes from the device.

        Returns the bytes actually read. May return fewer than `count` if
        not enough data is available. Returns b"" if no data is available
        (non-blocking).

        Why return fewer bytes? Because the device might not have as much
        data as you asked for. A keyboard might have only 3 keystrokes
        buffered when you asked for 10.
        """
        raise NotImplementedError

    def write(self, data: bytes) -> int:
        """Write bytes to the device.

        Returns the number of bytes actually written, or -1 on error.
        For output devices (display), this renders the data.
        For input-only devices (keyboard), this returns -1.
        """
        raise NotImplementedError


# =========================================================================
# BlockDevice
# =========================================================================
# Block devices read and write fixed-size chunks called "blocks" or
# "sectors." The standard block size is 512 bytes -- a legacy from the
# IBM PC/AT (1984) that persists to this day. Modern disks use 4096-byte
# sectors, but 512 is simpler and more traditional for education.
#
# The key difference from character devices: block devices support
# RANDOM ACCESS. You can read block 0, then block 9999, then block 42,
# in any order. This is essential for filesystems, which store files
# scattered across the disk surface.
#
# Real-world examples:
#   /dev/sda   -- first SCSI/SATA disk
#   /dev/nvme0 -- first NVMe solid-state drive
#   /dev/loop0 -- loopback device (a file pretending to be a disk)

class BlockDevice(Device):
    """A fixed-size block device (disk, SSD, USB drive).

    Block devices are random-access: you can read any block in any order.
    Every block is the same size (default 512 bytes). This is essential
    for filesystems, which store files scattered across the disk.

    Subclasses must implement read_block() and write_block().

    Attributes:
        block_size: Number of bytes per block (default 512).
        total_blocks: How many blocks this device has. A 1 MB disk
                      with 512-byte blocks has 2048 blocks.
    """

    def __init__(
        self,
        name: str,
        major: int,
        minor: int,
        block_size: int = 512,
        total_blocks: int = 0,
        interrupt_number: int = -1,
    ) -> None:
        super().__init__(name, DeviceType.BLOCK, major, minor, interrupt_number)
        self.block_size = block_size
        self.total_blocks = total_blocks

    def read_block(self, block_num: int) -> bytes:
        """Read exactly `block_size` bytes from block `block_num`.

        Why whole blocks? Disks physically read whole sectors at a time.
        Even if you only want 1 byte, the disk reads 512. The OS caches
        the extra bytes for later. This is why filesystems exist -- to
        manage partial-block reads/writes efficiently.

        Raises:
            ValueError: If block_num is out of range.
        """
        raise NotImplementedError

    def write_block(self, block_num: int, data: bytes) -> None:
        """Write exactly `block_size` bytes to block `block_num`.

        The data must be exactly `block_size` bytes long. Writing a
        partial block is not allowed -- the hardware reads and writes
        whole sectors atomically.

        Raises:
            ValueError: If block_num is out of range or data is wrong size.
        """
        raise NotImplementedError


# =========================================================================
# NetworkDevice
# =========================================================================
# Network devices deal in packets -- discrete messages with headers,
# addresses, and payloads. You do not read "byte 5 of the network" --
# you send and receive complete packets.
#
# Every network card has a MAC address -- a 6-byte unique identifier,
# like a mailing address. In real hardware, this is burned into the NIC
# at the factory. In simulation, we assign it at creation time.
#
# Real-world examples:
#   eth0  -- first Ethernet interface
#   wlan0 -- first WiFi interface
#   lo    -- loopback (the computer talking to itself)

class NetworkDevice(Device):
    """A packet-oriented network device (Ethernet NIC, WiFi adapter).

    Network devices send and receive discrete packets rather than streams
    of bytes or fixed-size blocks. Each packet is a self-contained message
    with its own headers and payload.

    Subclasses must implement send_packet(), receive_packet(), and has_packet().

    Attributes:
        mac_address: A 6-byte Media Access Control address. This is the
                     network card's unique identifier on the local network,
                     like a mailing address for packets.
    """

    def __init__(
        self,
        name: str,
        major: int,
        minor: int,
        mac_address: bytes,
        interrupt_number: int = -1,
    ) -> None:
        super().__init__(name, DeviceType.NETWORK, major, minor, interrupt_number)
        self.mac_address = mac_address

    def send_packet(self, data: bytes) -> int:
        """Send a packet over the network.

        Returns the number of bytes sent, or -1 on error.
        In simulation, this pushes the packet onto a shared "wire"
        (an in-memory queue) where other NICs can receive it.
        """
        raise NotImplementedError

    def receive_packet(self) -> bytes | None:
        """Receive the next packet from the network.

        Returns the packet data, or None if no packet is available.
        This is non-blocking: if nothing has arrived, it returns
        immediately with None rather than waiting.
        """
        raise NotImplementedError

    def has_packet(self) -> bool:
        """Check whether there is a packet waiting to be received.

        Returns True if at least one packet is queued.
        """
        raise NotImplementedError
