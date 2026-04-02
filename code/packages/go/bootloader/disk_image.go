package bootloader

// DiskImage simulates persistent storage (hard drive / SSD).
// Pre-loaded with kernel and user program binaries before boot.

const (
	DiskBootSectorOffset = 0x00000000
	DiskBootSectorSize   = 512
	DiskKernelOffset     = 0x00080000
	DiskUserProgramBase  = 0x00100000
	DefaultDiskSize      = 2 * 1024 * 1024
)

// DiskImage simulates persistent storage.
type DiskImage struct {
	data []byte
}

// NewDiskImage creates an empty disk image of the given size in bytes.
func NewDiskImage(sizeBytes int) *DiskImage {
	result, _ := StartNew[*DiskImage]("bootloader.NewDiskImage", nil,
		func(op *Operation[*DiskImage], rf *ResultFactory[*DiskImage]) *OperationResult[*DiskImage] {
			op.AddProperty("sizeBytes", sizeBytes)
			return rf.Generate(true, false, &DiskImage{data: make([]byte, sizeBytes)})
		}).GetResult()
	return result
}

// LoadKernel writes a kernel binary to the conventional disk offset.
func (d *DiskImage) LoadKernel(kernelBinary []byte) {
	_, _ = StartNew[struct{}]("bootloader.DiskImage.LoadKernel", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			d.LoadAt(DiskKernelOffset, kernelBinary)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// LoadUserProgram writes a user program binary at a specified disk offset.
func (d *DiskImage) LoadUserProgram(programBinary []byte, offset int) {
	_, _ = StartNew[struct{}]("bootloader.DiskImage.LoadUserProgram", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("offset", offset)
			d.LoadAt(offset, programBinary)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// LoadAt writes raw bytes at a specific offset within the disk image.
func (d *DiskImage) LoadAt(offset int, data []byte) {
	_, _ = StartNew[struct{}]("bootloader.DiskImage.LoadAt", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("offset", offset)
			if offset+len(data) > len(d.data) {
				panic("DiskImage: data exceeds disk size")
			}
			copy(d.data[offset:], data)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ReadWord reads a 32-bit little-endian word at the given disk offset.
func (d *DiskImage) ReadWord(offset int) uint32 {
	result, _ := StartNew[uint32]("bootloader.DiskImage.ReadWord", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("offset", offset)
			if offset < 0 || offset+4 > len(d.data) {
				return rf.Generate(true, false, uint32(0))
			}
			val := uint32(d.data[offset]) |
				uint32(d.data[offset+1])<<8 |
				uint32(d.data[offset+2])<<16 |
				uint32(d.data[offset+3])<<24
			return rf.Generate(true, false, val)
		}).GetResult()
	return result
}

// ReadByteAt reads a single byte at the given disk offset.
func (d *DiskImage) ReadByteAt(offset int) byte {
	result, _ := StartNew[byte]("bootloader.DiskImage.ReadByteAt", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			op.AddProperty("offset", offset)
			if offset < 0 || offset >= len(d.data) {
				return rf.Generate(true, false, byte(0))
			}
			return rf.Generate(true, false, d.data[offset])
		}).GetResult()
	return result
}

// Data returns the raw byte slice for memory-mapping into the address space.
func (d *DiskImage) Data() []byte {
	result, _ := StartNew[[]byte]("bootloader.DiskImage.Data", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			return rf.Generate(true, false, d.data)
		}).GetResult()
	return result
}

// Size returns the total size of the disk image in bytes.
func (d *DiskImage) Size() int {
	result, _ := StartNew[int]("bootloader.DiskImage.Size", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(d.data))
		}).GetResult()
	return result
}
