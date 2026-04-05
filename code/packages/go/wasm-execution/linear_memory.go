// linear_memory.go --- WASM linear memory implementation.
//
// ════════════════════════════════════════════════════════════════════════
// WHAT IS LINEAR MEMORY?
// ════════════════════════════════════════════════════════════════════════
//
// WebAssembly's memory model is a contiguous, byte-addressable array
// called "linear memory".  It is measured in 64 KiB pages.  Memory
// accesses are bounds-checked: reading or writing past the end traps.
//
// WASM always uses little-endian byte order.  Go's encoding/binary
// package provides LittleEndian helpers for reading/writing multi-byte
// values, which we use via binary.LittleEndian.
//
// ════════════════════════════════════════════════════════════════════════
// LOAD VARIANTS: SIGN AND ZERO EXTENSION
// ════════════════════════════════════════════════════════════════════════
//
//	LoadI32_8s  = load 8 bits, sign-extend to i32
//	LoadI32_8u  = load 8 bits, zero-extend to i32
//	LoadI32_16s = load 16 bits, sign-extend to i32
//	LoadI32_16u = load 16 bits, zero-extend to i32
package wasmexecution

import (
	"encoding/binary"
	"fmt"
	"math"
)

// PageSize is the size of a WASM memory page: 64 KiB.
const PageSize = 65536

// LinearMemory is a WASM linear memory: a resizable byte array.
type LinearMemory struct {
	data         []byte
	currentPages int
	maxPages     int  // -1 means no limit (other than spec max of 65536)
}

// NewLinearMemory creates a linear memory with the given initial and
// optional maximum page count.  Pass maxPages < 0 for no limit.
func NewLinearMemory(initialPages int, maxPages int) *LinearMemory {
	return &LinearMemory{
		data:         make([]byte, initialPages*PageSize),
		currentPages: initialPages,
		maxPages:     maxPages,
	}
}

// ════════════════════════════════════════════════════════════════════════
// BOUNDS CHECKING
// ════════════════════════════════════════════════════════════════════════

func (m *LinearMemory) boundsCheck(offset, width int) {
	if offset < 0 || offset+width > len(m.data) {
		panic(NewTrapError(fmt.Sprintf(
			"out of bounds memory access: offset=%d, size=%d, memory size=%d",
			offset, width, len(m.data))))
	}
}

// ════════════════════════════════════════════════════════════════════════
// FULL-WIDTH LOADS
// ═══════════════════════════════════════════════════════��════════════════

// LoadI32 loads 4 bytes as a signed 32-bit integer (little-endian).
func (m *LinearMemory) LoadI32(offset int) int32 {
	m.boundsCheck(offset, 4)
	return int32(binary.LittleEndian.Uint32(m.data[offset:]))
}

// LoadI64 loads 8 bytes as a signed 64-bit integer (little-endian).
func (m *LinearMemory) LoadI64(offset int) int64 {
	m.boundsCheck(offset, 8)
	return int64(binary.LittleEndian.Uint64(m.data[offset:]))
}

// LoadF32 loads 4 bytes as a 32-bit float (little-endian IEEE 754).
func (m *LinearMemory) LoadF32(offset int) float32 {
	m.boundsCheck(offset, 4)
	return math.Float32frombits(binary.LittleEndian.Uint32(m.data[offset:]))
}

// LoadF64 loads 8 bytes as a 64-bit float (little-endian IEEE 754).
func (m *LinearMemory) LoadF64(offset int) float64 {
	m.boundsCheck(offset, 8)
	return math.Float64frombits(binary.LittleEndian.Uint64(m.data[offset:]))
}

// ════════════════════════════════════════════════════════════════════════
// NARROW LOADS FOR I32 (8-bit and 16-bit)
// ════════════════════════════════════════════════════════════════════════

// LoadI32_8s loads 1 byte, sign-extends to int32.
func (m *LinearMemory) LoadI32_8s(offset int) int32 {
	m.boundsCheck(offset, 1)
	return int32(int8(m.data[offset]))
}

// LoadI32_8u loads 1 byte, zero-extends to int32.
func (m *LinearMemory) LoadI32_8u(offset int) int32 {
	m.boundsCheck(offset, 1)
	return int32(m.data[offset])
}

// LoadI32_16s loads 2 bytes (LE), sign-extends to int32.
func (m *LinearMemory) LoadI32_16s(offset int) int32 {
	m.boundsCheck(offset, 2)
	return int32(int16(binary.LittleEndian.Uint16(m.data[offset:])))
}

// LoadI32_16u loads 2 bytes (LE), zero-extends to int32.
func (m *LinearMemory) LoadI32_16u(offset int) int32 {
	m.boundsCheck(offset, 2)
	return int32(binary.LittleEndian.Uint16(m.data[offset:]))
}

