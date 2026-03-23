package devicesimulator

import (
	"strings"
	"testing"
)

// =========================================================================
// MemoryTransaction tests
// =========================================================================

func TestMemoryTransactionFields(t *testing.T) {
	txn := MemoryTransaction{
		Address:    0x1000,
		Size:       128,
		ThreadMask: 0xFFFFFFFF,
	}

	if txn.Address != 0x1000 {
		t.Errorf("Address: got %d, want %d", txn.Address, 0x1000)
	}
	if txn.Size != 128 {
		t.Errorf("Size: got %d, want %d", txn.Size, 128)
	}
	if txn.ThreadMask != 0xFFFFFFFF {
		t.Errorf("ThreadMask: got %d, want %d", txn.ThreadMask, uint64(0xFFFFFFFF))
	}
}

// =========================================================================
// GlobalMemoryStats tests
// =========================================================================

func TestGlobalMemoryStatsDefaults(t *testing.T) {
	stats := GlobalMemoryStats{}

	if stats.TotalReads != 0 {
		t.Errorf("TotalReads: got %d, want 0", stats.TotalReads)
	}
	if stats.CoalescingEfficiency != 0.0 {
		t.Errorf("CoalescingEfficiency: got %f, want 0.0", stats.CoalescingEfficiency)
	}
}

func TestGlobalMemoryStatsUpdateEfficiency(t *testing.T) {
	stats := GlobalMemoryStats{
		TotalRequests:     32,
		TotalTransactions: 1,
	}
	stats.UpdateEfficiency()

	if stats.CoalescingEfficiency != 32.0 {
		t.Errorf("CoalescingEfficiency: got %f, want 32.0", stats.CoalescingEfficiency)
	}
}

func TestGlobalMemoryStatsUpdateEfficiencyZero(t *testing.T) {
	stats := GlobalMemoryStats{}
	stats.UpdateEfficiency()

	if stats.CoalescingEfficiency != 0.0 {
		t.Errorf("CoalescingEfficiency: got %f, want 0.0", stats.CoalescingEfficiency)
	}
}

// =========================================================================
// KernelDescriptor tests
// =========================================================================

func TestKernelDescriptorDefaults(t *testing.T) {
	k := DefaultKernelDescriptor()

	if k.Name != "unnamed" {
		t.Errorf("Name: got %q, want %q", k.Name, "unnamed")
	}
	if k.GridDim != [3]int{1, 1, 1} {
		t.Errorf("GridDim: got %v, want [1,1,1]", k.GridDim)
	}
	if k.BlockDim != [3]int{32, 1, 1} {
		t.Errorf("BlockDim: got %v, want [32,1,1]", k.BlockDim)
	}
}

func TestKernelDescriptorTotalThreads(t *testing.T) {
	k := KernelDescriptor{
		GridDim:  [3]int{256, 1, 1},
		BlockDim: [3]int{256, 1, 1},
	}

	if got := k.TotalThreads(); got != 65536 {
		t.Errorf("TotalThreads: got %d, want 65536", got)
	}
}

func TestKernelDescriptorTotalBlocks(t *testing.T) {
	k := KernelDescriptor{
		GridDim: [3]int{4, 2, 3},
	}

	if got := k.TotalBlocks(); got != 24 {
		t.Errorf("TotalBlocks: got %d, want 24", got)
	}
}

func TestKernelDescriptorThreadsPerBlock(t *testing.T) {
	k := KernelDescriptor{
		BlockDim: [3]int{16, 16, 1},
	}

	if got := k.ThreadsPerBlock(); got != 256 {
		t.Errorf("ThreadsPerBlock: got %d, want 256", got)
	}
}

func TestKernelDescriptorTotalThreads3D(t *testing.T) {
	k := KernelDescriptor{
		GridDim:  [3]int{2, 3, 4},
		BlockDim: [3]int{8, 4, 2},
	}

	// grid: 2*3*4 = 24 blocks, block: 8*4*2 = 64 threads, total: 24*64 = 1536
	if got := k.TotalThreads(); got != 1536 {
		t.Errorf("TotalThreads: got %d, want 1536", got)
	}
}

