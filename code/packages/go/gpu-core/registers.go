package gpucore

// FPRegisterFile -- floating-point register storage for GPU cores.
//
// # What is a Register File?
//
// A register file is the fastest storage in a processor -- faster than cache,
// faster than RAM. It's where the processor keeps the values it's currently
// working with. Think of it like the handful of numbers you can keep in your
// head while doing mental math.
//
//	Register file (in your head):
//	    "first number"  = 3.14
//	    "second number" = 2.71
//	    "result"        = ???
//
//	Register file (in a GPU core):
//	    R0  = 3.14  (FloatBits: sign=0, exp=[...], mantissa=[...])
//	    R1  = 2.71  (FloatBits: sign=0, exp=[...], mantissa=[...])
//	    R2  = 0.00  (will hold the result)
//
// # GPU vs CPU Register Files
//
// CPU registers hold integers (32 or 64 bits of binary). GPU registers hold
// floating-point numbers (IEEE 754 FloatBits). This reflects their different
// purposes:
//
//	CPU: general-purpose computation (loops, pointers, addresses -> integers)
//	GPU: parallel numeric computation (vertices, pixels, gradients -> floats)
//
// # Why Configurable?
//
// Different GPU vendors use different register counts:
//
//	NVIDIA CUDA Core:    up to 255 registers per thread
//	AMD Stream Processor: 256 VGPRs (Vector General Purpose Registers)
//	Intel Vector Engine:  128 GRF entries (General Register File)
//	ARM Mali:            64 registers per thread
//
// By making the register count a constructor parameter, the same GPUCore
// struct can simulate any vendor's register architecture.
//
// # Register File Diagram
//
//	+------------------------------------------+
//	|           FP Register File               |
//	|         (32 registers x FP32)            |
//	+------------------------------------------+
//	|  R0:  [0][01111111][00000000000...0]     |  = +1.0
//	|  R1:  [0][10000000][00000000000...0]     |  = +2.0
//	|  R2:  [0][00000000][00000000000...0]     |  = +0.0
//	|  ...                                     |
//	|  R31: [0][00000000][00000000000...0]     |  = +0.0
//	+------------------------------------------+

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
)

// FPRegisterFile is a configurable floating-point register file.
//
// It stores FloatBits values (from the fp-arithmetic package) in a fixed
// number of registers. Provides both raw FloatBits and convenience float64
// interfaces for reading and writing.
type FPRegisterFile struct {
	// NumRegisters is how many registers this file contains.
	NumRegisters int

	// Fmt is the floating-point format (FP32, FP16, BF16).
	Fmt fp.FloatFormat

	// values holds the actual register contents.
	values []fp.FloatBits

	// zero is the cached zero value for this format.
	zero fp.FloatBits
}

// NewFPRegisterFile creates a new register file with the given number of
// registers and floating-point format.
//
// All registers are initialized to +0.0 in the specified format.
//
// Arguments:
//   - numRegisters: How many registers (1-256). Default in GPU cores is 32.
//   - fmt: The floating-point format. Use fparithmetic.FP32 for standard.
//
// Returns an error if numRegisters is out of range [1, 256].
func NewFPRegisterFile(numRegisters int, format fp.FloatFormat) (*FPRegisterFile, error) {
	return StartNew[*FPRegisterFile]("gpu-core.NewFPRegisterFile", nil,
		func(op *Operation[*FPRegisterFile], rf *ResultFactory[*FPRegisterFile]) *OperationResult[*FPRegisterFile] {
			if numRegisters < 1 || numRegisters > 256 {
				return rf.Fail(nil, fmt.Errorf("num_registers must be 1-256, got %d", numRegisters))
			}

			zero := fp.FloatToBits(0.0, format)
			values := make([]fp.FloatBits, numRegisters)
			for i := range values {
				values[i] = zero
			}

			return rf.Generate(true, false, &FPRegisterFile{
				NumRegisters: numRegisters,
				Fmt:          format,
				values:       values,
				zero:         zero,
			})
		}).GetResult()
}

// checkIndex validates a register index, returning an error if out of bounds.
func (rf *FPRegisterFile) checkIndex(index int) error {
	if index < 0 || index >= rf.NumRegisters {
		return fmt.Errorf("register index %d out of range [0, %d]", index, rf.NumRegisters-1)
	}
	return nil
}

