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
	result, _ := StartNew[*Memory]("cpu-simulator.NewMemory", nil,
		func(op *Operation[*Memory], rf *ResultFactory[*Memory]) *OperationResult[*Memory] {
			op.AddProperty("size", size)
			if size < 1 {
				panic("Memory size must be at least 1 byte")
			}
			return rf.Generate(true, false, &Memory{
				data: make([]byte, size),
				Size: size,
			})
		}).GetResult()
	return result
}

// checkAddress verifies an address is within bounds.
func (m *Memory) checkAddress(address, numBytes int) {
	if address < 0 || address+numBytes > m.Size {
		panic(fmt.Sprintf("Memory access out of bounds: address %d, size %d, memory size %d", address, numBytes, m.Size))
	}
}

// ReadByte reads a single byte from memory.
func (m *Memory) ReadByte(address int) byte {
	result, _ := StartNew[byte]("cpu-simulator.Memory.ReadByte", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			op.AddProperty("address", address)
			m.checkAddress(address, 1)
			return rf.Generate(true, false, m.data[address])
		}).GetResult()
	return result
}

// WriteByte writes a single byte to memory.
func (m *Memory) WriteByte(address int, value byte) {
	_, _ = StartNew[struct{}]("cpu-simulator.Memory.WriteByte", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			m.checkAddress(address, 1)
			m.data[address] = value
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ReadWord reads a 32-bit word (4 bytes) from memory, little-endian.
func (m *Memory) ReadWord(address int) uint32 {
	result, _ := StartNew[uint32]("cpu-simulator.Memory.ReadWord", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			op.AddProperty("address", address)
			m.checkAddress(address, 4)
			return rf.Generate(true, false, uint32(m.data[address])|
				(uint32(m.data[address+1])<<8)|
				(uint32(m.data[address+2])<<16)|
				(uint32(m.data[address+3])<<24))
		}).GetResult()
	return result
}

// WriteWord writes a 32-bit word to memory, little-endian.
func (m *Memory) WriteWord(address int, value uint32) {
	_, _ = StartNew[struct{}]("cpu-simulator.Memory.WriteWord", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			m.checkAddress(address, 4)
			m.data[address] = byte(value & 0xFF)
			m.data[address+1] = byte((value >> 8) & 0xFF)
			m.data[address+2] = byte((value >> 16) & 0xFF)
			m.data[address+3] = byte((value >> 24) & 0xFF)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// LoadBytes copies a sequence of bytes into memory starting at `address`.
func (m *Memory) LoadBytes(address int, data []byte) {
	_, _ = StartNew[struct{}]("cpu-simulator.Memory.LoadBytes", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			op.AddProperty("data_size", len(data))
			m.checkAddress(address, len(data))
			copy(m.data[address:], data)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Dump returns a slice of memory (copied to prevent modification).
func (m *Memory) Dump(start, length int) []byte {
	result, _ := StartNew[[]byte]("cpu-simulator.Memory.Dump", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("start", start)
			op.AddProperty("length", length)
			m.checkAddress(start, length)
			data := make([]byte, length)
			copy(data, m.data[start:start+length])
			return rf.Generate(true, false, data)
		}).GetResult()
	return result
}
