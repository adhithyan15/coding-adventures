package bootloader

// =========================================================================
// DiskImage -- simulated persistent storage
// =========================================================================
//
// === What is a disk image? ===
//
// Real computers have hard drives, SSDs, or floppy disks. Our simulated
// computer has a DiskImage -- a byte slice that acts as persistent storage.
// The disk image is pre-loaded with the kernel binary and (optionally) user
// program binaries before the system powers on.
//
// The disk is not directly addressable by the CPU. Instead, it is
// memory-mapped at DiskMemoryMapBase (0x10000000), and the bootloader copies
// from that region into RAM. This is a simplification of how real disk
// controllers work (which use DMA or I/O ports), but it demonstrates the
// concept clearly.
//
// === Disk Layout ===
//
//	Offset 0x00000000: Boot sector (512 bytes, unused in our system)
//	Offset 0x00000200: Reserved
//	Offset 0x00080000: Kernel binary (default location)
//	Offset 0x00100000: User program area
//
// === Analogy ===
//
// Think of the disk image as a warehouse. Items (binaries) are stored at
// known locations on shelves (offsets). The bootloader is the forklift
// operator who reads the manifest (boot protocol), goes to the right shelf
// (kernel offset), picks up the package (kernel binary), and delivers it
// to the loading dock (kernel RAM at 0x00020000).

const (
	// DiskBootSectorOffset is the conventional offset of the boot sector.
	// In our system, this is unused -- the BIOS handles boot directly.
	DiskBootSectorOffset = 0x00000000

	// DiskBootSectorSize is the standard boot sector size (512 bytes).
	DiskBootSectorSize = 512

	// DiskKernelOffset is the default offset where the kernel binary lives.
	// This is 512 KB into the disk, giving plenty of room for metadata.
	DiskKernelOffset = 0x00080000

	// DiskUserProgramBase is the default starting offset for user programs.
	DiskUserProgramBase = 0x00100000

	// DefaultDiskSize is the default disk image size (2 MB).
	// This is large enough for a kernel plus several user programs.
	DefaultDiskSize = 2 * 1024 * 1024
)

// DiskImage simulates persistent storage (hard drive / SSD).
// Pre-loaded with kernel and user program binaries before boot.
//
// The underlying representation is a flat byte slice. Reads and writes
// use byte offsets within the disk, just like real disk I/O.
type DiskImage struct {
	data []byte
}

// NewDiskImage creates an empty disk image of the given size in bytes.
// All bytes are initialized to zero, simulating a blank/formatted disk.
func NewDiskImage(sizeBytes int) *DiskImage {
	return &DiskImage{
		data: make([]byte, sizeBytes),
	}
}

// LoadKernel writes a kernel binary to the conventional disk offset
// (DiskKernelOffset = 0x00080000). This is the location the bootloader
// expects to find the kernel.
//
// The kernel binary is a raw sequence of RISC-V machine code instructions,
// followed by any data sections (like the "Hello World\n" string).
func (d *DiskImage) LoadKernel(kernelBinary []byte) {
	d.LoadAt(DiskKernelOffset, kernelBinary)
}

// LoadUserProgram writes a user program binary at a specified disk offset.
// This allows placing multiple programs on the disk at different locations.
func (d *DiskImage) LoadUserProgram(programBinary []byte, offset int) {
	d.LoadAt(offset, programBinary)
}

// LoadAt writes raw bytes at a specific offset within the disk image.
// Panics if the data would exceed the disk size.
func (d *DiskImage) LoadAt(offset int, data []byte) {
	if offset+len(data) > len(d.data) {
		panic("DiskImage: data exceeds disk size")
	}
	copy(d.data[offset:], data)
}

// ReadWord reads a 32-bit little-endian word at the given disk offset.
// Returns 0 for out-of-bounds reads.
func (d *DiskImage) ReadWord(offset int) uint32 {
	if offset < 0 || offset+4 > len(d.data) {
		return 0
	}
	return uint32(d.data[offset]) |
		uint32(d.data[offset+1])<<8 |
		uint32(d.data[offset+2])<<16 |
		uint32(d.data[offset+3])<<24
}

// ReadByteAt reads a single byte at the given disk offset.
// Returns 0 for out-of-bounds reads.
func (d *DiskImage) ReadByteAt(offset int) byte {
	if offset < 0 || offset >= len(d.data) {
		return 0
	}
	return d.data[offset]
}

// Data returns the raw byte slice for memory-mapping into the address space.
// The system board loads this into the SparseMemory at DiskMemoryMapBase.
func (d *DiskImage) Data() []byte {
	return d.data
}

// Size returns the total size of the disk image in bytes.
func (d *DiskImage) Size() int {
	return len(d.data)
}
