package devicesimulator

import (
	"testing"
)

// =========================================================================
// SimpleGlobalMemory -- construction and properties
// =========================================================================

func TestNewSimpleGlobalMemory(t *testing.T) {
	mem := NewSimpleGlobalMemory(DefaultSimpleGlobalMemoryConfig())

	if mem.Capacity() != 16*1024*1024 {
		t.Errorf("Capacity: got %d, want %d", mem.Capacity(), 16*1024*1024)
	}
	if mem.Bandwidth() != 1000.0 {
		t.Errorf("Bandwidth: got %f, want 1000.0", mem.Bandwidth())
	}
}

func TestSimpleGlobalMemoryCustomConfig(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        1024,
		Bandwidth:       500.0,
		Latency:         200,
		Channels:        4,
		TransactionSize: 64,
		HostBandwidth:   32.0,
		HostLatency:     500,
		Unified:         false,
	})

	if mem.Capacity() != 1024 {
		t.Errorf("Capacity: got %d, want 1024", mem.Capacity())
	}
}

// =========================================================================
// Allocation
// =========================================================================

func TestAllocateBasic(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	addr, err := mem.Allocate(256, 256)
	if err != nil {
		t.Fatalf("Allocate: unexpected error: %v", err)
	}
	if addr != 0 {
		t.Errorf("first allocation should be at 0, got %d", addr)
	}
}

func TestAllocateAlignment(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        8192,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	// First allocation at 0
	addr1, _ := mem.Allocate(100, 256)
	if addr1 != 0 {
		t.Errorf("addr1: got %d, want 0", addr1)
	}

	// Second allocation should be aligned to 256
	addr2, _ := mem.Allocate(100, 256)
	if addr2 != 256 {
		t.Errorf("addr2: got %d, want 256", addr2)
	}
}

func TestAllocateOutOfMemory(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        512,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	_, err := mem.Allocate(1024, 256)
	if err == nil {
		t.Error("expected out of memory error")
	}
}

func TestFree(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	addr, _ := mem.Allocate(256, 256)
	mem.Free(addr) // Should not panic

	// Freeing a non-existent address should be safe
	mem.Free(9999)
}

// =========================================================================
// Read / Write
// =========================================================================

func TestReadWriteBasic(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	data := []byte{0x41, 0x42, 0x43, 0x44}
	err := mem.Write(0, data)
	if err != nil {
		t.Fatalf("Write: unexpected error: %v", err)
	}

	result, err := mem.Read(0, 4)
	if err != nil {
		t.Fatalf("Read: unexpected error: %v", err)
	}

	for i, b := range data {
		if result[i] != b {
			t.Errorf("byte %d: got %x, want %x", i, result[i], b)
		}
	}
}

func TestReadUninitializedReturnsZeros(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	result, err := mem.Read(0, 4)
	if err != nil {
		t.Fatalf("Read: unexpected error: %v", err)
	}

	for i, b := range result {
		if b != 0 {
			t.Errorf("byte %d: got %x, want 0", i, b)
		}
	}
}

func TestReadOutOfRange(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        128,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	_, err := mem.Read(120, 16)
	if err == nil {
		t.Error("expected out of range error")
	}
}

func TestWriteOutOfRange(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        128,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	err := mem.Write(120, make([]byte, 16))
	if err == nil {
		t.Error("expected out of range error")
	}
}

func TestReadNegativeAddress(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        128,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	_, err := mem.Read(-1, 4)
	if err == nil {
		t.Error("expected out of range error for negative address")
	}
}

func TestReadWriteStats(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	mem.Write(0, []byte{1, 2, 3, 4})
	mem.Read(0, 4)
	mem.Read(0, 4)

	stats := mem.Stats()
	if stats.TotalWrites != 1 {
		t.Errorf("TotalWrites: got %d, want 1", stats.TotalWrites)
	}
	if stats.TotalReads != 2 {
		t.Errorf("TotalReads: got %d, want 2", stats.TotalReads)
	}
	if stats.BytesTransferred != 12 { // 4 + 4 + 4
		t.Errorf("BytesTransferred: got %d, want 12", stats.BytesTransferred)
	}
}

