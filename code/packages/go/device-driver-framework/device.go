// Package devicedriverframework provides a unified device driver abstraction
// for the coding-adventures simulated computer.
//
// Every operating system faces the same fundamental challenge: the kernel needs
// to communicate with dozens of different hardware devices (keyboards, disks,
// network cards, displays), but each device speaks a different protocol with
// different timing, register layouts, and data formats.
//
// Device drivers solve this by inserting a translation layer between the kernel
// and the hardware. The kernel speaks a small set of well-defined protocols
// (read bytes, write blocks, send packets), and each driver translates those
// generic operations into the specific commands its hardware understands.
//
// Analogy: Device drivers are like a universal remote control. You press
// "Volume Up" and it works on any TV brand. Each TV speaks a different infrared
// protocol, but the remote translates your single button press into the right
// signal for each brand.
package devicedriverframework

import "fmt"

// =========================================================================
// DeviceType
// =========================================================================
// We classify devices into three families based on how they naturally
// operate. Each family has a different interface reflecting the hardware's
// natural data model.

// DeviceType classifies a device into one of three families.
type DeviceType int

const (
	// DeviceCharacter represents byte-stream devices like keyboards and
	// serial ports. Data arrives one byte at a time, in order, with no
	// random access. You cannot "seek" to byte 47 of a keyboard.
	DeviceCharacter DeviceType = iota

	// DeviceBlock represents fixed-size block devices like disks and SSDs.
	// Data is accessed in fixed-size chunks (typically 512 bytes) at
	// arbitrary positions. You can read block 0, then block 9999, then
	// block 42 -- in any order.
	DeviceBlock

	// DeviceNetwork represents packet-oriented devices like Ethernet NICs.
	// Data comes in discrete packets (variable-length messages with headers
	// and payloads) rather than streams or blocks.
	DeviceNetwork
)

// String returns a human-readable name for the device type.
func (dt DeviceType) String() string {
	result, _ := StartNew[string]("device-driver-framework.DeviceType.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			switch dt {
			case DeviceCharacter:
				return rf.Generate(true, false, "CHARACTER")
			case DeviceBlock:
				return rf.Generate(true, false, "BLOCK")
			case DeviceNetwork:
				return rf.Generate(true, false, "NETWORK")
			default:
				return rf.Generate(true, false, fmt.Sprintf("UNKNOWN(%d)", int(dt)))
			}
		}).GetResult()
	return result
}

// =========================================================================
// DeviceBase -- common fields for all devices
// =========================================================================
// Every device in the system -- regardless of whether it's a keyboard, disk,
// or network card -- shares these core attributes. This is the "common
// denominator" that lets the DeviceRegistry store and manage all devices
// uniformly.

// DeviceBase holds the common fields shared by all device types.
//
// Fields:
//   - Name: Human-readable identifier, e.g. "disk0", "keyboard0".
//   - Type: Which family this device belongs to (CHARACTER, BLOCK, NETWORK).
//   - Major: Driver identifier. All devices handled by the same driver share
//     a major number.
//   - Minor: Instance identifier within the driver. First disk = 0, second = 1.
//   - InterruptNumber: Which interrupt this device raises when it needs
//     attention. -1 if the device does not use interrupts.
//   - Initialized: Whether Init() has been called.
type DeviceBase struct {
	Name            string
	Type            DeviceType
	Major           int
	Minor           int
	InterruptNumber int
	Initialized     bool
}

// Init sets the Initialized flag to true. Concrete device types override
// this to perform hardware-specific setup.
func (d *DeviceBase) Init() {
	_, _ = StartNew[struct{}]("device-driver-framework.DeviceBase.Init", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			d.Initialized = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GetBase returns the DeviceBase, satisfying the Device interface.
// This lets the registry access common fields without type assertions.
func (d *DeviceBase) GetBase() *DeviceBase {
	result, _ := StartNew[*DeviceBase]("device-driver-framework.DeviceBase.GetBase", nil,
		func(op *Operation[*DeviceBase], rf *ResultFactory[*DeviceBase]) *OperationResult[*DeviceBase] {
			return rf.Generate(true, false, d)
		}).GetResult()
	return result
}

// String returns a human-readable representation of the device.
func (d *DeviceBase) String() string {
	result, _ := StartNew[string]("device-driver-framework.DeviceBase.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, fmt.Sprintf("%s(name=%q, type=%s, major=%d, minor=%d, irq=%d)",
				"Device", d.Name, d.Type, d.Major, d.Minor, d.InterruptNumber))
		}).GetResult()
	return result
}

