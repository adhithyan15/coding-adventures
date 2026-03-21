package devicesimulator

import (
	"testing"

	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

func TestNvidiaGPUCreation(t *testing.T) {
	gpu := NewNvidiaGPU(nil, 4)

	if gpu.Name() != "NVIDIA GPU (4 SMs)" {
		t.Errorf("Name: got %q", gpu.Name())
	}
	if gpu.Config().NumComputeUnits != 4 {
		t.Errorf("NumComputeUnits: got %d, want 4", gpu.Config().NumComputeUnits)
	}
	if len(gpu.ComputeUnits()) != 4 {
		t.Errorf("ComputeUnits count: got %d, want 4", len(gpu.ComputeUnits()))
	}
}

func TestNvidiaGPUCreationWithConfig(t *testing.T) {
	cfg := DeviceConfig{
		Name:                   "Custom NVIDIA",
		Architecture:           "nvidia_sm",
		NumComputeUnits:        2,
		L2CacheSize:            4096,
		L2CacheLatency:         200,
		L2CacheAssociativity:   4,
		L2CacheLineSize:        64,
		GlobalMemorySize:       1024 * 1024,
		GlobalMemoryBandwidth:  1000.0,
		GlobalMemoryLatency:    400,
		MemoryChannels:         4,
		HostBandwidth:          64.0,
		HostLatency:            100,
		WorkDistributionPolicy: "round_robin",
	}

	gpu := NewNvidiaGPU(&cfg, 0)
	if gpu.Name() != "Custom NVIDIA" {
		t.Errorf("Name: got %q", gpu.Name())
	}
}

func TestNvidiaGPUMemoryOperations(t *testing.T) {
	gpu := NewNvidiaGPU(nil, 2)

	addr, err := gpu.Malloc(1024)
	if err != nil {
		t.Fatalf("Malloc: %v", err)
	}

	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}

	cycles, err := gpu.MemcpyHostToDevice(addr, data)
	if err != nil {
		t.Fatalf("MemcpyHostToDevice: %v", err)
	}
	if cycles <= 0 {
		t.Errorf("expected positive cycles, got %d", cycles)
	}

	result, readCycles, err := gpu.MemcpyDeviceToHost(addr, 256)
	if err != nil {
		t.Fatalf("MemcpyDeviceToHost: %v", err)
	}
	if readCycles <= 0 {
		t.Errorf("expected positive cycles, got %d", readCycles)
	}
	if result[0] != 0 || result[255] != 255 {
		t.Error("data mismatch after roundtrip")
	}

	gpu.Free(addr)
}

func TestNvidiaGPUIdleInitially(t *testing.T) {
	gpu := NewNvidiaGPU(nil, 2)
	if !gpu.Idle() {
		t.Error("expected idle initially")
	}
}

func TestNvidiaGPULaunchAndRun(t *testing.T) {
	gpu := NewNvidiaGPU(nil, 2)

	kernel := KernelDescriptor{
		Name:               "test_halt",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{2, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	gpu.LaunchKernel(kernel)

	if gpu.Idle() {
		t.Error("should not be idle after launch")
	}

	traces := gpu.Run(1000)
	if len(traces) == 0 {
		t.Error("expected at least one trace")
	}

	if !gpu.Idle() {
		t.Error("should be idle after run")
	}
}

func TestNvidiaGPUStats(t *testing.T) {
	gpu := NewNvidiaGPU(nil, 2)

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
	if stats.TotalCycles <= 0 {
		t.Errorf("TotalCycles should be positive, got %d", stats.TotalCycles)
	}
}

func TestNvidiaGPUReset(t *testing.T) {
	gpu := NewNvidiaGPU(nil, 2)

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

func TestNvidiaGPUGlobalMemAccess(t *testing.T) {
	gpu := NewNvidiaGPU(nil, 2)
	mem := gpu.GlobalMem()
	if mem == nil {
		t.Fatal("GlobalMem() returned nil")
	}
	if mem.Capacity() != 16*1024*1024 {
		t.Errorf("GlobalMem capacity: got %d", mem.Capacity())
	}
}

func TestNvidiaGPUStepReturnsTrace(t *testing.T) {
	gpu := NewNvidiaGPU(nil, 2)

	kernel := KernelDescriptor{
		Name:               "test",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{1, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	gpu.LaunchKernel(kernel)
	edge := gpu.clk.Tick()
	trace := gpu.Step(edge)

	if trace.DeviceName != gpu.Name() {
		t.Errorf("DeviceName: got %q, want %q", trace.DeviceName, gpu.Name())
	}
	if trace.Cycle != 1 {
		t.Errorf("Cycle: got %d, want 1", trace.Cycle)
	}
}

func TestNvidiaGPUMultipleKernels(t *testing.T) {
	gpu := NewNvidiaGPU(nil, 4)

	for i := 0; i < 3; i++ {
		kernel := KernelDescriptor{
			Name:               "test",
			Program:            []gpucore.Instruction{gpucore.Halt()},
			GridDim:            [3]int{1, 1, 1},
			BlockDim:           [3]int{32, 1, 1},
			RegistersPerThread: 32,
		}
		gpu.LaunchKernel(kernel)
	}

	gpu.Run(5000)

	stats := gpu.Stats()
	if stats.TotalKernelsLaunched != 3 {
		t.Errorf("TotalKernelsLaunched: got %d, want 3", stats.TotalKernelsLaunched)
	}
}
