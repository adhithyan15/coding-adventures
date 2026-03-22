package devicedriverframework

import "testing"

func TestRegistryRegisterAndLookup(t *testing.T) {
	reg := NewDeviceRegistry()
	dev := &DeviceBase{Name: "display0", Type: DeviceCharacter, Major: 1, Minor: 0}
	if err := reg.Register(dev); err != nil {
		t.Fatalf("Register failed: %v", err)
	}
	found, ok := reg.Lookup("display0")
	if !ok || found != dev {
		t.Error("Lookup should return the registered device")
	}
}

func TestRegistryLookupByMajorMinor(t *testing.T) {
	reg := NewDeviceRegistry()
	dev := &DeviceBase{Name: "display0", Type: DeviceCharacter, Major: 1, Minor: 0}
	reg.Register(dev)
	found, ok := reg.LookupByMajorMinor(1, 0)
	if !ok || found != dev {
		t.Error("LookupByMajorMinor should return the registered device")
	}
}

func TestRegistryLookupNonexistent(t *testing.T) {
	reg := NewDeviceRegistry()
	_, ok := reg.Lookup("nonexistent")
	if ok {
		t.Error("Lookup for nonexistent name should return false")
	}
}

func TestRegistryLookupByMajorMinorNonexistent(t *testing.T) {
	reg := NewDeviceRegistry()
	_, ok := reg.LookupByMajorMinor(99, 99)
	if ok {
		t.Error("LookupByMajorMinor for nonexistent pair should return false")
	}
}

func TestRegistryDuplicateNameError(t *testing.T) {
	reg := NewDeviceRegistry()
	reg.Register(&DeviceBase{Name: "dup", Major: 1, Minor: 0})
	err := reg.Register(&DeviceBase{Name: "dup", Major: 2, Minor: 0})
	if err == nil {
		t.Error("Registering duplicate name should return error")
	}
}

func TestRegistryDuplicateMajorMinorError(t *testing.T) {
	reg := NewDeviceRegistry()
	reg.Register(&DeviceBase{Name: "dev_a", Major: 3, Minor: 0})
	err := reg.Register(&DeviceBase{Name: "dev_b", Major: 3, Minor: 0})
	if err == nil {
		t.Error("Registering duplicate major/minor should return error")
	}
}

func TestRegistryUnregister(t *testing.T) {
	reg := NewDeviceRegistry()
	dev := &DeviceBase{Name: "disk0", Major: 3, Minor: 0}
	reg.Register(dev)
	removed, ok := reg.Unregister("disk0")
	if !ok || removed != dev {
		t.Error("Unregister should return the removed device")
	}
	_, ok = reg.Lookup("disk0")
	if ok {
		t.Error("Lookup after Unregister should return false")
	}
	_, ok = reg.LookupByMajorMinor(3, 0)
	if ok {
		t.Error("LookupByMajorMinor after Unregister should return false")
	}
}

func TestRegistryUnregisterNonexistent(t *testing.T) {
	reg := NewDeviceRegistry()
	_, ok := reg.Unregister("ghost")
	if ok {
		t.Error("Unregister for nonexistent should return false")
	}
}

func TestRegistryListDevices(t *testing.T) {
	reg := NewDeviceRegistry()
	reg.Register(&DeviceBase{Name: "a", Major: 1, Minor: 0})
	reg.Register(&DeviceBase{Name: "b", Major: 2, Minor: 0})
	reg.Register(&DeviceBase{Name: "c", Major: 3, Minor: 0})
	devices := reg.ListDevices()
	if len(devices) != 3 {
		t.Errorf("ListDevices returned %d devices, want 3", len(devices))
	}
}

func TestRegistryListDevicesEmpty(t *testing.T) {
	reg := NewDeviceRegistry()
	devices := reg.ListDevices()
	if len(devices) != 0 {
		t.Errorf("ListDevices on empty registry returned %d, want 0", len(devices))
	}
}

func TestRegistryListByType(t *testing.T) {
	reg := NewDeviceRegistry()
	reg.Register(&DeviceBase{Name: "kb0", Type: DeviceCharacter, Major: 2, Minor: 0})
	reg.Register(&DeviceBase{Name: "disk0", Type: DeviceBlock, Major: 3, Minor: 0})
	reg.Register(&DeviceBase{Name: "nic0", Type: DeviceNetwork, Major: 4, Minor: 0})

	chars := reg.ListByType(DeviceCharacter)
	if len(chars) != 1 {
		t.Errorf("ListByType(Character) returned %d, want 1", len(chars))
	}
	blocks := reg.ListByType(DeviceBlock)
	if len(blocks) != 1 {
		t.Errorf("ListByType(Block) returned %d, want 1", len(blocks))
	}
	nets := reg.ListByType(DeviceNetwork)
	if len(nets) != 1 {
		t.Errorf("ListByType(Network) returned %d, want 1", len(nets))
	}
}

func TestRegistryListByTypeEmpty(t *testing.T) {
	reg := NewDeviceRegistry()
	reg.Register(&DeviceBase{Name: "kb0", Type: DeviceCharacter, Major: 2, Minor: 0})
	blocks := reg.ListByType(DeviceBlock)
	if len(blocks) != 0 {
		t.Errorf("ListByType(Block) should be empty, got %d", len(blocks))
	}
}

func TestRegistryRegisterAfterUnregister(t *testing.T) {
	reg := NewDeviceRegistry()
	dev1 := &DeviceBase{Name: "disk0", Major: 3, Minor: 0}
	reg.Register(dev1)
	reg.Unregister("disk0")
	dev2 := &DeviceBase{Name: "disk0", Major: 3, Minor: 0}
	if err := reg.Register(dev2); err != nil {
		t.Fatalf("Re-registering after unregister should succeed: %v", err)
	}
	found, ok := reg.Lookup("disk0")
	if !ok || found != dev2 {
		t.Error("Should find the new device after re-register")
	}
}

func TestRegistryMultipleDevicesSameType(t *testing.T) {
	reg := NewDeviceRegistry()
	reg.Register(&DeviceBase{Name: "disk0", Type: DeviceBlock, Major: 3, Minor: 0})
	reg.Register(&DeviceBase{Name: "disk1", Type: DeviceBlock, Major: 3, Minor: 1})
	blocks := reg.ListByType(DeviceBlock)
	if len(blocks) != 2 {
		t.Errorf("ListByType(Block) returned %d, want 2", len(blocks))
	}
}
