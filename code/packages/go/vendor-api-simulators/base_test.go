package vendorapisimulators

import (
	"testing"

	cr "github.com/adhithyan15/coding-adventures/code/packages/go/compute-runtime"
)

// =========================================================================
// BaseVendorSimulator tests
// =========================================================================

func TestInitBaseDefaultDevices(t *testing.T) {
	base, err := InitBase(nil, "")
	if err != nil {
		t.Fatalf("InitBase failed: %v", err)
	}
	if base.Instance == nil {
		t.Fatal("expected non-nil Instance")
	}
	if len(base.PhysicalDevices) == 0 {
		t.Fatal("expected at least one physical device")
	}
	if base.PhysicalDevice == nil {
		t.Fatal("expected non-nil selected PhysicalDevice")
	}
	if base.LogicalDevice == nil {
		t.Fatal("expected non-nil LogicalDevice")
	}
	if base.ComputeQueue == nil {
		t.Fatal("expected non-nil ComputeQueue")
	}
	if base.MemoryManager == nil {
		t.Fatal("expected non-nil MemoryManager")
	}
}

func TestInitBaseWithNvidiaHint(t *testing.T) {
	base, err := InitBase(nil, "nvidia")
	if err != nil {
		t.Fatalf("InitBase failed: %v", err)
	}
	if base.PhysicalDevice.Vendor() != "nvidia" {
		t.Errorf("expected nvidia vendor, got %s", base.PhysicalDevice.Vendor())
	}
}

func TestInitBaseWithAppleHint(t *testing.T) {
	base, err := InitBase(nil, "apple")
	if err != nil {
		t.Fatalf("InitBase failed: %v", err)
	}
	if base.PhysicalDevice.Vendor() != "apple" {
		t.Errorf("expected apple vendor, got %s", base.PhysicalDevice.Vendor())
	}
}

func TestInitBaseWithDeviceType(t *testing.T) {
	dt := cr.DeviceTypeGPU
	base, err := InitBase(&dt, "")
	if err != nil {
		t.Fatalf("InitBase failed: %v", err)
	}
	if base.PhysicalDevice.DeviceType() != cr.DeviceTypeGPU {
		t.Errorf("expected GPU device type, got %v", base.PhysicalDevice.DeviceType())
	}
}

func TestInitBaseWithVendorAndType(t *testing.T) {
	dt := cr.DeviceTypeGPU
	base, err := InitBase(&dt, "nvidia")
	if err != nil {
		t.Fatalf("InitBase failed: %v", err)
	}
	if base.PhysicalDevice.Vendor() != "nvidia" {
		t.Errorf("expected nvidia, got %s", base.PhysicalDevice.Vendor())
	}
}

func TestSelectDeviceFallthrough(t *testing.T) {
	// Test pass 4: no match on vendor or type, takes first device
	base, err := InitBase(nil, "nonexistent_vendor")
	if err != nil {
		t.Fatalf("InitBase failed: %v", err)
	}
	if base.PhysicalDevice == nil {
		t.Fatal("expected a device even with non-matching vendor hint")
	}
}

func TestCreateAndSubmitCB(t *testing.T) {
	base, err := InitBase(nil, "")
	if err != nil {
		t.Fatalf("InitBase failed: %v", err)
	}

	// Allocate a buffer
	buf, err := base.MemoryManager.Allocate(64, DefaultMemType(), DefaultUsage())
	if err != nil {
		t.Fatalf("allocate failed: %v", err)
	}

	// Submit a fill command
	cb, err := base.CreateAndSubmitCB(func(cb *cr.CommandBuffer) error {
		return cb.CmdFillBuffer(buf, 0xAA, 0, 64)
	}, nil)
	if err != nil {
		t.Fatalf("CreateAndSubmitCB failed: %v", err)
	}
	if cb == nil {
		t.Fatal("expected non-nil command buffer")
	}

	// Verify the command buffer is in complete state
	if cb.State() != cr.CommandBufferStateComplete {
		t.Errorf("expected complete state, got %v", cb.State())
	}
}

func TestCreateAndSubmitCBWithExplicitQueue(t *testing.T) {
	base, err := InitBase(nil, "")
	if err != nil {
		t.Fatalf("InitBase failed: %v", err)
	}

	buf, err := base.MemoryManager.Allocate(32, DefaultMemType(), DefaultUsage())
	if err != nil {
		t.Fatalf("allocate failed: %v", err)
	}

	_, err = base.CreateAndSubmitCB(func(cb *cr.CommandBuffer) error {
		return cb.CmdFillBuffer(buf, 0, 0, 32)
	}, base.ComputeQueue)
	if err != nil {
		t.Fatalf("CreateAndSubmitCB with explicit queue failed: %v", err)
	}
}

func TestDefaultMemType(t *testing.T) {
	mt := DefaultMemType()
	if !mt.Has(cr.MemoryTypeDeviceLocal) {
		t.Error("expected DEVICE_LOCAL flag")
	}
	if !mt.Has(cr.MemoryTypeHostVisible) {
		t.Error("expected HOST_VISIBLE flag")
	}
	if !mt.Has(cr.MemoryTypeHostCoherent) {
		t.Error("expected HOST_COHERENT flag")
	}
}

func TestDefaultUsage(t *testing.T) {
	u := DefaultUsage()
	if !u.Has(cr.BufferUsageStorage) {
		t.Error("expected STORAGE flag")
	}
	if !u.Has(cr.BufferUsageTransferSrc) {
		t.Error("expected TRANSFER_SRC flag")
	}
	if !u.Has(cr.BufferUsageTransferDst) {
		t.Error("expected TRANSFER_DST flag")
	}
}

func TestMultiplePhysicalDevices(t *testing.T) {
	base, err := InitBase(nil, "")
	if err != nil {
		t.Fatalf("InitBase failed: %v", err)
	}
	// Default devices should include multiple vendors
	if len(base.PhysicalDevices) < 2 {
		t.Errorf("expected at least 2 physical devices, got %d", len(base.PhysicalDevices))
	}
}

func TestPhysicalDeviceProperties(t *testing.T) {
	base, err := InitBase(nil, "nvidia")
	if err != nil {
		t.Fatalf("InitBase failed: %v", err)
	}
	pd := base.PhysicalDevice
	if pd.Name() == "" {
		t.Error("expected non-empty device name")
	}
	if pd.Vendor() != "nvidia" {
		t.Errorf("expected nvidia vendor, got %s", pd.Vendor())
	}
}