// Read reads a register as a FloatBits value.
//
// Arguments:
//   - index: Register number (0 to NumRegisters-1).
//
// Returns the FloatBits value stored in that register, or an error if the
// index is out of range.
func (rf *FPRegisterFile) Read(index int) (fp.FloatBits, error) {
	return StartNew[fp.FloatBits]("gpu-core.FPRegisterFile.Read", fp.FloatBits{},
		func(op *Operation[fp.FloatBits], res *ResultFactory[fp.FloatBits]) *OperationResult[fp.FloatBits] {
			if err := rf.checkIndex(index); err != nil {
				return res.Fail(fp.FloatBits{}, err)
			}
			return res.Generate(true, false, rf.values[index])
		}).GetResult()
}

// Write stores a FloatBits value in a register.
//
// Arguments:
//   - index: Register number (0 to NumRegisters-1).
//   - value: The FloatBits value to store.
//
// Returns an error if the index is out of range.
func (rf *FPRegisterFile) Write(index int, value fp.FloatBits) error {
	_, err := StartNew[struct{}]("gpu-core.FPRegisterFile.Write", struct{}{},
		func(op *Operation[struct{}], res *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if err := rf.checkIndex(index); err != nil {
				return res.Fail(struct{}{}, err)
			}
			rf.values[index] = value
			return res.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// ReadFloat is a convenience method that reads a register as a Go float64.
//
// This decodes the FloatBits back to a float64, which is useful for
// inspection and testing but loses the bit-level detail.
func (rf *FPRegisterFile) ReadFloat(index int) (float64, error) {
	return StartNew[float64]("gpu-core.FPRegisterFile.ReadFloat", 0,
		func(op *Operation[float64], res *ResultFactory[float64]) *OperationResult[float64] {
			bits, err := rf.Read(index)
			if err != nil {
				return res.Fail(0, err)
			}
			return res.Generate(true, false, fp.BitsToFloat(bits))
		}).GetResult()
}

// WriteFloat is a convenience method that writes a Go float64 to a register.
//
// This encodes the float64 as FloatBits in the register file's format,
// then stores it. Useful for setting up test inputs.
func (rf *FPRegisterFile) WriteFloat(index int, value float64) error {
	_, err := StartNew[struct{}]("gpu-core.FPRegisterFile.WriteFloat", struct{}{},
		func(op *Operation[struct{}], res *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if err := rf.Write(index, fp.FloatToBits(value, rf.Fmt)); err != nil {
				return res.Fail(struct{}{}, err)
			}
			return res.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// Dump returns all non-zero register values as a map of "R{n}" -> float64.
//
// Useful for debugging and test assertions. Only includes non-zero
// registers to reduce noise.
func (rf *FPRegisterFile) Dump() map[string]float64 {
	result, _ := StartNew[map[string]float64]("gpu-core.FPRegisterFile.Dump", nil,
		func(op *Operation[map[string]float64], res *ResultFactory[map[string]float64]) *OperationResult[map[string]float64] {
			out := make(map[string]float64)
			for i := 0; i < rf.NumRegisters; i++ {
				val := fp.BitsToFloat(rf.values[i])
				if val != 0.0 {
					out[fmt.Sprintf("R%d", i)] = val
				}
			}
			return res.Generate(true, false, out)
		}).GetResult()
	return result
}

// DumpAll returns ALL register values as a map of "R{n}" -> float64.
//
// Unlike Dump(), this includes zero-valued registers.
func (rf *FPRegisterFile) DumpAll() map[string]float64 {
	result, _ := StartNew[map[string]float64]("gpu-core.FPRegisterFile.DumpAll", nil,
		func(op *Operation[map[string]float64], res *ResultFactory[map[string]float64]) *OperationResult[map[string]float64] {
			out := make(map[string]float64)
			for i := 0; i < rf.NumRegisters; i++ {
				out[fmt.Sprintf("R%d", i)] = fp.BitsToFloat(rf.values[i])
			}
			return res.Generate(true, false, out)
		}).GetResult()
	return result
}

// String returns a human-readable representation of the register file.
func (rf *FPRegisterFile) String() string {
	result, _ := StartNew[string]("gpu-core.FPRegisterFile.String", "",
		func(op *Operation[string], res *ResultFactory[string]) *OperationResult[string] {
			nonZero := rf.Dump()
			if len(nonZero) == 0 {
				return res.Generate(true, false, fmt.Sprintf("FPRegisterFile(%d regs, all zero)", rf.NumRegisters))
			}
			entries := ""
			for i := 0; i < rf.NumRegisters; i++ {
				key := fmt.Sprintf("R%d", i)
				if val, ok := nonZero[key]; ok {
					if entries != "" {
						entries += ", "
					}
					entries += fmt.Sprintf("%s=%g", key, val)
				}
			}
			return res.Generate(true, false, fmt.Sprintf("FPRegisterFile(%s)", entries))
		}).GetResult()
	return result
}
