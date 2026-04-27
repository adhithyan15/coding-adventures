package devicesimulator

// SimpleGlobalMemory -- device-wide VRAM / HBM simulator.
//
// # What is Global Memory?
//
// Global memory is the large, high-bandwidth memory that serves the entire
// accelerator device. Every compute unit can read from and write to global
// memory, making it the shared data store for all parallel computation.
//
//	NVIDIA: HBM3 (High Bandwidth Memory) -- 80 GB on H100
//	AMD:    GDDR6 -- 24 GB on RX 7900 XTX
//	Google: HBM2e -- 32 GB per TPU v4 chip
//	Intel:  GDDR6 -- 16 GB on Arc A770
//	Apple:  Unified LPDDR5 -- shared with CPU/GPU, up to 192 GB
//
// # Key Properties
//
//  1. **High bandwidth**: 1-3 TB/s. Much faster than CPU memory (~50 GB/s).
//  2. **High latency**: ~400-800 cycles to service a request.
//  3. **Shared**: ALL compute units on the device share global memory.
//  4. **Coalescing**: The memory controller can merge multiple thread
//     requests into fewer wide transactions if the addresses are contiguous.
//  5. **Partitioned**: Memory is physically split across channels/stacks.
//
// # Memory Coalescing
//
// Coalescing is the single most important optimization for GPU memory access.
// When 32 threads in a warp access addresses that fall within the same
// 128-byte cache line, the hardware combines them into ONE transaction:
//
//	Thread 0: addr 0x1000  --+
//	Thread 1: addr 0x1004    |
//	Thread 2: addr 0x1008    +-- All in same 128B line -> 1 transaction
//	...                      |
//	Thread 31: addr 0x107C --+
//
//	vs. scattered access:
//	Thread 0: addr 0x1000    -- Transaction 1
//	Thread 1: addr 0x5000    -- Transaction 2
//	...32 separate transactions = 32x more memory traffic!
//
// # Sparse Memory Representation
//
// Real devices have 16-80 GB of VRAM. We obviously can't allocate that in
// a simulator. Instead, we use a sparse map: only addresses that have
// been written to consume actual memory. A read to an uninitialized address
// returns zeros (matching real hardware behavior after cudaMemset).

import (
	"fmt"
	"sort"
)

// SimpleGlobalMemory models the device-wide memory (VRAM/HBM) that all compute
// units share. It tracks access patterns, coalescing efficiency, and partition
// conflicts to help identify memory bottlenecks.
//
// Usage:
//
//	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{Capacity: 1024*1024, Channels: 4})
//	addr, _ := mem.Allocate(256, 256)
//	mem.CopyFromHost(addr, data, 0)
//	mem.Write(addr, []byte{0x41, 0x42})
//	data, _ := mem.Read(addr, 4)
//	txns := mem.Coalesce(addrs, 4)
type SimpleGlobalMemory struct {
	capacity        int
	bandwidth       float64
	latency         int
	channels        int
	transactionSize int
	hostBandwidth   float64
	hostLatency     int
	unified         bool

	// Sparse storage -- only written addresses consume memory
	data map[int]byte

	// Simple bump allocator
	nextFree    int
	allocations map[int]int // start_addr -> size

	// Statistics
	stats GlobalMemoryStats
}

// SimpleGlobalMemoryConfig holds configuration for global memory.
type SimpleGlobalMemoryConfig struct {
	Capacity        int
	Bandwidth       float64
	Latency         int
	Channels        int
	TransactionSize int
	HostBandwidth   float64
	HostLatency     int
	Unified         bool
}

// DefaultSimpleGlobalMemoryConfig returns defaults for global memory.
func DefaultSimpleGlobalMemoryConfig() SimpleGlobalMemoryConfig {
	return SimpleGlobalMemoryConfig{
		Capacity:        16 * 1024 * 1024,
		Bandwidth:       1000.0,
		Latency:         400,
		Channels:        8,
		TransactionSize: 128,
		HostBandwidth:   64.0,
		HostLatency:     1000,
		Unified:         false,
	}
}

// NewSimpleGlobalMemory creates a new SimpleGlobalMemory with the given config.
func NewSimpleGlobalMemory(cfg SimpleGlobalMemoryConfig) *SimpleGlobalMemory {
	return &SimpleGlobalMemory{
		capacity:        cfg.Capacity,
		bandwidth:       cfg.Bandwidth,
		latency:         cfg.Latency,
		channels:        cfg.Channels,
		transactionSize: cfg.TransactionSize,
		hostBandwidth:   cfg.HostBandwidth,
		hostLatency:     cfg.HostLatency,
		unified:         cfg.Unified,
		data:            make(map[int]byte),
		allocations:     make(map[int]int),
	}
}

// Capacity returns the total memory in bytes.
func (m *SimpleGlobalMemory) Capacity() int {
	return m.capacity
}

// Bandwidth returns peak bandwidth in bytes per cycle.
func (m *SimpleGlobalMemory) Bandwidth() float64 {
	return m.bandwidth
}

// Stats returns a copy of the access statistics with efficiency updated.
func (m *SimpleGlobalMemory) Stats() GlobalMemoryStats {
	m.stats.UpdateEfficiency()
	return m.stats
}

// =========================================================================
// Allocation
// =========================================================================

// Allocate allocates memory and returns the start address.
//
// Uses a simple bump allocator with alignment. Like cudaMalloc,
// this returns a device pointer that can be passed to kernels.
//
// Returns an error if not enough memory remains.
func (m *SimpleGlobalMemory) Allocate(size int, alignment int) (int, error) {
	if alignment <= 0 {
		alignment = 256
	}
	// Align the next free pointer
	aligned := (m.nextFree + alignment - 1) & ^(alignment - 1)

	if aligned+size > m.capacity {
		return 0, fmt.Errorf("out of device memory: requested %d bytes at %d, capacity %d",
			size, aligned, m.capacity)
	}

	m.allocations[aligned] = size
	m.nextFree = aligned + size
	return aligned, nil
}

