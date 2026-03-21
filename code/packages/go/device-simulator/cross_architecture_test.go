package devicesimulator

import (
	"testing"

	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Cross-architecture tests -- verify all devices share the same interface
// =========================================================================

// TestAllDevicesImplementInterface verifies that all five device types
// satisfy the AcceleratorDevice interface.
func TestAllDevicesImplementInterface(t *testing.T) {
	var _ AcceleratorDevice = &NvidiaGPU{}
	var _ AcceleratorDevice = &AmdGPU{}
	var _ AcceleratorDevice = &GoogleTPU{}
	var _ AcceleratorDevice = &IntelGPU{}
	var _ AcceleratorDevice = &AppleANE{}
}

// TestAllDevicesMemoryLifecycle runs the same memory lifecycle on every device:
// malloc -> host-to-device copy -> device-to-host copy -> free.
func TestAllDevicesMemoryLifecycle(t *testing.T) {
	devices := []AcceleratorDevice{
		NewNvidiaGPU(nil, 2),
		NewAmdGPU(nil, 2),
		NewGoogleTPU(nil, 4),
		NewIntelGPU(nil, 2),
		NewAppleANE(nil, 2),
	}

	for _, dev := range devices {
		t.Run(dev.Name(), func(t *testing.T) {
			addr, err := dev.Malloc(1024)
			if err != nil {
				t.Fatalf("Malloc: %v", err)
			}

			data := []byte{0xAA, 0xBB, 0xCC, 0xDD}
			_, err = dev.MemcpyHostToDevice(addr, data)
			if err != nil {
				t.Fatalf("MemcpyHostToDevice: %v", err)
			}

			result, _, err := dev.MemcpyDeviceToHost(addr, 4)
			if err != nil {
				t.Fatalf("MemcpyDeviceToHost: %v", err)
			}

			for i, b := range data {
				if result[i] != b {
					t.Errorf("byte %d: got %x, want %x", i, result[i], b)
				}
			}

			dev.Free(addr)
		})
	}
}

// TestAllDevicesResetCycle verifies reset works on all devices.
func TestAllDevicesResetCycle(t *testing.T) {
	devices := []AcceleratorDevice{
		NewNvidiaGPU(nil, 2),
		NewAmdGPU(nil, 2),
		NewGoogleTPU(nil, 4),
		NewIntelGPU(nil, 2),
		NewAppleANE(nil, 2),
	}

	for _, dev := range devices {
		t.Run(dev.Name(), func(t *testing.T) {
			// Do some work
			dev.Malloc(256)
			dev.Reset()

			if !dev.Idle() {
				t.Error("should be idle after reset")
			}

			stats := dev.Stats()
			if stats.TotalCycles != 0 {
				t.Errorf("TotalCycles after reset: got %d", stats.TotalCycles)
			}
		})
	}
}

// TestAllDevicesHaveComputeUnits verifies that every device reports at least
// one compute unit.
func TestAllDevicesHaveComputeUnits(t *testing.T) {
	devices := []AcceleratorDevice{
		NewNvidiaGPU(nil, 4),
		NewAmdGPU(nil, 4),
		NewGoogleTPU(nil, 4),
		NewIntelGPU(nil, 4),
		NewAppleANE(nil, 4),
	}

	for _, dev := range devices {
		t.Run(dev.Name(), func(t *testing.T) {
			cus := dev.ComputeUnits()
			if len(cus) == 0 {
				t.Error("expected at least one compute unit")
			}
		})
	}
}

// TestAllDevicesHaveGlobalMem verifies global memory is accessible.
func TestAllDevicesHaveGlobalMem(t *testing.T) {
	devices := []AcceleratorDevice{
		NewNvidiaGPU(nil, 2),
		NewAmdGPU(nil, 2),
		NewGoogleTPU(nil, 4),
		NewIntelGPU(nil, 2),
		NewAppleANE(nil, 2),
	}

	for _, dev := range devices {
		t.Run(dev.Name(), func(t *testing.T) {
			mem := dev.GlobalMem()
			if mem == nil {
				t.Fatal("GlobalMem() returned nil")
			}
			if mem.Capacity() <= 0 {
				t.Error("expected positive memory capacity")
			}
		})
	}
}

// TestAllDevicesIdleInitially verifies idle state.
func TestAllDevicesIdleInitially(t *testing.T) {
	devices := []AcceleratorDevice{
		NewNvidiaGPU(nil, 2),
		NewAmdGPU(nil, 2),
		NewGoogleTPU(nil, 4),
		NewIntelGPU(nil, 2),
		NewAppleANE(nil, 2),
	}

	for _, dev := range devices {
		t.Run(dev.Name(), func(t *testing.T) {
			if !dev.Idle() {
				t.Error("expected idle initially")
			}
		})
	}
}

// TestAllDevicesConfigAccess verifies that Config() returns valid data.
func TestAllDevicesConfigAccess(t *testing.T) {
	devices := []AcceleratorDevice{
		NewNvidiaGPU(nil, 2),
		NewAmdGPU(nil, 2),
		NewGoogleTPU(nil, 4),
		NewIntelGPU(nil, 2),
		NewAppleANE(nil, 2),
	}

	for _, dev := range devices {
		t.Run(dev.Name(), func(t *testing.T) {
			cfg := dev.Config()
			if cfg.Name == "" {
				t.Error("expected non-empty name in config")
			}
			if cfg.NumComputeUnits <= 0 {
				t.Error("expected positive NumComputeUnits")
			}
		})
	}
}

// TestGPUDevicesKernelExecution tests all GPU-style devices with the same
// kernel (halt program, 2 blocks of 32 threads).
func TestGPUDevicesKernelExecution(t *testing.T) {
	gpuDevices := []AcceleratorDevice{
		NewNvidiaGPU(nil, 2),
		NewAmdGPU(nil, 2),
		NewIntelGPU(nil, 2),
	}

	kernel := KernelDescriptor{
		Name:               "cross_arch_halt",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{2, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	for _, dev := range gpuDevices {
		t.Run(dev.Name(), func(t *testing.T) {
			dev.LaunchKernel(kernel)
			traces := dev.Run(2000)

			if len(traces) == 0 {
				t.Error("expected traces")
			}
			if !dev.Idle() {
				t.Error("should be idle after run")
			}

			stats := dev.Stats()
			if stats.TotalKernelsLaunched != 1 {
				t.Errorf("TotalKernelsLaunched: got %d", stats.TotalKernelsLaunched)
			}
			if stats.TotalBlocksDispatched == 0 {
				t.Error("expected dispatched blocks")
			}
		})
	}
}

// TestDataflowDevicesOperationExecution tests TPU and ANE with the same
// dataflow operation (matmul).
func TestDataflowDevicesOperationExecution(t *testing.T) {
	dataflowDevices := []AcceleratorDevice{
		NewGoogleTPU(nil, 4),
		NewAppleANE(nil, 2),
	}

	kernel := KernelDescriptor{
		Name:       "cross_arch_matmul",
		Operation:  "matmul",
		InputData:  [][]float64{{1, 2}, {3, 4}},
		WeightData: [][]float64{{5, 6}, {7, 8}},
	}

	for _, dev := range dataflowDevices {
		t.Run(dev.Name(), func(t *testing.T) {
			dev.LaunchKernel(kernel)
			traces := dev.Run(1000)

			if len(traces) == 0 {
				t.Error("expected traces")
			}
			if !dev.Idle() {
				t.Error("should be idle after run")
			}

			stats := dev.Stats()
			if stats.TotalKernelsLaunched != 1 {
				t.Errorf("TotalKernelsLaunched: got %d", stats.TotalKernelsLaunched)
			}
		})
	}
}

// TestDeviceTraceFormatAcrossArchitectures verifies that trace formatting
// works for all device types.
func TestDeviceTraceFormatAcrossArchitectures(t *testing.T) {
	devices := []AcceleratorDevice{
		NewNvidiaGPU(nil, 1),
		NewAmdGPU(nil, 1),
		NewGoogleTPU(nil, 4),
		NewIntelGPU(nil, 1),
		NewAppleANE(nil, 1),
	}

	for _, dev := range devices {
		t.Run(dev.Name(), func(t *testing.T) {
			edge := dev.GlobalMem().stats // just need any edge
			_ = edge
			// Run one step
			traces := dev.Run(1)
			if len(traces) > 0 {
				formatted := traces[0].Format()
				if len(formatted) == 0 {
					t.Error("expected non-empty trace format")
				}
			}
		})
	}
}
