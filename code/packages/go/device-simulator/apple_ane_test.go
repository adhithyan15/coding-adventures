package devicesimulator

import (
	"testing"
)

func TestAppleANECreation(t *testing.T) {
	ane := NewAppleANE(nil, 4)

	if ane.Name() != "Apple ANE (4 cores)" {
		t.Errorf("Name: got %q", ane.Name())
	}
	if len(ane.ComputeUnits()) != 4 {
		t.Errorf("ComputeUnits: got %d, want 4", len(ane.ComputeUnits()))
	}
}

func TestAppleANECreationWithConfig(t *testing.T) {
	cfg := DeviceConfig{
		Name:                   "Custom ANE",
		Architecture:           "apple_ane_core",
		NumComputeUnits:        8,
		GlobalMemorySize:       4 * 1024 * 1024,
		GlobalMemoryBandwidth:  200.0,
		GlobalMemoryLatency:    100,
		MemoryChannels:         8,
		HostBandwidth:          200.0,
		HostLatency:            0,
		UnifiedMemory:          true,
		WorkDistributionPolicy: "scheduled",
	}

	ane := NewAppleANE(&cfg, 0)
	if ane.Name() != "Custom ANE" {
		t.Errorf("Name: got %q", ane.Name())
	}
	if len(ane.ComputeUnits()) != 8 {
		t.Errorf("ComputeUnits: got %d, want 8", len(ane.ComputeUnits()))
	}
}

func TestAppleANEUnifiedMemory(t *testing.T) {
	ane := NewAppleANE(nil, 4)

	if !ane.IsUnifiedMemory() {
		t.Error("ANE should always report unified memory")
	}

	addr, _ := ane.Malloc(1024)
	data := []byte{1, 2, 3, 4}

	// Unified memory: zero-copy
	cycles, err := ane.MemcpyHostToDevice(addr, data)
	if err != nil {
		t.Fatalf("MemcpyHostToDevice: %v", err)
	}
	if cycles != 0 {
		t.Errorf("unified memory copy should be zero-cost, got %d cycles", cycles)
	}

	result, readCycles, err := ane.MemcpyDeviceToHost(addr, 4)
	if err != nil {
		t.Fatalf("MemcpyDeviceToHost: %v", err)
	}
	if readCycles != 0 {
		t.Errorf("unified memory copy should be zero-cost, got %d cycles", readCycles)
	}
	if result[0] != 1 || result[3] != 4 {
		t.Error("data mismatch")
	}

	ane.Free(addr)
}

func TestAppleANEIdleInitially(t *testing.T) {
	ane := NewAppleANE(nil, 4)
	if !ane.Idle() {
		t.Error("expected idle initially")
	}
}

func TestAppleANELaunchAndRun(t *testing.T) {
	ane := NewAppleANE(nil, 2)

	kernel := KernelDescriptor{
		Name:       "conv2d",
		Operation:  "conv2d",
		InputData:  [][]float64{{1, 2}, {3, 4}},
		WeightData: [][]float64{{1, 0}, {0, 1}},
	}

	ane.LaunchKernel(kernel)

	if ane.Idle() {
		t.Error("should not be idle after launch")
	}

	traces := ane.Run(500)
	if len(traces) == 0 {
		t.Error("expected at least one trace")
	}

	if !ane.Idle() {
		t.Error("should be idle after run")
	}
}

func TestAppleANEStats(t *testing.T) {
	ane := NewAppleANE(nil, 2)

	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  [][]float64{{1}},
		WeightData: [][]float64{{1}},
	}

	ane.LaunchKernel(kernel)
	ane.Run(500)

	stats := ane.Stats()
	if stats.TotalKernelsLaunched != 1 {
		t.Errorf("TotalKernelsLaunched: got %d, want 1", stats.TotalKernelsLaunched)
	}
	if stats.TotalCycles <= 0 {
		t.Errorf("TotalCycles should be positive, got %d", stats.TotalCycles)
	}
	if stats.TotalBlocksDispatched == 0 {
		t.Error("TotalBlocksDispatched should be > 0")
	}
}

func TestAppleANEReset(t *testing.T) {
	ane := NewAppleANE(nil, 2)

	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  [][]float64{{1}},
		WeightData: [][]float64{{1}},
	}

	ane.LaunchKernel(kernel)
	ane.Run(500)
	ane.Reset()

	if !ane.Idle() {
		t.Error("should be idle after reset")
	}
	stats := ane.Stats()
	if stats.TotalCycles != 0 {
		t.Errorf("after reset, TotalCycles: got %d, want 0", stats.TotalCycles)
	}
}

func TestAppleANEGlobalMem(t *testing.T) {
	ane := NewAppleANE(nil, 2)
	mem := ane.GlobalMem()
	if mem == nil {
		t.Fatal("GlobalMem() returned nil")
	}
}

func TestAppleANEStepReturnsTrace(t *testing.T) {
	ane := NewAppleANE(nil, 2)

	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  [][]float64{{1}},
		WeightData: [][]float64{{1}},
	}

	ane.LaunchKernel(kernel)
	edge := ane.clk.Tick()
	trace := ane.Step(edge)

	if trace.DeviceName != ane.Name() {
		t.Errorf("DeviceName: got %q", trace.DeviceName)
	}
	if trace.Cycle != 1 {
		t.Errorf("Cycle: got %d, want 1", trace.Cycle)
	}
}

func TestAppleANEMultiCoreOperation(t *testing.T) {
	ane := NewAppleANE(nil, 4)

	// 3 rows -> 3 cores active
	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  [][]float64{{1, 2}, {3, 4}, {5, 6}},
		WeightData: [][]float64{{1, 0}, {0, 1}},
	}

	ane.LaunchKernel(kernel)
	ane.Run(500)

	if !ane.Idle() {
		t.Error("should complete multi-core operation")
	}

	stats := ane.Stats()
	if stats.TotalBlocksDispatched == 0 {
		t.Error("expected dispatched operations")
	}
}
