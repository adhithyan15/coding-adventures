package rombios

// === ROM: Read-Only Memory ===
//
// Real computers have a ROM chip soldered to the motherboard containing
// firmware that executes on power-on. The CPU's program counter starts at
// the ROM's base address (typically 0xFFFF0000 for our simulated machine).
//
// ROM has a distinctive property: writes are silently ignored. The data
// was "burned in" when the ROM was manufactured (or in our case, when the
// ROM object is created). This is like a recipe card sealed in plastic --
// you can read it any number of times, but you cannot modify it.
//
//     Memory map showing ROM's position:
//
//     0xFFFF_FFFF ┌──────────────────┐
//                 │    ROM (64 KB)   │ ← CPU starts executing here
//     0xFFFF_0000 ├──────────────────┤
//                 │   Framebuffer    │
//     0xFFFB_0000 ├──────────────────┤
//                 │       ...        │
//     0x0001_0000 ├──────────────────┤
//                 │   Bootloader     │ ← BIOS jumps here after init
//     0x0000_1000 ├──────────────────┤
//                 │   HardwareInfo   │ ← BIOS writes hardware config here
//     0x0000_0000 ├──────────────────┤
//                 │       IDT        │ ← Interrupt Descriptor Table
//                 └──────────────────┘

// DefaultROMBase is the standard base address for ROM: the top of the
// 32-bit address space minus 64 KB. This is where the CPU's program
// counter points on power-on.
const DefaultROMBase uint32 = 0xFFFF0000

// DefaultROMSize is the standard ROM size: 64 KB (65536 bytes).
// This is enough space for the BIOS firmware.
const DefaultROMSize = 65536

// ROMConfig defines the read-only memory region.
//
// The default configuration places ROM at the top of the 32-bit address
// space (0xFFFF0000), which is the conventional reset vector. The CPU's
// program counter is set to this address on power-on, so whatever code
// lives here executes first.
type ROMConfig struct {
	BaseAddress uint32 // Start address (default: 0xFFFF0000)
	Size        int    // Size in bytes (default: 65536 = 64KB)
}

// DefaultROMConfig returns the standard ROM configuration:
//
//	BaseAddress: 0xFFFF0000, Size: 65536 (64KB).
func DefaultROMConfig() ROMConfig {
	return ROMConfig{
		BaseAddress: DefaultROMBase,
		Size:        DefaultROMSize,
	}
}

// ROM represents a read-only memory region.
//
// Once created with a firmware image, the contents cannot be changed.
// Write operations are silently ignored -- this models the behavior of
// real ROM chips, which are programmed at the factory and cannot be
// rewritten by the CPU.
//
// Example usage:
//
//	bios := NewBIOSFirmware(DefaultBIOSConfig())
//	rom := NewROM(DefaultROMConfig(), bios.Generate())
//
//	// Reading works normally:
//	firstByte := rom.Read(0xFFFF0000)
//	firstWord := rom.ReadWord(0xFFFF0000)
//
//	// Writing is silently ignored:
//	rom.Write(0xFFFF0000, 0xFF)
//	// rom.Read(0xFFFF0000) still returns the original byte
type ROM struct {
	config ROMConfig
	data   []byte
}

// NewROM creates a ROM loaded with the given firmware bytes.
//
// The firmware is copied into internal storage -- the caller's slice
// is not retained. If len(firmware) < config.Size, the remaining
// bytes are zero-filled. If len(firmware) > config.Size, it panics.
//
// Example:
//
//	firmware := []byte{0x37, 0x12, 0x00, 0x00} // lui x4, 0x12
//	rom := NewROM(DefaultROMConfig(), firmware)
func NewROM(config ROMConfig, firmware []byte) *ROM {
	if len(firmware) > config.Size {
		panic("firmware larger than ROM size")
	}

	// Copy firmware into a fixed-size buffer (zero-filled beyond firmware)
	data := make([]byte, config.Size)
	copy(data, firmware)

	return &ROM{
		config: config,
		data:   data,
	}
}

// Read returns a single byte from the given absolute address.
//
// The address must fall within [BaseAddress, BaseAddress+Size).
// Out-of-range addresses return 0, modeling the behavior of reading
// from unmapped memory.
func (r *ROM) Read(address uint32) byte {
	offset := r.addressToOffset(address)
	if offset < 0 {
		return 0
	}
	return r.data[offset]
}

// ReadWord returns a 32-bit little-endian word starting at the given
// absolute address.
//
// This is the primary access pattern since RISC-V instructions are
// 32 bits wide. The CPU fetches one word at a time from ROM.
//
// Little-endian means the least significant byte is at the lowest address:
//
//	Address+0: byte 0 (bits 7:0)
//	Address+1: byte 1 (bits 15:8)
//	Address+2: byte 2 (bits 23:16)
//	Address+3: byte 3 (bits 31:24)
func (r *ROM) ReadWord(address uint32) uint32 {
	offset := r.addressToOffset(address)
	if offset < 0 || offset+3 >= len(r.data) {
		return 0
	}
	return readLE32(r.data[offset:])
}

// Write attempts to write a byte to ROM. Since ROM is read-only,
// this operation is silently ignored.
//
// In a real system, attempting to write to ROM has no effect. The
// data lines simply do not connect to write circuitry. We model this
// by accepting the call but doing nothing.
func (r *ROM) Write(address uint32, value byte) {
	// Silently ignored -- ROM is read-only.
	_ = address
	_ = value
}

// Size returns the total size of the ROM in bytes.
func (r *ROM) Size() int {
	return r.config.Size
}

// BaseAddress returns the base address of the ROM.
func (r *ROM) BaseAddress() uint32 {
	return r.config.BaseAddress
}

// Contains returns true if the given address falls within the ROM region.
func (r *ROM) Contains(address uint32) bool {
	return r.addressToOffset(address) >= 0
}

// addressToOffset converts an absolute address to a byte offset within
// the ROM's data buffer. Returns -1 if the address is out of range.
func (r *ROM) addressToOffset(address uint32) int {
	if address < r.config.BaseAddress {
		return -1
	}
	offset := int(address - r.config.BaseAddress)
	if offset >= r.config.Size {
		return -1
	}
	return offset
}
