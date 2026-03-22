package devicedriverframework

import "testing"

// =========================================================================
// DeviceType Tests
// =========================================================================

func TestDeviceTypeValues(t *testing.T) {
	// Verify that each device type has the expected integer value.
	if DeviceCharacter != 0 {
		t.Errorf("DeviceCharacter = %d, want 0", DeviceCharacter)
	}
	if DeviceBlock != 1 {
		t.Errorf("DeviceBlock = %d, want 1", DeviceBlock)
	}
	if DeviceNetwork != 2 {
		t.Errorf("DeviceNetwork = %d, want 2", DeviceNetwork)
	}
}

func TestDeviceTypeString(t *testing.T) {
	if s := DeviceCharacter.String(); s != "CHARACTER" {
		t.Errorf("DeviceCharacter.String() = %q, want CHARACTER", s)
	}
	if s := DeviceBlock.String(); s != "BLOCK" {
		t.Errorf("DeviceBlock.String() = %q, want BLOCK", s)
	}
	if s := DeviceNetwork.String(); s != "NETWORK" {
		t.Errorf("DeviceNetwork.String() = %q, want NETWORK", s)
	}
	// Unknown type
	unknown := DeviceType(99)
	s := unknown.String()
	if s != "UNKNOWN(99)" {
		t.Errorf("Unknown DeviceType.String() = %q, want UNKNOWN(99)", s)
	}
}

// =========================================================================
// DeviceBase Tests
// =========================================================================

func TestDeviceBaseFields(t *testing.T) {
	dev := DeviceBase{
		Name:            "test0",
		Type:            DeviceCharacter,
		Major:           1,
		Minor:           0,
		InterruptNumber: 33,
	}
	if dev.Name != "test0" {
		t.Errorf("Name = %q, want test0", dev.Name)
	}
	if dev.Type != DeviceCharacter {
		t.Errorf("Type = %v, want CHARACTER", dev.Type)
	}
	if dev.Major != 1 {
		t.Errorf("Major = %d, want 1", dev.Major)
	}
	if dev.Minor != 0 {
		t.Errorf("Minor = %d, want 0", dev.Minor)
	}
	if dev.InterruptNumber != 33 {
		t.Errorf("InterruptNumber = %d, want 33", dev.InterruptNumber)
	}
	if dev.Initialized {
		t.Error("Initialized should be false before Init()")
	}
}

func TestDeviceBaseInit(t *testing.T) {
	dev := &DeviceBase{Name: "test0", Type: DeviceBlock, Major: 3, Minor: 0}
	dev.Init()
	if !dev.Initialized {
		t.Error("Initialized should be true after Init()")
	}
}

func TestDeviceBaseGetBase(t *testing.T) {
	dev := &DeviceBase{Name: "test0"}
	base := dev.GetBase()
	if base != dev {
		t.Error("GetBase() should return a pointer to itself")
	}
}

func TestDeviceBaseString(t *testing.T) {
	dev := &DeviceBase{Name: "disk0", Type: DeviceBlock, Major: 3, Minor: 0, InterruptNumber: 34}
	s := dev.String()
	if s == "" {
		t.Error("String() should not return empty string")
	}
}

// =========================================================================
// Interface Compliance Tests
// =========================================================================

// Compile-time interface satisfaction checks.
var _ Device = (*DeviceBase)(nil)
var _ Device = (*SimulatedDisk)(nil)
var _ Device = (*SimulatedKeyboard)(nil)
var _ Device = (*SimulatedDisplay)(nil)
var _ Device = (*SimulatedNIC)(nil)

var _ CharacterDevice = (*SimulatedKeyboard)(nil)
var _ CharacterDevice = (*SimulatedDisplay)(nil)
var _ BlockDevice = (*SimulatedDisk)(nil)
var _ NetworkDevice = (*SimulatedNIC)(nil)
