package devicesimulator

import (
	"testing"

	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

func TestAmdGPUCreation(t *testing.T) {
	gpu := NewAmdGPU(nil, 4)

	if gpu.Name() != "AMD GPU (4 CUs)" {
		t.Errorf("Name: got %q", gpu.Name())
	}
	if len(gpu.ComputeUnits()) != 4 {
		t.Errorf("ComputeUnits: got %d, want 4", len(gpu.ComputeUnits()))
	}
}

func TestAmdGPUCreationWithConfig(t *testing.T) {
	cfg := DeviceConfig{
		Name:                   "Custom AMD",
		Architecture:           "amd_cu",
		NumComputeUnits:        6,
		L2CacheSize:            4096,
		L2CacheLatency:         150,
		L2CacheAssociativity:   4,
		L2CacheLineSize:        64,
		GlobalMemorySize:       1024 * 1024,
		GlobalMemoryBandwidth:  960.0,
		GlobalMemoryLatency:    350,
		MemoryChannels:         4,
		HostBandwidth:          32.0,
		HostLatency:            100,
		WorkDistributionPolicy: "round_robin",
	}

	gpu := NewAmdGPU(&cfg, 0)
	if gpu.Name() != "Custom AMD" {
		t.Errorf("Name: got %q", gpu.Name())
	}
	if len(gpu.ComputeUnits()) != 6 {
		t.Errorf("ComputeUnits: got %d, want 6", len(gpu.ComputeUnits()))
	}
}

func TestAmdGPUShaderEngines(t *testing.T) {
	gpu := NewAmdGPU(nil, 4)

	engines := gpu.ShaderEngines()
	if len(engines) == 0 {
		t.Fatal("expected at least one shader engine")
	}

	// All engines should be idle initially
	for _, se := range engines {
		if !se.Idle() {
			t.Errorf("Shader Engine %d should be idle", se.EngineID)
		}
	}
}

func TestAmdGPUMemoryOperations(t *testing.T) {
	gpu := NewAmdGPU(nil, 2)

	addr, err := gpu.Malloc(512)
	if err != nil {
		t.Fatalf("Malloc: %v", err)
	}

	data := []byte{0xDE, 0xAD, 0xBE, 0xEF}
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
	if result[0] != 0xDE || result[3] != 0xEF {
		t.Error("data mismatch after roundtrip")
	}

	gpu.Free(addr)
}

func TestAmdGPULaunchAndRun(t *testing.T) {
	gpu := NewAmdGPU(nil, 2)

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
		t.Error("expected at least one trace")
	}
	if !gpu.Idle() {
		t.Error("should be idle after run")
	}
}

func TestAmdGPUStats(t *testing.T) {
	gpu := NewAmdGPU(nil, 2)

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

func TestAmdGPUReset(t *testing.T) {
	gpu := NewAmdGPU(nil, 2)

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
	stats := gpu.Stats()
	if stats.TotalCycles != 0 {
		t.Errorf("after reset, TotalCycles: got %d, want 0", stats.TotalCycles)
	}
}

func TestAmdGPUGlobalMem(t *testing.T) {
	gpu := NewAmdGPU(nil, 2)
	mem := gpu.GlobalMem()
	if mem == nil {
		t.Fatal("GlobalMem() returned nil")
	}
}

func TestAmdGPUIdleInitially(t *testing.T) {
	gpu := NewAmdGPU(nil, 2)
	if !gpu.Idle() {
		t.Error("expected idle initially")
	}
}