// =========================================================================
// Device Interface
// =========================================================================
// All devices implement this interface so the registry can store them
// uniformly. Each device family (Character, Block, Network) extends this
// with family-specific methods.

// Device is the base interface that all devices implement.
// It provides access to the common DeviceBase fields.
type Device interface {
	GetBase() *DeviceBase
	Init()
}

// =========================================================================
// CharacterDevice Interface
// =========================================================================
// Character devices produce or consume a stream of bytes, one at a time.
// You cannot "seek" to a specific position -- data arrives when it arrives
// (like a keyboard) or is consumed in order (like a display).
//
// Real-world examples:
//   - /dev/ttyS0 -- serial port
//   - /dev/stdin -- keyboard input
//   - /dev/null  -- discards everything written to it

// CharacterDevice is a byte-stream device (keyboard, serial port, display).
type CharacterDevice interface {
	Device

	// Read reads up to len(buf) bytes from the device.
	// Returns the number of bytes actually read. Returns 0 if no data
	// is available (non-blocking).
	Read(buf []byte) int

	// Write writes data to the device.
	// Returns the number of bytes written, or -1 on error.
	Write(data []byte) int
}

// =========================================================================
// BlockDevice Interface
// =========================================================================
// Block devices read and write fixed-size chunks called "blocks" or
// "sectors." The standard block size is 512 bytes -- a legacy from the
// IBM PC/AT (1984). Block devices support RANDOM ACCESS: you can read
// any block in any order.
//
// Real-world examples:
//   - /dev/sda   -- first SCSI/SATA disk
//   - /dev/nvme0 -- first NVMe SSD

// BlockDevice is a fixed-size block device (disk, SSD, USB drive).
type BlockDevice interface {
	Device

	// ReadBlock reads exactly BlockSize() bytes from the given block number.
	// Returns the data and nil error, or nil data and an error if the
	// block number is out of range.
	ReadBlock(blockNum int) ([]byte, error)

	// WriteBlock writes exactly BlockSize() bytes to the given block number.
	// Returns an error if blockNum is out of range or data is the wrong size.
	WriteBlock(blockNum int, data []byte) error

	// BlockSize returns the number of bytes per block.
	BlockSize() int

	// TotalBlocks returns the total number of blocks on this device.
	TotalBlocks() int
}

// =========================================================================
// NetworkDevice Interface
// =========================================================================
// Network devices deal in packets -- discrete messages with headers,
// addresses, and payloads. Every NIC has a MAC address (a 6-byte unique
// identifier assigned at the factory).
//
// Real-world examples:
//   - eth0  -- first Ethernet interface
//   - wlan0 -- first WiFi interface

// NetworkDevice is a packet-oriented network device (Ethernet NIC, WiFi).
type NetworkDevice interface {
	Device

	// SendPacket sends a packet over the network.
	// Returns the number of bytes sent, or -1 on error.
	SendPacket(data []byte) int

	// ReceivePacket receives the next packet from the network.
	// Returns the packet data, or nil if no packet is available.
	ReceivePacket() []byte

	// HasPacket returns true if there is a packet waiting.
	HasPacket() bool

	// MACAddress returns the 6-byte MAC address of this NIC.
	MACAddress() []byte
}

// =========================================================================
// Well-known constants
// =========================================================================

const (
	// Interrupt numbers for devices (matching the spec).
	IntTimer    = 32
	IntKeyboard = 33
	IntDisk     = 34
	IntNIC      = 35

	// Major numbers for our simulated devices.
	MajorDisplay  = 1
	MajorKeyboard = 2
	MajorDisk     = 3
	MajorNIC      = 4
)