// =========================================================================
// DeviceConfig tests
// =========================================================================

func TestDefaultDeviceConfig(t *testing.T) {
	cfg := DefaultDeviceConfig()

	if cfg.Name != "Generic Accelerator" {
		t.Errorf("Name: got %q, want %q", cfg.Name, "Generic Accelerator")
	}
	if cfg.NumComputeUnits != 4 {
		t.Errorf("NumComputeUnits: got %d, want 4", cfg.NumComputeUnits)
	}
	if cfg.GlobalMemorySize != 16*1024*1024 {
		t.Errorf("GlobalMemorySize: got %d, want %d", cfg.GlobalMemorySize, 16*1024*1024)
	}
}

// =========================================================================
// Vendor config tests
// =========================================================================

func TestDefaultNvidiaConfig(t *testing.T) {
	cfg := DefaultNvidiaConfig()

	if cfg.Name != "NVIDIA H100" {
		t.Errorf("Name: got %q", cfg.Name)
	}
	if cfg.NumComputeUnits != 132 {
		t.Errorf("NumComputeUnits: got %d, want 132", cfg.NumComputeUnits)
	}
	if cfg.Architecture != "nvidia_sm" {
		t.Errorf("Architecture: got %q", cfg.Architecture)
	}
}

func TestDefaultAmdConfig(t *testing.T) {
	cfg := DefaultAmdConfig()

	if cfg.Name != "AMD RX 7900 XTX" {
		t.Errorf("Name: got %q", cfg.Name)
	}
	if cfg.NumComputeUnits != 96 {
		t.Errorf("NumComputeUnits: got %d, want 96", cfg.NumComputeUnits)
	}
	if cfg.NumShaderEngines != 6 {
		t.Errorf("NumShaderEngines: got %d, want 6", cfg.NumShaderEngines)
	}
	if cfg.InfinityCacheSize != 96*1024*1024 {
		t.Errorf("InfinityCacheSize: got %d", cfg.InfinityCacheSize)
	}
}

func TestDefaultTPUConfig(t *testing.T) {
	cfg := DefaultTPUConfig()

	if cfg.Name != "Google TPU v4" {
		t.Errorf("Name: got %q", cfg.Name)
	}
	if cfg.VectorUnitWidth != 128 {
		t.Errorf("VectorUnitWidth: got %d, want 128", cfg.VectorUnitWidth)
	}
	if !cfg.TransposeUnit {
		t.Error("TransposeUnit: want true")
	}
}

func TestDefaultIntelConfig(t *testing.T) {
	cfg := DefaultIntelConfig()

	if cfg.Name != "Intel Arc A770" {
		t.Errorf("Name: got %q", cfg.Name)
	}
	if cfg.NumXeSlices != 8 {
		t.Errorf("NumXeSlices: got %d, want 8", cfg.NumXeSlices)
	}
}

func TestDefaultAppleConfig(t *testing.T) {
	cfg := DefaultAppleConfig()

	if cfg.Name != "Apple M3 Max ANE" {
		t.Errorf("Name: got %q", cfg.Name)
	}
	if !cfg.UnifiedMemory {
		t.Error("UnifiedMemory: want true")
	}
	if cfg.SharedSRAMSize != 32*1024*1024 {
		t.Errorf("SharedSRAMSize: got %d", cfg.SharedSRAMSize)
	}
}

func TestShaderEngineConfigDefaults(t *testing.T) {
	cfg := DefaultShaderEngineConfig()
	if cfg.CUsPerEngine != 16 {
		t.Errorf("CUsPerEngine: got %d, want 16", cfg.CUsPerEngine)
	}
}