// ════════════════════════════════════════════════════════════════════════
// NARROW LOADS FOR I64 (8-bit, 16-bit, 32-bit)
// ════════════════════════════════════════════════════════════════════════

func (m *LinearMemory) LoadI64_8s(offset int) int64 {
	m.boundsCheck(offset, 1)
	return int64(int8(m.data[offset]))
}

func (m *LinearMemory) LoadI64_8u(offset int) int64 {
	m.boundsCheck(offset, 1)
	return int64(m.data[offset])
}

func (m *LinearMemory) LoadI64_16s(offset int) int64 {
	m.boundsCheck(offset, 2)
	return int64(int16(binary.LittleEndian.Uint16(m.data[offset:])))
}

func (m *LinearMemory) LoadI64_16u(offset int) int64 {
	m.boundsCheck(offset, 2)
	return int64(binary.LittleEndian.Uint16(m.data[offset:]))
}

func (m *LinearMemory) LoadI64_32s(offset int) int64 {
	m.boundsCheck(offset, 4)
	return int64(int32(binary.LittleEndian.Uint32(m.data[offset:])))
}

func (m *LinearMemory) LoadI64_32u(offset int) int64 {
	m.boundsCheck(offset, 4)
	return int64(binary.LittleEndian.Uint32(m.data[offset:]))
}

// ════════════════════════════════════════════════════════════════════════
// FULL-WIDTH STORES
// ════════════════════════════════════════════════════════════════════════

func (m *LinearMemory) StoreI32(offset int, value int32) {
	m.boundsCheck(offset, 4)
	binary.LittleEndian.PutUint32(m.data[offset:], uint32(value))
}

func (m *LinearMemory) StoreI64(offset int, value int64) {
	m.boundsCheck(offset, 8)
	binary.LittleEndian.PutUint64(m.data[offset:], uint64(value))
}

func (m *LinearMemory) StoreF32(offset int, value float32) {
	m.boundsCheck(offset, 4)
	binary.LittleEndian.PutUint32(m.data[offset:], math.Float32bits(value))
}

func (m *LinearMemory) StoreF64(offset int, value float64) {
	m.boundsCheck(offset, 8)
	binary.LittleEndian.PutUint64(m.data[offset:], math.Float64bits(value))
}

// ════════════════════════════════════════════════════════════════════════
// NARROW STORES (truncate to smaller width)
// ════════════════════════════════════════════════════════════════════════

func (m *LinearMemory) StoreI32_8(offset int, value int32) {
	m.boundsCheck(offset, 1)
	m.data[offset] = byte(value)
}

func (m *LinearMemory) StoreI32_16(offset int, value int32) {
	m.boundsCheck(offset, 2)
	binary.LittleEndian.PutUint16(m.data[offset:], uint16(value))
}

func (m *LinearMemory) StoreI64_8(offset int, value int64) {
	m.boundsCheck(offset, 1)
	m.data[offset] = byte(value)
}

func (m *LinearMemory) StoreI64_16(offset int, value int64) {
	m.boundsCheck(offset, 2)
	binary.LittleEndian.PutUint16(m.data[offset:], uint16(value))
}

func (m *LinearMemory) StoreI64_32(offset int, value int64) {
	m.boundsCheck(offset, 4)
	binary.LittleEndian.PutUint32(m.data[offset:], uint32(value))
}

// ════════════════════════════════════════════════════════════════════════
// MEMORY GROWTH
// ════════════════════════════════════════════════════════════════════════

// Grow adds deltaPages pages.  Returns the old page count on success,
// or -1 if growth would exceed the maximum.
func (m *LinearMemory) Grow(deltaPages int) int {
	oldPages := m.currentPages
	newPages := oldPages + deltaPages

	if m.maxPages >= 0 && newPages > m.maxPages {
		return -1
	}
	if newPages > 65536 {
		return -1
	}

	newData := make([]byte, newPages*PageSize)
	copy(newData, m.data)
	m.data = newData
	m.currentPages = newPages
	return oldPages
}

// ════════════════════════════════════════════════════════════════════════
// SIZE QUERIES
// ════════════════════════════════════════════════════════════════════════

// Size returns the current memory size in pages.
func (m *LinearMemory) Size() int {
	return m.currentPages
}

// ByteLength returns the current memory size in bytes.
func (m *LinearMemory) ByteLength() int {
	return len(m.data)
}

// ════════════════════════════════════════════════════════════���═══════════
// RAW BYTE ACCESS
// ════════════════════════════════════════════════════════════════════════

// WriteBytes copies raw bytes into memory at the given offset.
// Used during module instantiation for data segments.
func (m *LinearMemory) WriteBytes(offset int, data []byte) {
	m.boundsCheck(offset, len(data))
	copy(m.data[offset:], data)
}
