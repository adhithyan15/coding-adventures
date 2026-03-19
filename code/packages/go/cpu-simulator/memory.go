// Package cpusimulator provides the core framework for a CPU simulation.
//
// === What is memory? ===
//
// Memory (RAM) is a large array of bytes that the CPU can read from and write to.
// Unlike registers (which are tiny and fast), memory holds megabytes or gigabytes
// of data, but accessing it takes many clock cycles.
//
// Every byte in memory has an "address". To read a byte, you tell the
// memory controller "give me the byte at address 42."
package cpusimulator

import "fmt"

// Memory simulates byte-addressable RAM.
type Memory struct {
	data []byte
	Size int
}

// NewMemory creates a memory of `size` bytes, all initialized to 0.
func NewMemory(size int) *Memory {
	if size < 1 {
		panic("Memory size must be at least 1 byte")
	}
	return &Memory{
		data: make([]byte, size),
		Size: size,
	}
}

// checkAddress verifies an address is within bounds.
func (m *Memory) checkAddress(address, numBytes int) {
	if address < 0 || address+numBytes > m.Size {
		panic(fmt.Sprintf("Memory access out of bounds: address %d, size %d, memory size %d", address, numBytes, m.Size))
	}
}

// ReadByte reads a single byte from memory.
func (m *Memory) ReadByte(address int) byte {
	m.checkAddress(address, 1)
	return m.data[address]
}

// WriteByte writes a single byte to memory.
func (m *Memory) WriteByte(address int, value byte) {
	m.checkAddress(address, 1)
	m.data[address] = value
}

// ReadWord reads a 32-bit word (4 bytes) from memory, little-endian.
func (m *Memory) ReadWord(address int) uint32 {
	m.checkAddress(address, 4)
	return uint32(m.data[address]) |
		(uint32(m.data[address+1]) << 8) |
		(uint32(m.data[address+2]) << 16) |
		(uint32(m.data[address+3]) << 24)
}

// WriteWord writes a 32-bit word to memory, little-endian.
func (m *Memory) WriteWord(address int, value uint32) {
	m.checkAddress(address, 4)
	m.data[address] = byte(value & 0xFF)
	m.data[address+1] = byte((value >> 8) & 0xFF)
	m.data[address+2] = byte((value >> 16) & 0xFF)
	m.data[address+3] = byte((value >> 24) & 0xFF)
}

// LoadBytes copies a sequence of bytes into memory starting at `address`.
func (m *Memory) LoadBytes(address int, data []byte) {
	m.checkAddress(address, len(data))
	copy(m.data[address:], data)
}

// Dump returns a slice of memory (copied to prevent modification).
func (m *Memory) Dump(start, length int) []byte {
	m.checkAddress(start, length)
	result := make([]byte, length)
	copy(result, m.data[start:start+length])
	return result
}
