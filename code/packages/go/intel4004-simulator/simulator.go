// Package intel4004simulator implements the first commercial processor layout.
//
// === What is the Intel 4004? ===
//
// The Intel 4004 was the world's first commercial single-chip microprocessor,
// released by Intel in 1971. The entire processor contained just 2,300 transistors.
//
// === Why 4-bit? ===
//
// The 4004 is natively 4-bit. Every data value is 4 bits wide (0-15).
// All computations are forcefully masked to 4 bits (& 0xF). There is no native support
// for 8-bit, 16-bit, or 32-bit math representations anywhere in the data path. 
//
// === Accumulator architecture ===
//
// Operations aren't allowed between registers generically. Instead they are funneled entirely
// through the Accumulator:
// To Add Contextually: Load Accumulator value A, swap to register R0, Load Accumulator value B, Add R0 to A.
package intel4004simulator

import (
	"fmt"
)

// Intel4004Trace logs step execution modifications directly on the generic Accumulator/Carry constraints.
type Intel4004Trace struct {
	Address           int
	Raw               int
	Mnemonic          string
	AccumulatorBefore int
	AccumulatorAfter  int
	CarryBefore       bool
	CarryAfter        bool
}

// Intel4004Simulator stands on its own. Generic generic cpu-simulator relies too heavily on 32-bit.
type Intel4004Simulator struct {
	Accumulator int // 0-15
	Registers   []int // array 16, values 0-15
	Carry       bool
	Memory      []byte
	PC          int
	Halted      bool
}

func NewIntel4004Simulator(memorySize int) *Intel4004Simulator {
	return &Intel4004Simulator{
		Accumulator: 0,
		Registers:   make([]int, 16),
		Carry:       false,
		Memory:      make([]byte, memorySize),
		PC:          0,
		Halted:      false,
	}
}

func (s *Intel4004Simulator) LoadProgram(program []byte) {
	for i, b := range program {
		s.Memory[i] = b
	}
	s.PC = 0
	s.Halted = false
}

// Step performs explicit fetch-decode-execute linearly without decoupling into modular interfaces.
func (s *Intel4004Simulator) Step() Intel4004Trace {
	if s.Halted {
		panic("CPU is halted")
	}

	address := s.PC
	raw := int(s.Memory[s.PC])
	s.PC++

	accBefore := s.Accumulator
	carryBefore := s.Carry

	// Decoding the 8-bit instruction payload
	opcode := (raw >> 4) & 0xF
	operand := raw & 0xF

	mnemonic := s.execute(opcode, operand, raw)

	return Intel4004Trace{
		Address:           address,
		Raw:               raw,
		Mnemonic:          mnemonic,
		AccumulatorBefore: accBefore,
		AccumulatorAfter:  s.Accumulator,
		CarryBefore:       carryBefore,
		CarryAfter:        s.Carry,
	}
}

// execute resolves Accumulator updates locally based directly on the decoded Nibbles.
func (s *Intel4004Simulator) execute(opcode, operand, raw int) string {
	if opcode == 0xD { // LDM N
		s.Accumulator = operand & 0xF
		return fmt.Sprintf("LDM %d", operand)
	} else if opcode == 0xB { // XCH RN
		reg := operand & 0xF
		oldA := s.Accumulator
		s.Accumulator = s.Registers[reg] & 0xF
		s.Registers[reg] = oldA & 0xF
		return fmt.Sprintf("XCH R%d", reg)
	} else if opcode == 0x8 { // ADD RN
		reg := operand & 0xF
		result := s.Accumulator + s.Registers[reg]
		s.Carry = result > 0xF
		s.Accumulator = result & 0xF
		return fmt.Sprintf("ADD R%d", reg)
	} else if opcode == 0x9 { // SUB RN
		reg := operand & 0xF
		result := s.Accumulator - s.Registers[reg]
		s.Carry = result < 0
		s.Accumulator = result & 0xF
		return fmt.Sprintf("SUB R%d", reg)
	} else if raw == 0x01 { // HLT (custom testing instruction)
		s.Halted = true
		return "HLT"
	}
	return fmt.Sprintf("UNKNOWN(0x%02X)", raw)
}

func (s *Intel4004Simulator) Run(program []byte, maxSteps int) []Intel4004Trace {
	s.LoadProgram(program)
	var traces []Intel4004Trace
	for i := 0; i < maxSteps; i++ {
		if s.Halted {
			break
		}
		trace := s.Step()
		traces = append(traces, trace)
	}
	return traces
}

// Encoders supporting testing verification
func EncodeLdm(n int) byte { return byte((0xD << 4) | (n & 0xF)) }
func EncodeXch(r int) byte { return byte((0xB << 4) | (r & 0xF)) }
func EncodeAdd(r int) byte { return byte((0x8 << 4) | (r & 0xF)) }
func EncodeSub(r int) byte { return byte((0x9 << 4) | (r & 0xF)) }
func EncodeHlt() byte      { return 0x01 }
