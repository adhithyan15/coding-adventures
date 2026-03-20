package devicesimulator

import (
	"testing"
)

func TestGoogleTPUCreation(t *testing.T) {
	tpu := NewGoogleTPU(nil, 4)

	if tpu.Name() != "Google TPU (MXU 4x4)" {
		t.Errorf("Name: got %q", tpu.Name())
	}
	if len(tpu.ComputeUnits()) != 1 {
		t.Errorf("ComputeUnits: got %d, want 1", len(tpu.ComputeUnits()))
	}
}

func TestGoogleTPUCreationWithConfig(t *testing.T) {
	cfg := DeviceConfig{
		Name:                   "Custom TPU",
		Architecture:           "google_mxu",
		NumComputeUnits:        1,
		GlobalMemorySize:       1024 * 1024,
		GlobalMemoryBandwidth:  1200.0,
		GlobalMemoryLatency:    300,
		MemoryChannels:         4,
		HostBandwidth:          500.0,
		HostLatency:            100,
		WorkDistributionPolicy: "sequential",
	}

	tpu := NewGoogleTPU(&cfg, 4)
	if tpu.Name() != "Custom TPU" {
		t.Errorf("Name: got %q", tpu.Name())
	}
}

func TestGoogleTPUMemoryOperations(t *testing.T) {
	tpu := NewGoogleTPU(nil, 4)

	addr, err := tpu.Malloc(512)
	if err != nil {
		t.Fatalf("Malloc: %v", err)
	}

	data := []byte{1, 2, 3, 4}
	cycles, err := tpu.MemcpyHostToDevice(addr, data)
	if err != nil {
		t.Fatalf("MemcpyHostToDevice: %v", err)
	}
	if cycles <= 0 {
		t.Errorf("expected positive cycles, got %d", cycles)
	}

	result, _, err := tpu.MemcpyDeviceToHost(addr, 4)
	if err != nil {
		t.Fatalf("MemcpyDeviceToHost: %v", err)
	}
	if result[0] != 1 || result[3] != 4 {
		t.Error("data mismatch")
	}

	tpu.Free(addr)
}

func TestGoogleTPUIdleInitially(t *testing.T) {
	tpu := NewGoogleTPU(nil, 4)
	if !tpu.Idle() {
		t.Error("expected idle initially")
	}
}

func TestGoogleTPULaunchMatmul(t *testing.T) {
	tpu := NewGoogleTPU(nil, 4)

	kernel := KernelDescriptor{
		Name:       "matmul",
		Operation:  "matmul",
		InputData:  [][]float64{{1, 2}, {3, 4}},
		WeightData: [][]float64{{5, 6}, {7, 8}},
	}

	tpu.LaunchKernel(kernel)

	if tpu.Idle() {
		t.Error("should not be idle after launch")
	}

	traces := tpu.Run(500)
	if len(traces) == 0 {
		t.Error("expected at least one trace")
	}

	if !tpu.Idle() {
		t.Error("should be idle after run")
	}
}

func TestGoogleTPUStats(t *testing.T) {
	tpu := NewGoogleTPU(nil, 4)

	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  [][]float64{{1}},
		WeightData: [][]float64{{1}},
	}

	tpu.LaunchKernel(kernel)
	tpu.Run(500)

	stats := tpu.Stats()
	if stats.TotalKernelsLaunched != 1 {
		t.Errorf("TotalKernelsLaunched: got %d, want 1", stats.TotalKernelsLaunched)
	}
	if stats.TotalCycles <= 0 {
		t.Errorf("TotalCycles should be positive, got %d", stats.TotalCycles)
	}
}

func TestGoogleTPUReset(t *testing.T) {
	tpu := NewGoogleTPU(nil, 4)

	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  [][]float64{{1}},
		WeightData: [][]float64{{1}},
	}

	tpu.LaunchKernel(kernel)
	tpu.Run(500)
	tpu.Reset()

	if !tpu.Idle() {
		t.Error("should be idle after reset")
	}
	stats := tpu.Stats()
	if stats.TotalCycles != 0 {
		t.Errorf("after reset, TotalCycles: got %d, want 0", stats.TotalCycles)
	}
}

func TestGoogleTPUGlobalMem(t *testing.T) {
	tpu := NewGoogleTPU(nil, 4)
	mem := tpu.GlobalMem()
	if mem == nil {
		t.Fatal("GlobalMem() returned nil")
	}
}

func TestGoogleTPUStepReturnsTrace(t *testing.T) {
	tpu := NewGoogleTPU(nil, 4)

	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  [][]float64{{1}},
		WeightData: [][]float64{{1}},
	}

	tpu.LaunchKernel(kernel)
	edge := tpu.clk.Tick()
	trace := tpu.Step(edge)

	if trace.DeviceName != tpu.Name() {
		t.Errorf("DeviceName: got %q", trace.DeviceName)
	}
	if trace.Cycle != 1 {
		t.Errorf("Cycle: got %d, want 1", trace.Cycle)
	}
}

func TestGoogleTPULargeMatrix(t *testing.T) {
	tpu := NewGoogleTPU(nil, 2)

	// 8x8 matrix with MXU size 2 -> many tiles
	input := make([][]float64, 8)
	for i := range input {
		input[i] = make([]float64, 8)
		for j := range input[i] {
			input[i][j] = float64(i*8 + j)
		}
	}
	weights := make([][]float64, 8)
	for i := range weights {
		weights[i] = make([]float64, 8)
		weights[i][i] = 1.0 // identity
	}

	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  input,
		WeightData: weights,
	}

	tpu.LaunchKernel(kernel)
	tpu.Run(5000)

	if !tpu.Idle() {
		t.Error("should complete large matrix operation")
	}

	stats := tpu.Stats()
	if stats.TotalBlocksDispatched == 0 {
		t.Error("expected dispatched tiles")
	}
}
