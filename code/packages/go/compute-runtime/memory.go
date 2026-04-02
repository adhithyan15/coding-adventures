package computeruntime

// Memory management -- typed allocations, mapping, staging.
//
// # Memory Types on a GPU
//
// Unlike a CPU where all RAM is equally accessible, GPUs have distinct memory
// pools with different performance characteristics:
//
//	+-------------------------------------------------------------+
//	|                   Discrete GPU (NVIDIA, AMD)                  |
//	|                                                              |
//	|   CPU side (system RAM)              GPU side (VRAM)         |
//	|   +------------------+               +------------------+    |
//	|   |   HOST_VISIBLE   |<---- PCIe --->|   DEVICE_LOCAL   |    |
//	|   |   HOST_COHERENT  |   ~32 GB/s    |   (HBM / GDDR6)  |    |
//	|   |   (staging pool) |               |   1-3 TB/s        |    |
//	|   +------------------+               +------------------+    |
//	+-------------------------------------------------------------+
//
//	+-------------------------------------------------------------+
//	|                 Unified Memory (Apple M-series)               |
//	|                                                              |
//	|   +------------------------------------------------------+  |
//	|   |        DEVICE_LOCAL + HOST_VISIBLE + HOST_COHERENT    |  |
//	|   |        (shared physical RAM)                          |  |
//	|   |        Both CPU and GPU see the same bytes            |  |
//	|   +------------------------------------------------------+  |
//	+-------------------------------------------------------------+
//
// # The Staging Buffer Pattern
//
// On discrete GPUs, the standard way to get data onto the GPU is:
//
//  1. Allocate a HOST_VISIBLE staging buffer (CPU can write to it)
//  2. Map it, write your data, unmap it
//  3. Record a cmd_copy_buffer from staging -> DEVICE_LOCAL
//  4. Submit and wait
//
// This two-step dance is necessary because DEVICE_LOCAL memory (VRAM) is
// not directly writable by the CPU.

import (
	"fmt"

	devicesimulator "github.com/adhithyan15/coding-adventures/code/packages/go/device-simulator"
)

// =========================================================================
// Buffer -- a typed allocation on the device
// =========================================================================

// Buffer represents a memory allocation on the device.
//
// # Buffer Lifecycle
//
//	Allocate() -> Buffer (with DeviceAddress)
//	Map()      -> MappedMemory (CPU can read/write)
//	Unmap()    -> buffer is GPU-only again
//	Free()     -> memory returned to pool
type Buffer struct {
	BufferID      int        // Unique identifier for this buffer.
	Size          int        // Size in bytes.
	MemType       MemoryType // What kind of memory (DEVICE_LOCAL, HOST_VISIBLE, etc.).
	Usage         BufferUsage // How it will be used (STORAGE, TRANSFER_SRC, etc.).
	DeviceAddress int        // Address on the device (from Layer 6 malloc).
	Mapped        bool       // Whether this buffer is currently CPU-mapped.
	Freed         bool       // Whether this buffer has been freed.
}

// =========================================================================
// MappedMemory -- CPU-accessible view of a buffer
// =========================================================================

// MappedMemory provides a CPU-accessible view of a mapped GPU buffer.
//
// # What is Memory Mapping?
//
// Mapping makes device memory accessible to the CPU. On discrete GPUs,
// this only works for HOST_VISIBLE memory (system RAM accessible via PCIe).
// On unified memory, any buffer can be mapped.
//
// After mapping, you can Read() and Write() bytes. After Unmap(), the
// CPU can no longer access this memory.
type MappedMemory struct {
	buffer *Buffer
	data   []byte
	dirty  bool
}

// Buffer returns the buffer this mapping refers to.
func (m *MappedMemory) Buffer() *Buffer {
	result, _ := StartNew[*Buffer]("compute-runtime.MappedMemory.Buffer", nil,
		func(op *Operation[*Buffer], rf *ResultFactory[*Buffer]) *OperationResult[*Buffer] {
			return rf.Generate(true, false, m.buffer)
		}).GetResult()
	return result
}

// Size returns the size of the mapped region.
func (m *MappedMemory) Size() int {
	result, _ := StartNew[int]("compute-runtime.MappedMemory.Size", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(m.data))
		}).GetResult()
	return result
}

// Dirty returns whether any writes have been made since mapping.
func (m *MappedMemory) Dirty() bool {
	result, _ := StartNew[bool]("compute-runtime.MappedMemory.Dirty", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, m.dirty)
		}).GetResult()
	return result
}

