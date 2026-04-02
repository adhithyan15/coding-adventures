package gpucore

// LocalMemory -- byte-addressable scratchpad with floating-point load/store.
//
// # What is Local Memory?
//
// Every GPU thread has a small, private memory area called "local memory" or
// "scratchpad." It's used for temporary storage that doesn't fit in registers:
// spilled variables, array elements, intermediate results.
//
//	+----------------------------------------------+
//	|              Local Memory (4 KB)              |
//	+----------------------------------------------+
//	|  0x000: [42] [00] [48] [42]  <- 3.14 as FP32 |
//	|  0x004: [EC] [51] [2D] [40]  <- 2.71 as FP32 |
//	|  0x008: [00] [00] [00] [00]  <- 0.0           |
//	|  ...                                          |
//	|  0xFFC: [00] [00] [00] [00]                   |
//	+----------------------------------------------+
//
// # How Floats Live in Memory
//
// A FloatBits value (sign + exponent + mantissa) must be converted to raw bytes
// before it can be stored in memory. This is the same process that happens in
// real hardware when a GPU core executes a STORE instruction:
//
//  1. Take the FloatBits fields: sign=0, exponent=[01111111], mantissa=[10010...]
//  2. Concatenate into a bit string: 0_01111111_10010001000011111101101
//  3. Group into bytes: [3F] [C9] [0F] [DB] (that's 3.14159 in FP32)
//  4. Write bytes to memory in little-endian order: [DB] [0F] [C9] [3F]
//
// Loading reverses this: read bytes, reassemble bits, create FloatBits.
//
// # Memory Sizes Across Vendors
//
//	NVIDIA: 512 KB local memory per thread (rarely used, slow)
//	AMD:    Scratch memory, up to 4 MB per wavefront
//	ARM:    Stack memory region per thread
//	TPU:    No per-PE memory (data flows through systolic array)
//
// Our default of 4 KB is small but sufficient for educational programs.

import (
	"encoding/binary"
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
)

// LocalMemory is a byte-addressable local scratchpad memory with FP-aware
// load/store.
//
// Provides both raw byte access and convenient floating-point operations
// that handle the conversion between FloatBits and byte sequences.
type LocalMemory struct {
	// Size is the memory size in bytes.
	Size int

	// data holds the raw byte contents.
	data []byte
}

// NewLocalMemory creates a new local memory of the given size in bytes.
//
// All bytes are initialized to zero. Returns an error if size < 1.
func NewLocalMemory(size int) (*LocalMemory, error) {
	return StartNew[*LocalMemory]("gpu-core.NewLocalMemory", nil,
		func(op *Operation[*LocalMemory], rf *ResultFactory[*LocalMemory]) *OperationResult[*LocalMemory] {
			if size < 1 {
				return rf.Fail(nil, fmt.Errorf("memory size must be positive, got %d", size))
			}
			return rf.Generate(true, false, &LocalMemory{
				Size: size,
				data: make([]byte, size),
			})
		}).GetResult()
}

// checkBounds validates that a memory access is within bounds.
func (m *LocalMemory) checkBounds(address, numBytes int) error {
	if address < 0 || address+numBytes > m.Size {
		return fmt.Errorf(
			"memory access at %d:%d out of bounds [0, %d)",
			address, address+numBytes, m.Size,
		)
	}
	return nil
}

// =========================================================================
// Raw byte access
// =========================================================================

// ReadByte reads a single byte from memory.
func (m *LocalMemory) ReadByte(address int) (byte, error) {
	return StartNew[byte]("gpu-core.LocalMemory.ReadByte", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			if err := m.checkBounds(address, 1); err != nil {
				return rf.Fail(0, err)
			}
			return rf.Generate(true, false, m.data[address])
		}).GetResult()
}

