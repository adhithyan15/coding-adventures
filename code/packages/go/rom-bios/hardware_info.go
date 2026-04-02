// Package rombios implements ROM (read-only memory) and BIOS firmware for
// the simulated computer's power-on initialization sequence.
//
// === What is HardwareInfo? ===
//
// When the BIOS finishes initializing hardware, it leaves a "status report"
// at a well-known memory address (0x00001000). This report is the HardwareInfo
// struct -- a block of 7 little-endian uint32 fields that tell the bootloader
// and kernel everything they need to know about the hardware configuration.
//
// Think of it like a building manager's morning report left on the front desk:
//
//     "Building report:
//      - 64 MB of usable space
//      - Display: 80 columns x 25 rows
//      - Framebuffer at 0xFFFB0000
//      - Interrupt table at 0x00000000 (256 entries)
//      - Bootloader should start at 0x00010000"
//
// Memory layout at 0x00001000 (28 bytes total):
//
//     Offset  Size    Field              Default
//     ------  ------  -----------------  ----------
//     0x00    4       MemorySize         (probed)
//     0x04    4       DisplayColumns     80
//     0x08    4       DisplayRows        25
//     0x0C    4       FramebufferBase    0xFFFB0000
//     0x10    4       IDTBase            0x00000000
//     0x14    4       IDTEntries         256
//     0x18    4       BootloaderEntry    0x00010000
package rombios

// HardwareInfoAddress is the fixed memory address where the BIOS writes
// the HardwareInfo struct. The bootloader reads from this address to
// discover hardware configuration.
const HardwareInfoAddress uint32 = 0x00001000

// HardwareInfoSize is the size of the HardwareInfo struct in bytes.
// 7 fields * 4 bytes each = 28 bytes.
const HardwareInfoSize = 28

// HardwareInfo describes the hardware configuration discovered and
// configured by the BIOS during initialization.
//
// This struct is written to memory at HardwareInfoAddress (0x00001000)
// by the BIOS firmware. The bootloader and kernel read it to learn
// about the system without needing to re-probe hardware.
//
// All fields are stored as little-endian uint32 values in memory.
type HardwareInfo struct {
	// MemorySize is the total amount of usable RAM in bytes, as
	// discovered by the BIOS memory probe. For example, 64*1024*1024
	// means 64 MB of RAM.
	MemorySize uint32

	// DisplayColumns is the number of text columns in the display.
	// Standard value: 80 (matching the classic 80-column terminal).
	DisplayColumns uint32

	// DisplayRows is the number of text rows in the display.
	// Standard value: 25 (matching the classic 25-row terminal).
	DisplayRows uint32

	// FramebufferBase is the starting address of the display's
	// framebuffer in memory. Characters written here appear on screen.
	// Default: 0xFFFB0000.
	FramebufferBase uint32

	// IDTBase is the starting address of the Interrupt Descriptor Table.
	// Default: 0x00000000 (the very beginning of memory).
	IDTBase uint32

	// IDTEntries is the number of entries in the IDT.
	// Default: 256 (matching x86 convention: 0-255).
	IDTEntries uint32

	// BootloaderEntry is the address where the bootloader code begins.
	// After BIOS finishes, it jumps to this address.
	// Default: 0x00010000.
	BootloaderEntry uint32
}

// DefaultHardwareInfo returns a HardwareInfo with standard default values.
// MemorySize is set to 0, meaning "not yet probed" -- the BIOS firmware
// will fill it in during the memory probe step.
func DefaultHardwareInfo() HardwareInfo {
	result, _ := StartNew[HardwareInfo]("rom-bios.DefaultHardwareInfo", HardwareInfo{},
		func(op *Operation[HardwareInfo], rf *ResultFactory[HardwareInfo]) *OperationResult[HardwareInfo] {
			return rf.Generate(true, false, HardwareInfo{
				MemorySize:      0,
				DisplayColumns:  80,
				DisplayRows:     25,
				FramebufferBase: 0xFFFB0000,
				IDTBase:         0x00000000,
				IDTEntries:      256,
				BootloaderEntry: 0x00010000,
			})
		}).GetResult()
	return result
}

// ToBytes serializes the HardwareInfo struct to a 28-byte slice in
// little-endian format, matching the memory layout that the BIOS
// firmware writes and the bootloader expects to read.
//
// Example:
//
//     info := DefaultHardwareInfo()
//     info.MemorySize = 64 * 1024 * 1024
//     bytes := info.ToBytes()
//     // bytes[0:4]  = 0x04000000 (64 MB in little-endian)
//     // bytes[4:8]  = 0x50000000 (80 in little-endian)
//     // ...
func (h HardwareInfo) ToBytes() []byte {
	result, _ := StartNew[[]byte]("rom-bios.HardwareInfo.ToBytes", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			buf := make([]byte, HardwareInfoSize)
			putLE32(buf[0:], h.MemorySize)
			putLE32(buf[4:], h.DisplayColumns)
			putLE32(buf[8:], h.DisplayRows)
			putLE32(buf[12:], h.FramebufferBase)
			putLE32(buf[16:], h.IDTBase)
			putLE32(buf[20:], h.IDTEntries)
			putLE32(buf[24:], h.BootloaderEntry)
			return rf.Generate(true, false, buf)
		}).GetResult()
	return result
}

// HardwareInfoFromBytes deserializes a 28-byte little-endian buffer
// into a HardwareInfo struct. This is the inverse of ToBytes().
//
// Panics if len(data) < HardwareInfoSize.
func HardwareInfoFromBytes(data []byte) HardwareInfo {
	result, _ := StartNew[HardwareInfo]("rom-bios.HardwareInfoFromBytes", HardwareInfo{},
		func(op *Operation[HardwareInfo], rf *ResultFactory[HardwareInfo]) *OperationResult[HardwareInfo] {
			if len(data) < HardwareInfoSize {
				panic("HardwareInfoFromBytes: data too short")
			}
			return rf.Generate(true, false, HardwareInfo{
				MemorySize:      readLE32(data[0:]),
				DisplayColumns:  readLE32(data[4:]),
				DisplayRows:     readLE32(data[8:]),
				FramebufferBase: readLE32(data[12:]),
				IDTBase:         readLE32(data[16:]),
				IDTEntries:      readLE32(data[20:]),
				BootloaderEntry: readLE32(data[24:]),
			})
		}).GetResult()
	return result
}

// putLE32 writes a uint32 in little-endian byte order.
func putLE32(buf []byte, val uint32) {
	buf[0] = byte(val)
	buf[1] = byte(val >> 8)
	buf[2] = byte(val >> 16)
	buf[3] = byte(val >> 24)
}

// readLE32 reads a uint32 in little-endian byte order.
func readLE32(buf []byte) uint32 {
	return uint32(buf[0]) | uint32(buf[1])<<8 | uint32(buf[2])<<16 | uint32(buf[3])<<24
}