func TestXeSliceConfigDefaults(t *testing.T) {
	cfg := DefaultXeSliceConfig()
	if cfg.XeCoresPerSlice != 4 {
		t.Errorf("XeCoresPerSlice: got %d, want 4", cfg.XeCoresPerSlice)
	}
}

func TestICILinkFields(t *testing.T) {
	link := ICILink{TargetChipID: 1, Bandwidth: 500.0, Latency: 500}
	if link.TargetChipID != 1 {
		t.Errorf("TargetChipID: got %d, want 1", link.TargetChipID)
	}
}

// =========================================================================
// DeviceTrace tests
// =========================================================================

func TestDeviceTraceFormat(t *testing.T) {
	trace := DeviceTrace{
		Cycle:              10,
		DeviceName:         "Test GPU",
		DistributorActions: []string{"Block 0 -> SM 0"},
		PendingBlocks:      5,
		ActiveBlocks:       3,
		L2Hits:             100,
		L2Misses:           10,
		MemoryTransactions:  8,
		MemoryBandwidthUsed: 0.452,
		TotalActiveWarps:   24,
		DeviceOccupancy:    0.75,
	}

	formatted := trace.Format()

	if !strings.Contains(formatted, "Cycle 10") {
		t.Errorf("Expected 'Cycle 10' in output: %s", formatted)
	}
	if !strings.Contains(formatted, "Test GPU") {
		t.Errorf("Expected 'Test GPU' in output: %s", formatted)
	}
	if !strings.Contains(formatted, "75.0% occupancy") {
		t.Errorf("Expected '75.0%% occupancy' in output: %s", formatted)
	}
	if !strings.Contains(formatted, "Distributor") {
		t.Errorf("Expected 'Distributor' in output: %s", formatted)
	}
	if !strings.Contains(formatted, "L2: 100 hits") {
		t.Errorf("Expected L2 stats in output: %s", formatted)
	}
}

func TestDeviceTraceFormatNoL2(t *testing.T) {
	trace := DeviceTrace{
		Cycle:      1,
		DeviceName: "TPU",
	}

	formatted := trace.Format()
	if strings.Contains(formatted, "L2:") {
		t.Errorf("Should not show L2 stats when no hits/misses: %s", formatted)
	}
}

func TestDeviceTraceFormatNoDistributor(t *testing.T) {
	trace := DeviceTrace{
		Cycle:      1,
		DeviceName: "TPU",
	}

	formatted := trace.Format()
	if strings.Contains(formatted, "Distributor") {
		t.Errorf("Should not show distributor when no actions: %s", formatted)
	}
}

// =========================================================================
// DeviceStats tests
// =========================================================================

func TestDeviceStatsDefaults(t *testing.T) {
	stats := DeviceStats{}

	if stats.TotalCycles != 0 {
		t.Errorf("TotalCycles: got %d, want 0", stats.TotalCycles)
	}
	if stats.ComputeUtilization != 0.0 {
		t.Errorf("ComputeUtilization: got %f, want 0.0", stats.ComputeUtilization)
	}
}

// =========================================================================
// ANEConfig / TPUConfig embedded field access tests
// =========================================================================

func TestANEConfigEmbeddedAccess(t *testing.T) {
	cfg := DefaultAppleConfig()
	// Should be able to access DeviceConfig fields directly
	if cfg.HostLatency != 0 {
		t.Errorf("HostLatency: got %d, want 0 (unified memory)", cfg.HostLatency)
	}
	if cfg.DMAChannels != 4 {
		t.Errorf("DMAChannels: got %d, want 4", cfg.DMAChannels)
	}
}

func TestTPUConfigEmbeddedAccess(t *testing.T) {
	cfg := DefaultTPUConfig()
	if cfg.HostBandwidth != 500.0 {
		t.Errorf("HostBandwidth: got %f, want 500.0", cfg.HostBandwidth)
	}
	if cfg.ScalarRegisters != 32 {
		t.Errorf("ScalarRegisters: got %d, want 32", cfg.ScalarRegisters)
	}
}
