"""SimulatedDisk -- an in-memory block storage device.

==========================================================================
What Is a Disk?
==========================================================================

A disk is a random-access storage device organized into fixed-size chunks
called "blocks" or "sectors." The traditional sector size is 512 bytes,
a legacy from the IBM PC/AT in 1984 that persists to this day (though
modern drives use 4096-byte "advanced format" sectors).

A disk is fundamentally different from memory (RAM):
  - RAM is volatile: loses data when power is off
  - Disk is persistent: retains data across reboots
  - RAM is byte-addressable: read any single byte
  - Disk is block-addressable: must read/write whole sectors

==========================================================================
How SimulatedDisk Works
==========================================================================

Our simulated disk is just a big bytearray in memory. It pretends to be
a real disk by enforcing block-aligned access:

  Physical disk:              SimulatedDisk:
  ┌──────────────────┐        ┌──────────────────┐
  │ Block 0 (512 B)  │        │ storage[0:512]   │
  ├──────────────────┤        ├──────────────────┤
  │ Block 1 (512 B)  │        │ storage[512:1024]│
  ├──────────────────┤        ├──────────────────┤
  │ Block 2 (512 B)  │        │ storage[1024:1536]│
  ├──────────────────┤        ├──────────────────┤
  │ ...              │        │ ...              │
  └──────────────────┘        └──────────────────┘

Default configuration:
  - block_size = 512 bytes
  - total_blocks = 2048 (= 1 MB total)
  - major = 3 (disk driver)
  - minor = 0 (first disk)
  - interrupt_number = 34 (disk I/O complete)
"""

from device_driver_framework.device import BlockDevice


class SimulatedDisk(BlockDevice):
    """A simulated disk backed by an in-memory byte array.

    This is the "hard drive" for our simulated computer. It stores data
    in a Python bytearray, but enforces the same block-aligned access
    patterns that real disk hardware uses.

    Args:
        name: Device name (default "disk0").
        minor: Minor number for this disk instance (default 0).
        block_size: Bytes per block (default 512).
        total_blocks: Number of blocks (default 2048 = 1 MB).
        interrupt_number: IRQ for I/O completion (default 34).
    """

    def __init__(
        self,
        name: str = "disk0",
        minor: int = 0,
        block_size: int = 512,
        total_blocks: int = 2048,
        interrupt_number: int = 34,
    ) -> None:
        super().__init__(
            name=name,
            major=3,  # Major 3 = disk driver (from the spec)
            minor=minor,
            block_size=block_size,
            total_blocks=total_blocks,
            interrupt_number=interrupt_number,
        )
        # The backing store: a bytearray of total_blocks * block_size bytes.
        # Initially all zeros, just like a freshly formatted disk.
        self._storage = bytearray(total_blocks * block_size)

    def init(self) -> None:
        """Initialize the disk.

        For a simulated disk, initialization means zeroing out the storage.
        On a real disk, this might involve reading the partition table,
        spinning up the platters, or calibrating the read/write heads.
        """
        self._storage = bytearray(self.total_blocks * self.block_size)
        self.initialized = True

    def read_block(self, block_num: int) -> bytes:
        """Read one block from the disk.

        The math is straightforward:
          offset = block_num * block_size
          data = storage[offset : offset + block_size]

        On a real disk, this would involve:
          1. Moving the read/write head to the correct track (seek)
          2. Waiting for the correct sector to rotate under the head
          3. Reading the magnetic flux patterns and decoding them
          4. Raising interrupt 34 to signal completion

        Our simulation skips steps 1-3 (instant access) but the interface
        is identical.

        Args:
            block_num: Which block to read (0-indexed).

        Returns:
            A bytes object of exactly block_size bytes.

        Raises:
            ValueError: If block_num is out of range.
        """
        if block_num < 0 or block_num >= self.total_blocks:
            raise ValueError(
                f"Block number {block_num} out of range "
                f"(0..{self.total_blocks - 1})"
            )
        offset = block_num * self.block_size
        return bytes(self._storage[offset : offset + self.block_size])

    def write_block(self, block_num: int, data: bytes) -> None:
        """Write one block to the disk.

        The data must be exactly block_size bytes. This constraint mirrors
        real disk hardware, which always writes complete sectors. If you
        want to change just 1 byte in a sector, you must:
          1. Read the entire sector into memory
          2. Modify the byte
          3. Write the entire sector back

        This read-modify-write pattern is so common that most disk
        controllers handle it in hardware.

        Args:
            block_num: Which block to write (0-indexed).
            data: Exactly block_size bytes to write.

        Raises:
            ValueError: If block_num is out of range or data is wrong size.
        """
        if block_num < 0 or block_num >= self.total_blocks:
            raise ValueError(
                f"Block number {block_num} out of range "
                f"(0..{self.total_blocks - 1})"
            )
        if len(data) != self.block_size:
            raise ValueError(
                f"Data must be exactly {self.block_size} bytes, "
                f"got {len(data)}"
            )
        offset = block_num * self.block_size
        self._storage[offset : offset + self.block_size] = data

    @property
    def storage(self) -> bytearray:
        """Direct access to the backing store (for testing/debugging)."""
        return self._storage
