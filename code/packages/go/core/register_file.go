package core

import "fmt"

// =========================================================================
// RegisterFile -- general-purpose register file for the Core
// =========================================================================

// RegisterFile is the Core's register file -- fast, small storage that the
// pipeline reads and writes every cycle.
//
// # Why a Custom Register File?
//
// The cpu-simulator package has a RegisterFile, but it uses uint32 values
// and panics on out-of-range access. The Core needs a register file that:
//
//   - Uses int values (matching PipelineToken fields)
//   - Supports configurable width (32 or 64 bit)
//   - Optionally hardwires register 0 to zero (RISC-V convention)
//   - Returns errors instead of panicking
//
// This register file wraps the concept but provides a Core-friendly API.
//
// # Zero Register Convention
//
// In RISC-V and MIPS, register x0 (or $zero) is hardwired to the value 0.
// Writes to it are silently discarded. This simplifies instruction encoding:
//
//	MOV Rd, Rs  = ADD Rd, Rs, x0   (add zero)
//	NOP         = ADD x0, x0, x0   (write nothing to zero register)
//	NEG Rd, Rs  = SUB Rd, x0, Rs   (subtract from zero)
//
// ARM does NOT have a zero register (all 31 registers are general-purpose).
// x86 does not have one either. The ZeroRegister config controls this.
type RegisterFile struct {
	// config holds the register file configuration.
	config RegisterFileConfig

	// values stores the register values. values[0] is R0.
	values []int

	// mask is the bit mask for the register width (e.g., 0xFFFFFFFF for 32-bit).
	mask int
}

// NewRegisterFile creates a new register file from the given configuration.
//
// All registers are initialized to 0. If config is nil, the default
// configuration is used (16 registers, 32-bit, zero register enabled).
func NewRegisterFile(config *RegisterFileConfig) *RegisterFile {
	result, _ := StartNew[*RegisterFile]("core.NewRegisterFile", nil,
		func(op *Operation[*RegisterFile], rf *ResultFactory[*RegisterFile]) *OperationResult[*RegisterFile] {
			cfg := DefaultRegisterFileConfig()
			if config != nil {
				cfg = *config
			}

			var mask int
			if cfg.Width >= 64 {
				mask = int(^uint(0) >> 1)
			} else {
				mask = (1 << cfg.Width) - 1
			}

			return rf.Generate(true, false, &RegisterFile{
				config: cfg,
				values: make([]int, cfg.Count),
				mask:   mask,
			})
		}).GetResult()
	return result
}

// Read returns the value of register at the given index.
//
// If the zero register convention is enabled, reading register 0 always
// returns 0, regardless of what was written to it.
//
// Returns 0 if the index is out of range (defensive -- avoids panics in
// the pipeline, which processes untrusted instruction data).
func (r *RegisterFile) Read(index int) int {
	result, _ := StartNew[int]("core.RegisterFile.Read", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("index", index)
			if index < 0 || index >= r.config.Count {
				return rf.Generate(true, false, 0)
			}
			if r.config.ZeroRegister && index == 0 {
				return rf.Generate(true, false, 0)
			}
			return rf.Generate(true, false, r.values[index])
		}).GetResult()
	return result
}

// Write stores a value into the register at the given index.
//
// The value is masked to the register width (e.g., 32-bit mask for 32-bit
// registers). Writes to register 0 are silently ignored when the zero
// register convention is enabled.
//
// Writes to out-of-range indices are silently ignored (defensive).
func (r *RegisterFile) Write(index int, value int) {
	_, _ = StartNew[struct{}]("core.RegisterFile.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("index", index)
			if index < 0 || index >= r.config.Count {
				return rf.Generate(true, false, struct{}{})
			}
			if r.config.ZeroRegister && index == 0 {
				return rf.Generate(true, false, struct{}{})
			}
			r.values[index] = value & r.mask
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Values returns a copy of all register values (for inspection and debugging).
func (r *RegisterFile) Values() []int {
	result, _ := StartNew[[]int]("core.RegisterFile.Values", nil,
		func(op *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			res := make([]int, len(r.values))
			copy(res, r.values)
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// Count returns the number of registers.
func (r *RegisterFile) Count() int {
	result, _ := StartNew[int]("core.RegisterFile.Count", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, r.config.Count)
		}).GetResult()
	return result
}

// Width returns the bit width of each register.
func (r *RegisterFile) Width() int {
	result, _ := StartNew[int]("core.RegisterFile.Width", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, r.config.Width)
		}).GetResult()
	return result
}

// Config returns the register file configuration.
func (r *RegisterFile) Config() RegisterFileConfig {
	result, _ := StartNew[RegisterFileConfig]("core.RegisterFile.Config", RegisterFileConfig{},
		func(op *Operation[RegisterFileConfig], rf *ResultFactory[RegisterFileConfig]) *OperationResult[RegisterFileConfig] {
			return rf.Generate(true, false, r.config)
		}).GetResult()
	return result
}

// Reset sets all registers to zero.
func (r *RegisterFile) Reset() {
	_, _ = StartNew[struct{}]("core.RegisterFile.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for i := range r.values {
				r.values[i] = 0
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// String returns a human-readable dump of all registers.
//
// Format:
//
//	RegisterFile(16x32): R0=0 R1=42 R2=100 ...
func (r *RegisterFile) String() string {
	result, _ := StartNew[string]("core.RegisterFile.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			s := fmt.Sprintf("RegisterFile(%dx%d):", r.config.Count, r.config.Width)
			for i := 0; i < r.config.Count; i++ {
				if r.values[i] != 0 {
					s += fmt.Sprintf(" R%d=%d", i, r.values[i])
				}
			}
			return rf.Generate(true, false, s)
		}).GetResult()
	return result
}
