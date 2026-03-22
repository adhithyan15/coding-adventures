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
	cfg := DefaultRegisterFileConfig()
	if config != nil {
		cfg = *config
	}

	// Compute the bit mask for the register width.
	// For 32-bit: mask = 0xFFFFFFFF
	// For 64-bit: mask = 0x7FFFFFFFFFFFFFFF (Go's int is at least 64 bits)
	var mask int
	if cfg.Width >= 64 {
		mask = int(^uint(0) >> 1) // max int
	} else {
		mask = (1 << cfg.Width) - 1
	}

	return &RegisterFile{
		config: cfg,
		values: make([]int, cfg.Count),
		mask:   mask,
	}
}

// Read returns the value of register at the given index.
//
// If the zero register convention is enabled, reading register 0 always
// returns 0, regardless of what was written to it.
//
// Returns 0 if the index is out of range (defensive -- avoids panics in
// the pipeline, which processes untrusted instruction data).
func (r *RegisterFile) Read(index int) int {
	if index < 0 || index >= r.config.Count {
		return 0
	}
	if r.config.ZeroRegister && index == 0 {
		return 0
	}
	return r.values[index]
}

// Write stores a value into the register at the given index.
//
// The value is masked to the register width (e.g., 32-bit mask for 32-bit
// registers). Writes to register 0 are silently ignored when the zero
// register convention is enabled.
//
// Writes to out-of-range indices are silently ignored (defensive).
func (r *RegisterFile) Write(index int, value int) {
	if index < 0 || index >= r.config.Count {
		return
	}
	if r.config.ZeroRegister && index == 0 {
		return // writes to zero register are discarded
	}
	r.values[index] = value & r.mask
}

// Values returns a copy of all register values (for inspection and debugging).
func (r *RegisterFile) Values() []int {
	result := make([]int, len(r.values))
	copy(result, r.values)
	return result
}

// Count returns the number of registers.
func (r *RegisterFile) Count() int {
	return r.config.Count
}

// Width returns the bit width of each register.
func (r *RegisterFile) Width() int {
	return r.config.Width
}

// Config returns the register file configuration.
func (r *RegisterFile) Config() RegisterFileConfig {
	return r.config
}

// Reset sets all registers to zero.
func (r *RegisterFile) Reset() {
	for i := range r.values {
		r.values[i] = 0
	}
}

// String returns a human-readable dump of all registers.
//
// Format:
//
//	RegisterFile(16x32): R0=0 R1=42 R2=100 ...
func (r *RegisterFile) String() string {
	s := fmt.Sprintf("RegisterFile(%dx%d):", r.config.Count, r.config.Width)
	for i := 0; i < r.config.Count; i++ {
		if r.values[i] != 0 {
			s += fmt.Sprintf(" R%d=%d", i, r.values[i])
		}
	}
	return s
}