// =========================================================================
// Host transfers
// =========================================================================

func TestCopyFromHostBasic(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
		HostLatency:     100,
		Unified:         false,
	})

	data := []byte{0xDE, 0xAD, 0xBE, 0xEF}
	cycles, err := mem.CopyFromHost(0, data, 0)
	if err != nil {
		t.Fatalf("CopyFromHost: unexpected error: %v", err)
	}
	if cycles <= 0 {
		t.Errorf("expected positive cycles, got %d", cycles)
	}

	// Verify data was written
	result, _ := mem.Read(0, 4)
	for i, b := range data {
		if result[i] != b {
			t.Errorf("byte %d: got %x, want %x", i, result[i], b)
		}
	}
}

func TestCopyFromHostUnifiedMemory(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   200.0,
		HostLatency:     0,
		Unified:         true,
	})

	data := []byte{1, 2, 3, 4}
	cycles, err := mem.CopyFromHost(0, data, 0)
	if err != nil {
		t.Fatalf("CopyFromHost: unexpected error: %v", err)
	}
	if cycles != 0 {
		t.Errorf("unified memory copy should be zero-cost, got %d cycles", cycles)
	}
}

func TestCopyToHostBasic(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
		HostLatency:     100,
		Unified:         false,
	})

	// Write some data
	mem.Write(0, []byte{0xCA, 0xFE})

	result, cycles, err := mem.CopyToHost(0, 2, 0)
	if err != nil {
		t.Fatalf("CopyToHost: unexpected error: %v", err)
	}
	if cycles <= 0 {
		t.Errorf("expected positive cycles, got %d", cycles)
	}
	if result[0] != 0xCA || result[1] != 0xFE {
		t.Errorf("data mismatch: got %v", result)
	}
}

func TestCopyToHostUnifiedMemory(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   200.0,
		HostLatency:     0,
		Unified:         true,
	})

	mem.Write(0, []byte{1, 2})
	_, cycles, err := mem.CopyToHost(0, 2, 0)
	if err != nil {
		t.Fatalf("CopyToHost: unexpected error: %v", err)
	}
	if cycles != 0 {
		t.Errorf("unified memory copy should be zero-cost, got %d cycles", cycles)
	}
}

func TestHostTransferStats(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
		HostLatency:     100,
		Unified:         false,
	})

	mem.CopyFromHost(0, make([]byte, 256), 0)
	mem.CopyToHost(0, 128, 0)

	stats := mem.Stats()
	if stats.HostToDeviceBytes != 256 {
		t.Errorf("HostToDeviceBytes: got %d, want 256", stats.HostToDeviceBytes)
	}
	if stats.DeviceToHostBytes != 128 {
		t.Errorf("DeviceToHostBytes: got %d, want 128", stats.DeviceToHostBytes)
	}
	if stats.HostTransferCycles <= 0 {
		t.Errorf("HostTransferCycles should be positive, got %d", stats.HostTransferCycles)
	}
}

func TestCopyFromHostOutOfRange(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        128,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	_, err := mem.CopyFromHost(120, make([]byte, 16), 0)
	if err == nil {
		t.Error("expected out of range error")
	}
}

// =========================================================================
// Coalescing
// =========================================================================

func TestCoalescePerfect(t *testing.T) {
	// All 32 threads access contiguous 4-byte elements -> 1 transaction
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	addresses := make([]int, 32)
	for i := 0; i < 32; i++ {
		addresses[i] = i * 4 // contiguous 4-byte accesses within 128B
	}

	txns := mem.Coalesce(addresses, 4)
	if len(txns) != 1 {
		t.Errorf("perfect coalescing should produce 1 transaction, got %d", len(txns))
	}
}

