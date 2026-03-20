package devicesimulator

import (
	"testing"

	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

func TestIntelGPUCreation(t *testing.T) {
	gpu := NewIntelGPU(nil, 4)

	if gpu.Name() != "Intel GPU (4 Xe-Cores)" {
		t.Errorf("Name: got %q", gpu.Name())
	}
	if len(gpu.ComputeUnits()) != 4 {
		t.Errorf("ComputeUnits: got %d, want 4", len(gpu.ComputeUnits()))
	}
}

func TestIntelGPUCreationWithConfig(t *testing.T) {
	cfg := DeviceConfig{
		Name:                   "Custom Intel",
		Architecture:           "intel_xe_core",
		NumComputeUnits:        8,
		L2CacheSize:            4096,
		L2CacheLatency:         180,
		L2CacheAssociativity:   4,
		L2CacheLineSize:        64,
		GlobalMemorySize:       1024 * 1024,
		GlobalMemoryBandwidth:  512.0,
		GlobalMemoryLatency:    350,
		MemoryChannels:         4,
		HostBandwidth:          32.0,
		HostLatency:            100,
		WorkDistributionPolicy: "round_robin",
	}

	gpu := NewIntelGPU(&cfg, 0)
	if gpu.Name() != "Custom Intel" {
		t.Errorf("Name: got %q", gpu.Name())
	}
	if len(gpu.ComputeUnits()) != 8 {
		t.Errorf("ComputeUnits: got %d, want 8", len(gpu.ComputeUnits()))
	}
}

func TestIntelGPUXeSlices(t *testing.T) {
	gpu := NewIntelGPU(nil, 4)

	slices := gpu.XeSlices()
	if len(slices) == 0 {
		t.Fatal("expected at least one Xe-Slice")
	}

	for _, slice := range slices {
		if !slice.Idle() {
			t.Errorf("Xe-Slice %d should be idle initially", slice.SliceID)
		}
	}
}

func TestIntelGPUMemoryOperations(t *testing.T) {
	gpu := NewIntelGPU(nil, 2)

	addr, err := gpu.Malloc(512)
	if err != nil {
		t.Fatalf("Malloc: %v", err)
	}

	data := []byte{0xCA, 0xFE, 0xBA, 0xBE}
	cycles, err := gpu.MemcpyHostToDevice(addr, data)
	if err != nil {
		t.Fatalf("MemcpyHostToDevice: %v", err)
	}
	if cycles <= 0 {
		t.Errorf("expected positive cycles, got %d", cycles)
	}

	result, _, err := gpu.MemcpyDeviceToHost(addr, 4)
	if err != nil {
		t.Fatalf("MemcpyDeviceToHost: %v", err)
	}
	if result[0] != 0xCA || result[3] != 0xBE {
		t.Error("data mismatch")
	}

	gpu.Free(addr)
}

func TestIntelGPUIdleInitially(t *testing.T) {
	gpu := NewIntelGPU(nil, 2)
	if !gpu.Idle() {
		t.Error("expected idle initially")
	}
}

func TestIntelGPULaunchAndRun(t *testing.T) {
	gpu := NewIntelGPU(nil, 2)

	kernel := KernelDescriptor{
		Name:               "test_halt",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{2, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	gpu.LaunchKernel(kernel)
	traces := gpu.Run(1000)

	if len(traces) == 0 {
		t.Error("expected traces")
	}
	if !gpu.Idle() {
		t.Error("should be idle after run")
	}
}

func TestIntelGPUStats(t *testing.T) {
	gpu := NewIntelGPU(nil, 2)

	kernel := KernelDescriptor{
		Name:               "test",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{2, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	gpu.LaunchKernel(kernel)
	gpu.Run(1000)

	stats := gpu.Stats()
	if stats.TotalKernelsLaunched != 1 {
		t.Errorf("TotalKernelsLaunched: got %d, want 1", stats.TotalKernelsLaunched)
	}
}

func TestIntelGPUReset(t *testing.T) {
	gpu := NewIntelGPU(nil, 2)

	kernel := KernelDescriptor{
		Name:               "test",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{2, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	gpu.LaunchKernel(kernel)
	gpu.Run(1000)
	gpu.Reset()

	if !gpu.Idle() {
		t.Error("should be idle after reset")
	}
}

func TestIntelGPUGlobalMem(t *testing.T) {
	gpu := NewIntelGPU(nil, 2)
	mem := gpu.GlobalMem()
	if mem == nil {
		t.Fatal("GlobalMem() returned nil")
	}
}
