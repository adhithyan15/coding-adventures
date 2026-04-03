package devicedriverframework

import "fmt"

// =========================================================================
// SimulatedDisk -- an in-memory block storage device
// =========================================================================
//
// A disk is a random-access storage device organized into fixed-size chunks
// called "blocks" or "sectors." The traditional sector size is 512 bytes,
// a legacy from the IBM PC/AT (1984).
//
// Our simulated disk is just a byte slice in memory. It enforces block-aligned
// access, just like real disk hardware:
//
//   Physical disk:              SimulatedDisk:
//   ┌──────────────────┐        ┌──────────────────┐
//   │ Block 0 (512 B)  │        │ storage[0:512]   │
//   ├──────────────────┤        ├──────────────────┤
//   │ Block 1 (512 B)  │        │ storage[512:1024]│
//   ├──────────────────┤        ├──────────────────┤
//   │ ...              │        │ ...              │
//   └──────────────────┘        └──────────────────┘

// SimulatedDisk is a simulated disk backed by an in-memory byte slice.
type SimulatedDisk struct {
	DeviceBase
	blockSize   int
	totalBlocks int
	storage     []byte
}

// NewSimulatedDisk creates a new simulated disk.
//
// Parameters:
//   - name: device name (e.g., "disk0")
//   - minor: minor number for this disk instance
//   - blockSize: bytes per block (typically 512)
//   - totalBlocks: number of blocks (e.g., 2048 for 1 MB)
func NewSimulatedDisk(name string, minor, blockSize, totalBlocks int) *SimulatedDisk {
	result, _ := StartNew[*SimulatedDisk]("device-driver-framework.NewSimulatedDisk", nil,
		func(op *Operation[*SimulatedDisk], rf *ResultFactory[*SimulatedDisk]) *OperationResult[*SimulatedDisk] {
			op.AddProperty("name", name)
			op.AddProperty("minor", minor)
			op.AddProperty("blockSize", blockSize)
			op.AddProperty("totalBlocks", totalBlocks)
			return rf.Generate(true, false, &SimulatedDisk{
				DeviceBase: DeviceBase{
					Name:            name,
					Type:            DeviceBlock,
					Major:           MajorDisk,
					Minor:           minor,
					InterruptNumber: IntDisk,
				},
				blockSize:   blockSize,
				totalBlocks: totalBlocks,
				storage:     make([]byte, blockSize*totalBlocks),
			})
		}).GetResult()
	return result
}

// Init initializes the disk by zeroing out the storage.
func (d *SimulatedDisk) Init() {
	_, _ = StartNew[struct{}]("device-driver-framework.SimulatedDisk.Init", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			d.storage = make([]byte, d.blockSize*d.totalBlocks)
			d.Initialized = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ReadBlock reads one block from the disk.
//
// The math:
//
//	offset = blockNum * blockSize
//	data = storage[offset : offset + blockSize]
//
// On a real disk, this would involve seeking, waiting for rotation,
// and reading magnetic flux patterns. Our simulation is instant.
func (d *SimulatedDisk) ReadBlock(blockNum int) ([]byte, error) {
	return StartNew[[]byte]("device-driver-framework.SimulatedDisk.ReadBlock", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("blockNum", blockNum)
			if blockNum < 0 || blockNum >= d.totalBlocks {
				return rf.Fail(nil, fmt.Errorf("block number %d out of range (0..%d)", blockNum, d.totalBlocks-1))
			}
			offset := blockNum * d.blockSize
			result := make([]byte, d.blockSize)
			copy(result, d.storage[offset:offset+d.blockSize])
			return rf.Generate(true, false, result)
		}).GetResult()
}

// WriteBlock writes one block to the disk.
//
// The data must be exactly blockSize bytes. This constraint mirrors real
// disk hardware, which always writes complete sectors.
func (d *SimulatedDisk) WriteBlock(blockNum int, data []byte) error {
	_, err := StartNew[struct{}]("device-driver-framework.SimulatedDisk.WriteBlock", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("blockNum", blockNum)
			if blockNum < 0 || blockNum >= d.totalBlocks {
				return rf.Fail(struct{}{}, fmt.Errorf("block number %d out of range (0..%d)", blockNum, d.totalBlocks-1))
			}
			if len(data) != d.blockSize {
				return rf.Fail(struct{}{}, fmt.Errorf("data must be exactly %d bytes, got %d", d.blockSize, len(data)))
			}
			offset := blockNum * d.blockSize
			copy(d.storage[offset:offset+d.blockSize], data)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// BlockSize returns the number of bytes per block.
func (d *SimulatedDisk) BlockSize() int {
	result, _ := StartNew[int]("device-driver-framework.SimulatedDisk.BlockSize", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, d.blockSize)
		}).GetResult()
	return result
}

// TotalBlocks returns the total number of blocks on this device.
func (d *SimulatedDisk) TotalBlocks() int {
	result, _ := StartNew[int]("device-driver-framework.SimulatedDisk.TotalBlocks", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, d.totalBlocks)
		}).GetResult()
	return result
}

// Storage returns the backing byte slice (for testing/debugging).
func (d *SimulatedDisk) Storage() []byte {
	result, _ := StartNew[[]byte]("device-driver-framework.SimulatedDisk.Storage", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			return rf.Generate(true, false, d.storage)
		}).GetResult()
	return result
}