func TestCoalesceScattered(t *testing.T) {
	// Each thread accesses a different 128B region -> 32 transactions
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        65536,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	addresses := make([]int, 32)
	for i := 0; i < 32; i++ {
		addresses[i] = i * 512 // widely scattered
	}

	txns := mem.Coalesce(addresses, 4)
	if len(txns) != 32 {
		t.Errorf("scattered access should produce 32 transactions, got %d", len(txns))
	}
}

func TestCoalescePartialCoalescing(t *testing.T) {
	// 32 threads, 2 groups of 16 -> 2 transactions
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	addresses := make([]int, 32)
	for i := 0; i < 16; i++ {
		addresses[i] = i * 4 // first 16 in line 0
	}
	for i := 16; i < 32; i++ {
		addresses[i] = 128 + (i-16)*4 // next 16 in line 1
	}

	txns := mem.Coalesce(addresses, 4)
	if len(txns) != 2 {
		t.Errorf("two-group access should produce 2 transactions, got %d", len(txns))
	}
}

func TestCoalesceThreadMask(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	// Threads 0 and 1 in same line
	addresses := []int{0, 4}
	txns := mem.Coalesce(addresses, 4)

	if len(txns) != 1 {
		t.Fatalf("expected 1 transaction, got %d", len(txns))
	}
	// ThreadMask should have bits 0 and 1 set = 0b11 = 3
	if txns[0].ThreadMask != 3 {
		t.Errorf("ThreadMask: got %d, want 3", txns[0].ThreadMask)
	}
}

func TestCoalesceStatsTracking(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	addresses := make([]int, 32)
	for i := 0; i < 32; i++ {
		addresses[i] = i * 4
	}
	mem.Coalesce(addresses, 4)

	stats := mem.Stats()
	if stats.TotalRequests != 32 {
		t.Errorf("TotalRequests: got %d, want 32", stats.TotalRequests)
	}
	if stats.TotalTransactions != 1 {
		t.Errorf("TotalTransactions: got %d, want 1", stats.TotalTransactions)
	}
	if stats.CoalescingEfficiency != 32.0 {
		t.Errorf("CoalescingEfficiency: got %f, want 32.0", stats.CoalescingEfficiency)
	}
}

func TestCoalescePartitionConflicts(t *testing.T) {
	// With 4 channels and transaction_size=128, transactions at addresses
	// 0 and 512 map to the same channel: (0/128)%4 = 0, (512/128)%4 = 0
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	addresses := []int{0, 512} // Both map to channel 0
	mem.Coalesce(addresses, 4)

	stats := mem.Stats()
	if stats.PartitionConflicts != 1 {
		t.Errorf("PartitionConflicts: got %d, want 1", stats.PartitionConflicts)
	}
}

// =========================================================================
// Reset
// =========================================================================

func TestReset(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
	})

	mem.Write(0, []byte{1, 2, 3})
	mem.Allocate(100, 256)
	mem.Read(0, 3)

	mem.Reset()

	// After reset, reading should return zeros
	result, _ := mem.Read(0, 3)
	for i, b := range result {
		if b != 0 {
			t.Errorf("after reset, byte %d: got %x, want 0", i, b)
		}
	}

	// Stats should be reset
	stats := mem.Stats()
	if stats.TotalReads != 1 { // The read we just did
		t.Errorf("after reset, TotalReads: got %d, want 1", stats.TotalReads)
	}
}

func TestCopyFromHostCustomBandwidth(t *testing.T) {
	mem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        4096,
		Channels:        4,
		TransactionSize: 128,
		HostBandwidth:   64.0,
		HostLatency:     100,
		Unified:         false,
	})

	// Use custom bandwidth override
	cycles, _ := mem.CopyFromHost(0, make([]byte, 128), 128.0)
	// cycles = 100 + int(128/128) = 100 + 1 = 101
	if cycles != 101 {
		t.Errorf("CopyFromHost with custom bandwidth: got %d, want 101", cycles)
	}
}