// Read reads bytes from the mapped buffer.
//
// Returns an error if offset + size exceeds buffer size.
func (m *MappedMemory) Read(offset, size int) ([]byte, error) {
	res, err := StartNew[[]byte]("compute-runtime.MappedMemory.Read", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("offset", offset)
			op.AddProperty("size", size)
			if offset+size > len(m.data) {
				return rf.Fail(nil, fmt.Errorf(
					"read out of bounds: offset=%d, size=%d, buffer_size=%d",
					offset, size, len(m.data),
				))
			}
			result := make([]byte, size)
			copy(result, m.data[offset:offset+size])
			return rf.Generate(true, false, result)
		}).GetResult()
	return res, err
}

// Write writes bytes to the mapped buffer.
//
// Returns an error if offset + len(data) exceeds buffer size.
func (m *MappedMemory) Write(offset int, data []byte) error {
	_, err := StartNew[struct{}]("compute-runtime.MappedMemory.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("offset", offset)
			op.AddProperty("data_size", len(data))
			if offset+len(data) > len(m.data) {
				return rf.Fail(struct{}{}, fmt.Errorf(
					"write out of bounds: offset=%d, data_size=%d, buffer_size=%d",
					offset, len(data), len(m.data),
				))
			}
			copy(m.data[offset:offset+len(data)], data)
			m.dirty = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// GetData returns the full contents of the mapped buffer.
func (m *MappedMemory) GetData() []byte {
	result, _ := StartNew[[]byte]("compute-runtime.MappedMemory.GetData", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			res := make([]byte, len(m.data))
			copy(res, m.data)
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// =========================================================================
// MemoryManager -- allocates, maps, frees device memory
// =========================================================================

// MemoryManager manages typed memory allocations on a device.
//
// # How It Works
//
// The MemoryManager wraps Layer 6's raw malloc/free with type information.
// Each allocation is tagged with a MemoryType and BufferUsage, which the
// runtime uses for validation and optimization.
//
// For HOST_VISIBLE allocations, the manager supports mapping -- making the
// buffer accessible to the CPU. For DEVICE_LOCAL-only allocations, mapping
// is not allowed (you must use a staging buffer + copy).
type MemoryManager struct {
	device     devicesimulator.AcceleratorDevice
	properties MemoryProperties
	stats      *RuntimeStats
	buffers    map[int]*Buffer
	bufferData map[int][]byte
	nextID     int
	currentBytes int
}

// NewMemoryManager creates a new MemoryManager for the given device.
func NewMemoryManager(
	device devicesimulator.AcceleratorDevice,
	properties MemoryProperties,
	stats *RuntimeStats,
) *MemoryManager {
	result, _ := StartNew[*MemoryManager]("compute-runtime.NewMemoryManager", nil,
		func(op *Operation[*MemoryManager], rf *ResultFactory[*MemoryManager]) *OperationResult[*MemoryManager] {
			return rf.Generate(true, false, &MemoryManager{
				device:     device,
				properties: properties,
				stats:      stats,
				buffers:    make(map[int]*Buffer),
				bufferData: make(map[int][]byte),
			})
		}).GetResult()
	return result
}

// MemoryProperties returns the memory properties of the underlying device.
func (mm *MemoryManager) MemoryProperties() MemoryProperties {
	result, _ := StartNew[MemoryProperties]("compute-runtime.MemoryManager.MemoryProperties", MemoryProperties{},
		func(op *Operation[MemoryProperties], rf *ResultFactory[MemoryProperties]) *OperationResult[MemoryProperties] {
			return rf.Generate(true, false, mm.properties)
		}).GetResult()
	return result
}

// Allocate allocates a buffer on the device.
//
// # The Allocation Flow
//
//	MemoryManager.Allocate(1024, MemoryTypeDeviceLocal)
//	    |
//	    +---> Validate: size > 0
//	    +---> Layer 6: device.Malloc(1024) -> deviceAddress
//	    +---> Create Buffer object with metadata
//	    +---> Track in buffers map
//	    +---> Log RuntimeTrace event
//
// Returns an error if size <= 0 or allocation fails.
func (mm *MemoryManager) Allocate(size int, memType MemoryType, usage BufferUsage) (*Buffer, error) {
	res, err := StartNew[*Buffer]("compute-runtime.MemoryManager.Allocate", nil,
		func(op *Operation[*Buffer], rf *ResultFactory[*Buffer]) *OperationResult[*Buffer] {
			op.AddProperty("size", size)
			if size <= 0 {
				return rf.Fail(nil, fmt.Errorf("allocation size must be positive, got %d", size))
			}

			deviceAddress, err := mm.device.Malloc(size)
			if err != nil {
				return rf.Fail(nil, fmt.Errorf("device malloc failed: %w", err))
			}

			bufID := mm.nextID
			mm.nextID++

			buf := &Buffer{
				BufferID:      bufID,
				Size:          size,
				MemType:       memType,
				Usage:         usage,
				DeviceAddress: deviceAddress,
			}
			mm.buffers[bufID] = buf
			mm.bufferData[bufID] = make([]byte, size)

			mm.currentBytes += size
			mm.stats.TotalAllocatedBytes += size
			mm.stats.TotalAllocations++
			if mm.currentBytes > mm.stats.PeakAllocatedBytes {
				mm.stats.PeakAllocatedBytes = mm.currentBytes
			}

			mm.stats.Traces = append(mm.stats.Traces, RuntimeTrace{
				EventType:   RuntimeEventMemoryAlloc,
				Description: fmt.Sprintf("Allocated %d bytes (buf#%d, %s)", size, bufID, memType),
			})

			return rf.Generate(true, false, buf)
		}).GetResult()
	return res, err
}

// Free frees a device memory allocation.
//
// Returns an error if the buffer is already freed, not found, or still mapped.
func (mm *MemoryManager) Free(buffer *Buffer) error {
	_, err := StartNew[struct{}]("compute-runtime.MemoryManager.Free", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", buffer.BufferID)
			if buffer.Freed {
				return rf.Fail(struct{}{}, fmt.Errorf("buffer %d already freed", buffer.BufferID))
			}
			if _, ok := mm.buffers[buffer.BufferID]; !ok {
				return rf.Fail(struct{}{}, fmt.Errorf("buffer %d not found", buffer.BufferID))
			}
			if buffer.Mapped {
				return rf.Fail(struct{}{}, fmt.Errorf("buffer %d is still mapped -- unmap before freeing", buffer.BufferID))
			}

			mm.device.Free(buffer.DeviceAddress)
			buffer.Freed = true
			mm.currentBytes -= buffer.Size
			delete(mm.buffers, buffer.BufferID)
			delete(mm.bufferData, buffer.BufferID)

			mm.stats.TotalFrees++
			mm.stats.Traces = append(mm.stats.Traces, RuntimeTrace{
				EventType:   RuntimeEventMemoryFree,
				Description: fmt.Sprintf("Freed buf#%d (%d bytes)", buffer.BufferID, buffer.Size),
			})

			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// Map maps a buffer for CPU access.
//
// Only HOST_VISIBLE buffers can be mapped. Returns an error if the buffer
// is not HOST_VISIBLE, already mapped, or freed.
func (mm *MemoryManager) Map(buffer *Buffer) (*MappedMemory, error) {
	res, err := StartNew[*MappedMemory]("compute-runtime.MemoryManager.Map", nil,
		func(op *Operation[*MappedMemory], rf *ResultFactory[*MappedMemory]) *OperationResult[*MappedMemory] {
			op.AddProperty("buffer_id", buffer.BufferID)
			if buffer.Freed {
				return rf.Fail(nil, fmt.Errorf("cannot map freed buffer %d", buffer.BufferID))
			}
			if buffer.Mapped {
				return rf.Fail(nil, fmt.Errorf("buffer %d is already mapped", buffer.BufferID))
			}
			if !buffer.MemType.Has(MemoryTypeHostVisible) {
				return rf.Fail(nil, fmt.Errorf(
					"cannot map buffer %d: not HOST_VISIBLE (type=%s)",
					buffer.BufferID, buffer.MemType,
				))
			}

			buffer.Mapped = true
			mm.stats.TotalMaps++

			mm.stats.Traces = append(mm.stats.Traces, RuntimeTrace{
				EventType:   RuntimeEventMemoryMap,
				Description: fmt.Sprintf("Mapped buf#%d", buffer.BufferID),
			})

			return rf.Generate(true, false, &MappedMemory{
				buffer: buffer,
				data:   mm.bufferData[buffer.BufferID],
			})
		}).GetResult()
	return res, err
}

// Unmap unmaps a buffer, ending CPU access.
//
// If the mapped memory was written to (dirty) and the buffer has
// HOST_COHERENT, the data is automatically synced to the device.
func (mm *MemoryManager) Unmap(buffer *Buffer) error {
	_, err := StartNew[struct{}]("compute-runtime.MemoryManager.Unmap", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", buffer.BufferID)
			if !buffer.Mapped {
				return rf.Fail(struct{}{}, fmt.Errorf("buffer %d is not mapped", buffer.BufferID))
			}

			// If HOST_COHERENT, automatically sync to device
			if buffer.MemType.Has(MemoryTypeHostCoherent) {
				data := make([]byte, len(mm.bufferData[buffer.BufferID]))
				copy(data, mm.bufferData[buffer.BufferID])
				_, err := mm.device.MemcpyHostToDevice(buffer.DeviceAddress, data)
				if err != nil {
					return rf.Fail(struct{}{}, fmt.Errorf("sync to device failed: %w", err))
				}
			}

			buffer.Mapped = false
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// Flush flushes CPU writes to make them visible to GPU.
//
// Only needed for HOST_VISIBLE buffers without HOST_COHERENT.
func (mm *MemoryManager) Flush(buffer *Buffer, offset, size int) error {
	_, err := StartNew[struct{}]("compute-runtime.MemoryManager.Flush", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", buffer.BufferID)
			op.AddProperty("offset", offset)
			op.AddProperty("size", size)
			if buffer.Freed {
				return rf.Fail(struct{}{}, fmt.Errorf("cannot flush freed buffer %d", buffer.BufferID))
			}
			actualSize := size
			if actualSize <= 0 {
				actualSize = buffer.Size
			}
			data := make([]byte, actualSize)
			copy(data, mm.bufferData[buffer.BufferID][offset:offset+actualSize])
			_, err := mm.device.MemcpyHostToDevice(buffer.DeviceAddress+offset, data)
			if err != nil {
				return rf.Fail(struct{}{}, err)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// Invalidate invalidates CPU cache so GPU writes become visible to CPU.
func (mm *MemoryManager) Invalidate(buffer *Buffer, offset, size int) error {
	_, err := StartNew[struct{}]("compute-runtime.MemoryManager.Invalidate", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", buffer.BufferID)
			op.AddProperty("offset", offset)
			op.AddProperty("size", size)
			if buffer.Freed {
				return rf.Fail(struct{}{}, fmt.Errorf("cannot invalidate freed buffer %d", buffer.BufferID))
			}
			actualSize := size
			if actualSize <= 0 {
				actualSize = buffer.Size
			}
			data, _, err := mm.device.MemcpyDeviceToHost(buffer.DeviceAddress+offset, actualSize)
			if err != nil {
				return rf.Fail(struct{}{}, err)
			}
			copy(mm.bufferData[buffer.BufferID][offset:offset+actualSize], data)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// GetBuffer looks up a buffer by ID.
func (mm *MemoryManager) GetBuffer(bufferID int) (*Buffer, error) {
	res, err := StartNew[*Buffer]("compute-runtime.MemoryManager.GetBuffer", nil,
		func(op *Operation[*Buffer], rf *ResultFactory[*Buffer]) *OperationResult[*Buffer] {
			op.AddProperty("buffer_id", bufferID)
			buf, ok := mm.buffers[bufferID]
			if !ok {
				return rf.Fail(nil, fmt.Errorf("buffer %d not found", bufferID))
			}
			return rf.Generate(true, false, buf)
		}).GetResult()
	return res, err
}

// AllocatedBufferCount returns the number of currently allocated buffers.
func (mm *MemoryManager) AllocatedBufferCount() int {
	result, _ := StartNew[int]("compute-runtime.MemoryManager.AllocatedBufferCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(mm.buffers))
		}).GetResult()
	return result
}

// CurrentAllocatedBytes returns the current total bytes allocated.
func (mm *MemoryManager) CurrentAllocatedBytes() int {
	result, _ := StartNew[int]("compute-runtime.MemoryManager.CurrentAllocatedBytes", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, mm.currentBytes)
		}).GetResult()
	return result
}

// GetBufferData returns raw data for a buffer (internal use).
func (mm *MemoryManager) GetBufferData(bufferID int) []byte {
	result, _ := StartNew[[]byte]("compute-runtime.MemoryManager.GetBufferData", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("buffer_id", bufferID)
			return rf.Generate(true, false, mm.bufferData[bufferID])
		}).GetResult()
	return result
}

// SyncBufferToDevice pushes buffer data to device. Returns cycles consumed.
func (mm *MemoryManager) SyncBufferToDevice(buffer *Buffer) (int, error) {
	res, err := StartNew[int]("compute-runtime.MemoryManager.SyncBufferToDevice", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("buffer_id", buffer.BufferID)
			data := make([]byte, len(mm.bufferData[buffer.BufferID]))
			copy(data, mm.bufferData[buffer.BufferID])
			cycles, err := mm.device.MemcpyHostToDevice(buffer.DeviceAddress, data)
			if err != nil {
				return rf.Fail(0, err)
			}
			return rf.Generate(true, false, cycles)
		}).GetResult()
	return res, err
}

// SyncBufferFromDevice pulls buffer data from device. Returns cycles consumed.
func (mm *MemoryManager) SyncBufferFromDevice(buffer *Buffer) (int, error) {
	res, err := StartNew[int]("compute-runtime.MemoryManager.SyncBufferFromDevice", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("buffer_id", buffer.BufferID)
			data, cycles, err := mm.device.MemcpyDeviceToHost(buffer.DeviceAddress, buffer.Size)
			if err != nil {
				return rf.Fail(0, err)
			}
			copy(mm.bufferData[buffer.BufferID], data)
			return rf.Generate(true, false, cycles)
		}).GetResult()
	return res, err
}