// WriteByte writes a single byte to memory.
func (m *LocalMemory) WriteByte(address int, value byte) error {
	_, err := StartNew[struct{}]("gpu-core.LocalMemory.WriteByte", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if err := m.checkBounds(address, 1); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			m.data[address] = value
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// ReadBytes reads multiple bytes from memory.
func (m *LocalMemory) ReadBytes(address, count int) ([]byte, error) {
	return StartNew[[]byte]("gpu-core.LocalMemory.ReadBytes", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			if err := m.checkBounds(address, count); err != nil {
				return rf.Fail(nil, err)
			}
			result := make([]byte, count)
			copy(result, m.data[address:address+count])
			return rf.Generate(true, false, result)
		}).GetResult()
}

// WriteBytes writes multiple bytes to memory.
func (m *LocalMemory) WriteBytes(address int, data []byte) error {
	_, err := StartNew[struct{}]("gpu-core.LocalMemory.WriteBytes", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if err := m.checkBounds(address, len(data)); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			copy(m.data[address:], data)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// =========================================================================
// Floating-point access
// =========================================================================

// floatByteWidth returns how many bytes a float format uses: FP32=4, FP16/BF16=2.
func floatByteWidth(fmt fp.FloatFormat) int {
	return fmt.TotalBits / 8
}

// floatBitsToBytes converts a FloatBits to raw bytes (little-endian).
//
// The process:
//  1. Concatenate sign + exponent + mantissa into one integer
//  2. Pack that integer into bytes using little-endian encoding
//
// Example for FP32 value 1.0:
//
//	sign=0, exponent=[0,1,1,1,1,1,1,1], mantissa=[0]*23
//	-> bit string: 0_01111111_00000000000000000000000
//	-> integer: 0x3F800000
//	-> bytes (little-endian): [00, 00, 80, 3F]
func floatBitsToBytes(value fp.FloatBits) ([]byte, error) {
	// Reassemble the bit pattern from FloatBits fields.
	bits := value.Sign
	for _, b := range value.Exponent {
		bits = (bits << 1) | b
	}
	for _, b := range value.Mantissa {
		bits = (bits << 1) | b
	}

	// Pack as bytes using little-endian encoding.
	byteWidth := floatByteWidth(value.Fmt)
	switch byteWidth {
	case 4:
		buf := make([]byte, 4)
		binary.LittleEndian.PutUint32(buf, uint32(bits))
		return buf, nil
	case 2:
		buf := make([]byte, 2)
		binary.LittleEndian.PutUint16(buf, uint16(bits))
		return buf, nil
	default:
		return nil, fmt.Errorf("unsupported float width: %d bytes", byteWidth)
	}
}

// bytesToFloatBits converts raw bytes (little-endian) back to a FloatBits.
//
// Reverses floatBitsToBytes: unpack integer, split into fields.
func bytesToFloatBits(data []byte, format fp.FloatFormat) (fp.FloatBits, error) {
	byteWidth := floatByteWidth(format)
	var bits int

	switch byteWidth {
	case 4:
		bits = int(binary.LittleEndian.Uint32(data))
	case 2:
		bits = int(binary.LittleEndian.Uint16(data))
	default:
		return fp.FloatBits{}, fmt.Errorf("unsupported float width: %d bytes", byteWidth)
	}

	totalBits := format.TotalBits
	mantissaBits := format.MantissaBits
	exponentBits := format.ExponentBits

	// Mantissa is the lowest mantissa_bits bits.
	mantissaMask := (1 << mantissaBits) - 1
	mantissaInt := bits & mantissaMask
	mantissa := make([]int, mantissaBits)
	for i := 0; i < mantissaBits; i++ {
		mantissa[i] = (mantissaInt >> (mantissaBits - 1 - i)) & 1
	}

	// Exponent is the next exponent_bits bits.
	exponentMask := (1 << exponentBits) - 1
	exponentInt := (bits >> mantissaBits) & exponentMask
	exponent := make([]int, exponentBits)
	for i := 0; i < exponentBits; i++ {
		exponent[i] = (exponentInt >> (exponentBits - 1 - i)) & 1
	}

	// Sign is the highest bit.
	sign := (bits >> (totalBits - 1)) & 1

	return fp.FloatBits{
		Sign:     sign,
		Exponent: exponent,
		Mantissa: mantissa,
		Fmt:      format,
	}, nil
}

// LoadFloat loads a floating-point value from memory.
//
// Reads the appropriate number of bytes (4 for FP32, 2 for FP16/BF16)
// starting at the given address, and converts them to a FloatBits.
func (m *LocalMemory) LoadFloat(address int, format fp.FloatFormat) (fp.FloatBits, error) {
	return StartNew[fp.FloatBits]("gpu-core.LocalMemory.LoadFloat", fp.FloatBits{},
		func(op *Operation[fp.FloatBits], rf *ResultFactory[fp.FloatBits]) *OperationResult[fp.FloatBits] {
			byteWidth := floatByteWidth(format)
			data, err := m.ReadBytes(address, byteWidth)
			if err != nil {
				return rf.Fail(fp.FloatBits{}, err)
			}
			bits, err := bytesToFloatBits(data, format)
			if err != nil {
				return rf.Fail(fp.FloatBits{}, err)
			}
			return rf.Generate(true, false, bits)
		}).GetResult()
}

// StoreFloat stores a floating-point value to memory.
//
// Converts the FloatBits to bytes and writes them starting at the given address.
func (m *LocalMemory) StoreFloat(address int, value fp.FloatBits) error {
	_, err := StartNew[struct{}]("gpu-core.LocalMemory.StoreFloat", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			data, err := floatBitsToBytes(value)
			if err != nil {
				return rf.Fail(struct{}{}, err)
			}
			if err := m.WriteBytes(address, data); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// LoadFloatAsGo is a convenience method: load a float and convert to Go float64.
func (m *LocalMemory) LoadFloatAsGo(address int, format fp.FloatFormat) (float64, error) {
	return StartNew[float64]("gpu-core.LocalMemory.LoadFloatAsGo", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			bits, err := m.LoadFloat(address, format)
			if err != nil {
				return rf.Fail(0, err)
			}
			return rf.Generate(true, false, fp.BitsToFloat(bits))
		}).GetResult()
}

// StoreGoFloat is a convenience method: store a Go float64 to memory.
func (m *LocalMemory) StoreGoFloat(address int, value float64, format fp.FloatFormat) error {
	_, err := StartNew[struct{}]("gpu-core.LocalMemory.StoreGoFloat", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if err := m.StoreFloat(address, fp.FloatToBits(value, format)); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// Dump returns a slice of memory as a list of byte values.
//
// Useful for debugging. Default shows the first 64 bytes.
func (m *LocalMemory) Dump(start, length int) []byte {
	result, _ := StartNew[[]byte]("gpu-core.LocalMemory.Dump", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			end := start + length
			if end > m.Size {
				end = m.Size
			}
			out := make([]byte, end-start)
			copy(out, m.data[start:end])
			return rf.Generate(true, false, out)
		}).GetResult()
	return result
}

// String returns a human-readable representation of the memory.
func (m *LocalMemory) String() string {
	result, _ := StartNew[string]("gpu-core.LocalMemory.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			used := 0
			for _, b := range m.data {
				if b != 0 {
					used++
				}
			}
			return rf.Generate(true, false, fmt.Sprintf("LocalMemory(%d bytes, %d non-zero)", m.Size, used))
		}).GetResult()
	return result
}
