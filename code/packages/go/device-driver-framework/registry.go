package devicedriverframework

import "fmt"

// =========================================================================
// DeviceRegistry -- the kernel's phonebook for devices
// =========================================================================
//
// When the system boots, drivers create device objects and register them here.
// Later, when the kernel needs to perform I/O, it looks up the target device
// in the registry.
//
// The registry supports two lookup strategies:
//   1. By name: "disk0" -> SimulatedDisk instance (human-friendly)
//   2. By (major, minor): (3, 0) -> SimulatedDisk (machine-friendly)
//
// Both are O(1) lookups using Go maps.

// majorMinorKey creates a map key from major and minor numbers.
// We encode the pair as a single string to use as a map key, since Go
// maps require comparable key types and tuples aren't built-in.
type majorMinorKey struct {
	Major int
	Minor int
}

// DeviceRegistry is the central registry for all devices in the system.
type DeviceRegistry struct {
	byName       map[string]Device
	byMajorMinor map[majorMinorKey]Device
}

// NewDeviceRegistry creates an empty device registry.
func NewDeviceRegistry() *DeviceRegistry {
	return &DeviceRegistry{
		byName:       make(map[string]Device),
		byMajorMinor: make(map[majorMinorKey]Device),
	}
}

// Register adds a device to the registry.
//
// The device must have a unique name and a unique (major, minor) pair.
// Returns an error if a device with the same name or same (major, minor)
// is already registered.
func (r *DeviceRegistry) Register(dev Device) error {
	base := dev.GetBase()
	if _, exists := r.byName[base.Name]; exists {
		return fmt.Errorf("device with name %q is already registered", base.Name)
	}
	key := majorMinorKey{base.Major, base.Minor}
	if existing, exists := r.byMajorMinor[key]; exists {
		return fmt.Errorf("device with major=%d, minor=%d is already registered as %q",
			base.Major, base.Minor, existing.GetBase().Name)
	}
	r.byName[base.Name] = dev
	r.byMajorMinor[key] = dev
	return nil
}

// Unregister removes a device from the registry by name.
// Returns the removed device and true, or nil and false if not found.
func (r *DeviceRegistry) Unregister(name string) (Device, bool) {
	dev, exists := r.byName[name]
	if !exists {
		return nil, false
	}
	base := dev.GetBase()
	delete(r.byName, name)
	delete(r.byMajorMinor, majorMinorKey{base.Major, base.Minor})
	return dev, true
}

// Lookup finds a device by its human-readable name.
// Returns the device and true, or nil and false if not found.
func (r *DeviceRegistry) Lookup(name string) (Device, bool) {
	dev, exists := r.byName[name]
	return dev, exists
}

// LookupByMajorMinor finds a device by its (major, minor) number pair.
// Returns the device and true, or nil and false if not found.
func (r *DeviceRegistry) LookupByMajorMinor(major, minor int) (Device, bool) {
	dev, exists := r.byMajorMinor[majorMinorKey{major, minor}]
	return dev, exists
}

// ListDevices returns all registered devices.
func (r *DeviceRegistry) ListDevices() []Device {
	devices := make([]Device, 0, len(r.byName))
	for _, dev := range r.byName {
		devices = append(devices, dev)
	}
	return devices
}

// ListByType returns all devices of a specific type.
func (r *DeviceRegistry) ListByType(deviceType DeviceType) []Device {
	var devices []Device
	for _, dev := range r.byName {
		if dev.GetBase().Type == deviceType {
			devices = append(devices, dev)
		}
	}
	return devices
}