// Free releases a previous allocation.
//
// Note: our simple bump allocator doesn't reclaim memory. In a real
// implementation you'd use a more sophisticated allocator.
func (m *SimpleGlobalMemory) Free(address int) {
	delete(m.allocations, address)
}

// =========================================================================
// Read / Write
// =========================================================================

// Read reads bytes from global memory.
//
// Uninitialized addresses return zeros (like cudaMemset(0)).
// Returns an error if the address is out of range.
func (m *SimpleGlobalMemory) Read(address int, size int) ([]byte, error) {
	if address < 0 || address+size > m.capacity {
		return nil, fmt.Errorf("address %d+%d out of range [0, %d)", address, size, m.capacity)
	}

	m.stats.TotalReads++
	m.stats.BytesTransferred += size

	result := make([]byte, size)
	for i := 0; i < size; i++ {
		if v, ok := m.data[address+i]; ok {
			result[i] = v
		}
		// else zero (default)
	}
	return result, nil
}

// Write writes bytes to global memory.
//
// Returns an error if the address is out of range.
func (m *SimpleGlobalMemory) Write(address int, data []byte) error {
	size := len(data)
	if address < 0 || address+size > m.capacity {
		return fmt.Errorf("address %d+%d out of range [0, %d)", address, size, m.capacity)
	}

	m.stats.TotalWrites++
	m.stats.BytesTransferred += size

	for i, b := range data {
		m.data[address+i] = b
	}
	return nil
}

// =========================================================================
// Host transfers
// =========================================================================

// CopyFromHost copies from host (CPU) to device memory.
//
// Like cudaMemcpy(dst, src, size, cudaMemcpyHostToDevice).
//
// For unified memory (Apple), this is zero-cost -- no actual data
// movement, just a page table remap.
//
// Returns the number of cycles consumed by the transfer.
func (m *SimpleGlobalMemory) CopyFromHost(dstAddr int, data []byte, hostBandwidth float64) (int, error) {
	if err := m.Write(dstAddr, data); err != nil {
		return 0, err
	}

	bw := hostBandwidth
	if bw <= 0 {
		bw = m.hostBandwidth
	}
	size := len(data)
	m.stats.HostToDeviceBytes += size

	if m.unified {
		return 0, nil
	}

	cycles := 0
	if bw > 0 {
		cycles = m.hostLatency + int(float64(size)/bw)
	}
	m.stats.HostTransferCycles += cycles
	return cycles, nil
}

// CopyToHost copies from device memory to host (CPU).
//
// Like cudaMemcpy(dst, src, size, cudaMemcpyDeviceToHost).
//
// Returns (data, cycles).
func (m *SimpleGlobalMemory) CopyToHost(srcAddr int, size int, hostBandwidth float64) ([]byte, int, error) {
	data, err := m.Read(srcAddr, size)
	if err != nil {
		return nil, 0, err
	}

	bw := hostBandwidth
	if bw <= 0 {
		bw = m.hostBandwidth
	}
	m.stats.DeviceToHostBytes += size

	if m.unified {
		return data, 0, nil
	}

	cycles := 0
	if bw > 0 {
		cycles = m.hostLatency + int(float64(size)/bw)
	}
	m.stats.HostTransferCycles += cycles
	return data, cycles, nil
}

// =========================================================================
// Coalescing
// =========================================================================

// Coalesce merges per-thread addresses into coalesced transactions.
//
// # Coalescing Algorithm
//
//  1. For each thread's address, compute which transaction-sized
//     aligned region it falls in.
//  2. Group threads by aligned region.
//  3. Each group becomes one transaction.
//
// The fewer transactions, the better -- ideal is 1 transaction
// for 32 threads (128 bytes of contiguous access).
func (m *SimpleGlobalMemory) Coalesce(addresses []int, size int) []MemoryTransaction {
	ts := m.transactionSize

	// Group threads by aligned transaction address
	groups := make(map[int]uint64) // aligned_addr -> thread_mask
	for threadIdx, addr := range addresses {
		aligned := (addr / ts) * ts
		groups[aligned] |= 1 << uint(threadIdx)
	}

	// Sort by aligned address for deterministic output
	sortedAddrs := make([]int, 0, len(groups))
	for addr := range groups {
		sortedAddrs = append(sortedAddrs, addr)
	}
	sort.Ints(sortedAddrs)

	transactions := make([]MemoryTransaction, 0, len(groups))
	for _, aligned := range sortedAddrs {
		transactions = append(transactions, MemoryTransaction{
			Address:    aligned,
			Size:       ts,
			ThreadMask: groups[aligned],
		})
	}

	// Track stats
	m.stats.TotalRequests += len(addresses)
	m.stats.TotalTransactions += len(transactions)

	// Check partition conflicts
	channelsHit := make(map[int]int)
	for _, txn := range transactions {
		channel := (txn.Address / ts) % m.channels
		channelsHit[channel]++
	}
	for _, count := range channelsHit {
		if count > 1 {
			m.stats.PartitionConflicts += count - 1
		}
	}

	return transactions
}

// =========================================================================
// Reset
// =========================================================================

// Reset clears all data, allocations, and statistics.
func (m *SimpleGlobalMemory) Reset() {
	m.data = make(map[int]byte)
	m.nextFree = 0
	m.allocations = make(map[int]int)
	m.stats = GlobalMemoryStats{}
}
