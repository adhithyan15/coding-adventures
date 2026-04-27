// Package logicgates implements fundamental digital logic gates and
// sequential circuits — the building blocks of every digital computer.
//
// # Why logic gates matter
//
// Every computation your CPU performs — from adding numbers to rendering
// 3D graphics — ultimately reduces to billions of tiny switches called
// transistors flipping between on (1) and off (0). Logic gates are the
// first abstraction layer above transistors: they combine a few transistors
// into a circuit that performs a Boolean operation.
//
// From just NAND gates (or just NOR gates), you can build every other gate.
// From gates, you build adders. From adders, you build ALUs. From ALUs,
// you build CPUs. This package implements that foundational layer.
//
// # The seven fundamental gates
//
// There are exactly seven Boolean functions of two variables that have
// standard gate names:
//
//	Gate    | Symbol | Truth Table (A,B → Out)
//	--------|--------|-------------------------
//	AND     |  A·B   | 0,0→0  0,1→0  1,0→0  1,1→1
//	OR      |  A+B   | 0,0→0  0,1→1  1,0→1  1,1→1
//	NOT     |  ¬A    | 0→1  1→0  (unary gate)
//	XOR     |  A⊕B   | 0,0→0  0,1→1  1,0→1  1,1→0
//	NAND    | ¬(A·B) | 0,0→1  0,1→1  1,0→1  1,1→0
//	NOR     | ¬(A+B) | 0,0→1  0,1→0  1,0→0  1,1→0
//	XNOR    | ¬(A⊕B) | 0,0→1  0,1→0  1,0→0  1,1→1
//
// # NAND as the universal gate
//
// NAND is called a "universal gate" because any other gate can be built
// from NANDs alone. This is not just a theoretical curiosity — real chip
// fabrication processes (like CMOS) often build everything from NAND or
// NOR gates because their transistor layouts are simpler and faster.
//
//	NOT from NAND:   NAND(A, A) = ¬A
//	AND from NAND:   NAND(NAND(A,B), NAND(A,B)) = A·B
//	OR from NAND:    NAND(NAND(A,A), NAND(B,B)) = A+B
//	XOR from NAND:   NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))
//
// This package provides NAND-derived versions of each gate to demonstrate
// functional completeness.
//
// # Input conventions
//
// All inputs must be 0 or 1 (representing low/high voltage in hardware).
// Functions panic on invalid inputs. In real hardware, voltages outside
// the valid range cause undefined behavior — our panic is the software
// equivalent of "the chip does something unpredictable."
//
// # Operations
//
// Every public function is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery.
package logicgates

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/transistors"
)

// cmosAnd, cmosOr, cmosNot, cmosXor, cmosXnor, cmosNand, cmosNor are package-level
// CMOS gate instances created once at startup. Using default circuit parameters
// (3.3 V Vdd, 180 nm CMOS node). All seven fundamental gates now delegate their
// digital evaluation to these CMOS transistor models, reflecting the physical
// reality that logic gates are built from transistors.
var (
	_cmosAnd  = transistors.NewCMOSAnd(nil)
	_cmosOr   = transistors.NewCMOSOr(nil)
	_cmosNot  = transistors.NewCMOSInverter(nil, nil, nil)
	_cmosXor  = transistors.NewCMOSXor(nil)
	_cmosXnor = transistors.NewCMOSXnor(nil)
	_cmosNand = transistors.NewCMOSNand(nil, nil, nil)
	_cmosNor  = transistors.NewCMOSNor(nil, nil, nil)
)

// =========================================================================
// Input Validation
// =========================================================================

// validateBit checks that a value is a valid binary digit (0 or 1).
//
// In digital electronics, a "bit" is a signal that is either LOW (0) or
// HIGH (1). Anything else is meaningless — there is no "2" in binary.
// Real hardware enforces this through voltage thresholds; we enforce it
// with a runtime check.
func validateBit(value int, name string) {
	if value != 0 && value != 1 {
		panic(fmt.Sprintf("logicgates: %s must be 0 or 1, got %d", name, value))
	}
}

// validateBits checks that all values in a slice are valid binary digits.
func validateBits(values []int, name string) {
	for i, v := range values {
		if v != 0 && v != 1 {
			panic(fmt.Sprintf("logicgates: %s[%d] must be 0 or 1, got %d", name, i, v))
		}
	}
}

// =========================================================================
// The Seven Fundamental Gates
// =========================================================================

// AND returns 1 only when BOTH inputs are 1.
//
// Circuit diagram (two transistors in series):
//
//	    Vcc (+)
//	     |
//	     R  (pull-up resistor)
//	     |
//	     +--- Output
//	     |
//	    [A]  (transistor controlled by input A)
//	     |
//	    [B]  (transistor controlled by input B)
//	     |
//	    GND
//
// Both transistors must be ON (both inputs = 1) for current to flow
// from Vcc through the resistor to ground, pulling the output LOW.
// Wait — that gives us NAND! In practice, AND gates are built as
// NAND followed by NOT. We show the logical behavior here.
//
// Truth table:
//
//	A | B | A AND B
//	--|---|--------
//	0 | 0 |   0
//	0 | 1 |   0
//	1 | 0 |   0
//	1 | 1 |   1
//
// Real-world use: AND gates are used in address decoders (is this
// BOTH the right row AND the right column?), enable signals (is the
// chip selected AND the clock active?), and masking operations.
func AND(a, b int) int {
	result, _ := StartNew[int]("logic-gates.AND", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			validateBit(a, "a")
			validateBit(b, "b")
			// Delegate to the CMOS AND gate (NAND + inverter = 6 transistors).
			// EvaluateDigital returns (int, error); panic on error since we've
			// already validated inputs, making an error impossible.
			res, err := _cmosAnd.EvaluateDigital(a, b)
			if err != nil {
				panic(fmt.Sprintf("logicgates: AND CMOS evaluation error: %v", err))
			}
			return rf.Generate(true, false, res)
		}).PanicOnUnexpected().GetResult()
	return result
}

// OR returns 1 when AT LEAST ONE input is 1.
//
// Circuit diagram (two transistors in parallel):
//
//	    Vcc (+)
//	     |
//	     R  (pull-up resistor)
//	     |
//	     +--- Output
//	    / \
//	  [A] [B]  (transistors in parallel)
//	    \ /
//	     |
//	    GND
//
// Either transistor being ON creates a path to ground. Like AND,
// the raw circuit gives NOR, so OR = NOR + NOT in hardware.
//
// Truth table:
//
//	A | B | A OR B
//	--|---|-------
//	0 | 0 |   0
//	0 | 1 |   1
//	1 | 0 |   1
//	1 | 1 |   1
//
// Real-world use: interrupt controllers (did ANY device signal?),
// bus arbitration, combining error flags.
func OR(a, b int) int {
	result, _ := StartNew[int]("logic-gates.OR", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			validateBit(a, "a")
			validateBit(b, "b")
			// Delegate to the CMOS OR gate (NOR + inverter = 6 transistors).
			res, err := _cmosOr.EvaluateDigital(a, b)
			if err != nil {
				panic(fmt.Sprintf("logicgates: OR CMOS evaluation error: %v", err))
			}
			return rf.Generate(true, false, res)
		}).PanicOnUnexpected().GetResult()
	return result
}

// NOT inverts its input: 0 becomes 1, 1 becomes 0.
//
// Circuit diagram (single transistor inverter):
//
//	    Vcc (+)
//	     |
//	     R  (pull-up resistor)
//	     |
//	     +--- Output
//	     |
//	    [A]  (transistor controlled by input A)
//	     |
//	    GND
//
// When A = 1, the transistor conducts, pulling output to GND (0).
// When A = 0, the transistor is off, output floats up to Vcc (1).
// This is the simplest possible gate — just one transistor.
//
// Truth table:
//
//	A | NOT A
//	--|------
//	0 |   1
//	1 |   0
//
// Real-world use: NOT is everywhere — clock inversion, active-low
// signals, complementary outputs, building other gates.
func NOT(a int) int {
	result, _ := StartNew[int]("logic-gates.NOT", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			validateBit(a, "a")
			// Delegate to the CMOS inverter (2 transistors: 1 PMOS + 1 NMOS).
			res, err := _cmosNot.EvaluateDigital(a)
			if err != nil {
				panic(fmt.Sprintf("logicgates: NOT CMOS evaluation error: %v", err))
			}
			return rf.Generate(true, false, res)
		}).PanicOnUnexpected().GetResult()
	return result
}

// XOR (exclusive OR) returns 1 when inputs DIFFER.
//
// XOR answers the question: "Are these two bits different?"
// This makes it invaluable for comparison, parity checking,
// and arithmetic (it's the core of binary addition).
//
// Truth table:
//
//	A | B | A XOR B
//	--|---|--------
//	0 | 0 |   0
//	0 | 1 |   1
//	1 | 0 |   1
//	1 | 1 |   0
//
// Notice: XOR is like OR but "exclusive" — it excludes the case
// where both inputs are 1. Another way to think about it:
//   XOR = (A OR B) AND NOT (A AND B)
//
// Real-world use: binary addition (half adder sum bit), parity
// generators, CRC checksums, cryptographic operations, toggling
// bits in registers.
func XOR(a, b int) int {
	result, _ := StartNew[int]("logic-gates.XOR", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			validateBit(a, "a")
			validateBit(b, "b")
			// Delegate to the CMOS XOR gate (4 NAND gates = 16 transistors).
			res, err := _cmosXor.EvaluateDigital(a, b)
			if err != nil {
				panic(fmt.Sprintf("logicgates: XOR CMOS evaluation error: %v", err))
			}
			return rf.Generate(true, false, res)
		}).PanicOnUnexpected().GetResult()
	return result
}

// NAND returns 0 only when BOTH inputs are 1.
//
// NAND = NOT(AND). It is the "universal gate" — you can build
// ANY other Boolean function using only NAND gates. This is not
// just theory: early computers (like the Apollo Guidance Computer)
// were built entirely from NOR gates, and modern CMOS fabrication
// naturally produces NAND/NOR as the primitive operation.
//
// Truth table:
//
//	A | B | A NAND B
//	--|---|--------
//	0 | 0 |   1
//	0 | 1 |   1
//	1 | 0 |   1
//	1 | 1 |   0
//
// Why NAND is special:
//   - NAND(A,A) = NOT(A)        → gives us NOT
//   - NOT(NAND(A,B)) = AND(A,B) → gives us AND
//   - NAND(NOT(A),NOT(B)) = OR  → gives us OR
//   - From these, we can build XOR, MUX, adders, memory...
//
// Real-world use: NAND flash memory (the storage in SSDs) is named
// after this gate because its memory cells are arranged in a NAND
// configuration.
func NAND(a, b int) int {
	result, _ := StartNew[int]("logic-gates.NAND", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			validateBit(a, "a")
			validateBit(b, "b")
			// Delegate to the CMOS NAND gate (4 transistors — the natural CMOS gate).
			res, err := _cmosNand.EvaluateDigital(a, b)
			if err != nil {
				panic(fmt.Sprintf("logicgates: NAND CMOS evaluation error: %v", err))
			}
			return rf.Generate(true, false, res)
		}).PanicOnUnexpected().GetResult()
	return result
}

// NOR returns 1 only when BOTH inputs are 0.
//
// NOR = NOT(OR). Like NAND, NOR is also a universal gate — you
// can build any circuit from NOR gates alone. The Apollo Guidance
// Computer (which landed humans on the Moon) used about 5,600
// NOR gates as its only logic element.
//
// Truth table:
//
//	A | B | A NOR B
//	--|---|--------
//	0 | 0 |   1
//	0 | 1 |   0
//	1 | 0 |   0
//	1 | 1 |   0
//
// Real-world use: SR latches (the simplest memory element) are
// built from two cross-coupled NOR gates. We implement this in
// sequential.go.
func NOR(a, b int) int {
	result, _ := StartNew[int]("logic-gates.NOR", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			validateBit(a, "a")
			validateBit(b, "b")
			// Delegate to the CMOS NOR gate (4 transistors — the other natural CMOS gate).
			res, err := _cmosNor.EvaluateDigital(a, b)
			if err != nil {
				panic(fmt.Sprintf("logicgates: NOR CMOS evaluation error: %v", err))
			}
			return rf.Generate(true, false, res)
		}).PanicOnUnexpected().GetResult()
	return result
}

// XNOR returns 1 when inputs are the SAME.
//
// XNOR = NOT(XOR). It is the "equivalence" gate — it answers
// "are these two bits equal?" This is the complement of XOR.
//
// Truth table:
//
//	A | B | A XNOR B
//	--|---|--------
//	0 | 0 |   1
//	0 | 1 |   0
//	1 | 0 |   0
//	1 | 1 |   1
//
// Real-world use: equality comparators (are these two bus lines
// carrying the same value?), error detection circuits.
func XNOR(a, b int) int {
	result, _ := StartNew[int]("logic-gates.XNOR", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			validateBit(a, "a")
			validateBit(b, "b")
			// Delegate to the dedicated CMOS XNOR gate (XOR + Inverter = 8 transistors).
			res, err := _cmosXnor.EvaluateDigital(a, b)
			if err != nil {
				panic(fmt.Sprintf("logicgates: XNOR CMOS evaluation error: %v", err))
			}
			return rf.Generate(true, false, res)
		}).PanicOnUnexpected().GetResult()
	return result
}

// =========================================================================
// NAND-Derived Gates (Proving Functional Completeness)
// =========================================================================
//
// The following functions rebuild each fundamental gate using ONLY NAND.
// This proves that NAND is functionally complete — it alone can express
// any Boolean function.
//
// Think of it like building with LEGO: NAND is our one brick shape,
// and we can build any structure from it.

// NAND_NOT implements NOT using only NAND gates.
//
// The trick: feed the same input to both sides of a NAND.
//
//	NAND(A, A) = NOT(A AND A) = NOT(A)
//
// Circuit:
//
//	A ---+
//	     |--- NAND --- Output
//	A ---+
//
// This works because A AND A = A, so NOT(A AND A) = NOT(A).
func NAND_NOT(a int) int {
	result, _ := StartNew[int]("logic-gates.NAND_NOT", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			validateBit(a, "a")
			return rf.Generate(true, false, NAND(a, a))
		}).PanicOnUnexpected().GetResult()
	return result
}

// NAND_AND implements AND using only NAND gates.
//
// AND = NOT(NAND). Since we already have NAND_NOT:
//
//	NAND_AND(A, B) = NAND_NOT(NAND(A, B))
//
// Circuit (2 NAND gates):
//
//	A ---+
//	     |--- NAND ---+
//	B ---+            |--- NAND --- Output
//	                  |
//	           (same wire to both inputs)
//
// Gate count: 2 NANDs to make 1 AND. In real hardware, this is
// why AND gates are slightly slower than NAND gates — they require
// an extra inverter stage.
func NAND_AND(a, b int) int {
	result, _ := StartNew[int]("logic-gates.NAND_AND", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			validateBit(a, "a")
			validateBit(b, "b")
			return rf.Generate(true, false, NAND_NOT(NAND(a, b)))
		}).PanicOnUnexpected().GetResult()
	return result
}

// NAND_OR implements OR using only NAND gates.
//
// By De Morgan's law: A OR B = NOT(NOT(A) AND NOT(B))
//                            = NAND(NOT(A), NOT(B))
//                            = NAND(NAND(A,A), NAND(B,B))
//
// Circuit (3 NAND gates):
//
//	A ---+
//	     |--- NAND ---+
//	A ---+            |
//	                  +--- NAND --- Output
//	B ---+            |
//	     |--- NAND ---+
//	B ---+
//
// Gate count: 3 NANDs to make 1 OR.
func NAND_OR(a, b int) int {
	result, _ := StartNew[int]("logic-gates.NAND_OR", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			validateBit(a, "a")
			validateBit(b, "b")
			return rf.Generate(true, false, NAND(NAND(a, a), NAND(b, b)))
		}).PanicOnUnexpected().GetResult()
	return result
}

// NAND_XOR implements XOR using only NAND gates.
//
// XOR(A,B) = A AND NOT(B) OR NOT(A) AND B
//
// Using NAND, a compact construction is:
//
//	Let C = NAND(A, B)
//	XOR  = NAND(NAND(A, C), NAND(B, C))
//
// Circuit (4 NAND gates):
//
//	A ---+--- NAND(A,B) = C ---+
//	     |                     |
//	B ---+                     |
//	                           |
//	A --- NAND(A,C) ---+       |
//	                   |--- NAND --- Output
//	B --- NAND(B,C) ---+
//
// Gate count: 4 NANDs to make 1 XOR. This is the minimum — you
// cannot build XOR from fewer than 4 NAND gates.
func NAND_XOR(a, b int) int {
	result, _ := StartNew[int]("logic-gates.NAND_XOR", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			validateBit(a, "a")
			validateBit(b, "b")
			c := NAND(a, b)
			return rf.Generate(true, false, NAND(NAND(a, c), NAND(b, c)))
		}).PanicOnUnexpected().GetResult()
	return result
}

// =========================================================================
// Multi-Input Gates
// =========================================================================
//
// Real circuits often need to AND or OR more than two signals together.
// For example, a 4-input AND gate checks "are ALL four signals high?"
//
// Multi-input gates are built by chaining two-input gates:
//
//	AND(A, B, C, D) = AND(AND(AND(A, B), C), D)
//
// In hardware, this creates a "gate chain" with increasing propagation
// delay. Wide gates (8+ inputs) are sometimes built as balanced trees
// to minimize delay:
//
//	Tree structure (faster):     Chain structure (simpler):
//
//	A --+                        A --+
//	    AND --+                      AND --+
//	B --+     |                  B --+     |
//	          AND -- Output              AND --+
//	C --+     |                  C ------+     |
//	    AND --+                                AND -- Output
//	D --+                        D ------------+

// ANDn returns 1 only when ALL inputs are 1.
//
// This is the variadic (multi-input) version of AND. It chains
// two-input AND operations across all inputs using left folding:
//
//	ANDn(a, b, c, d) = AND(AND(AND(a, b), c), d)
//
// Panics if fewer than 2 inputs are provided (a gate needs at
// least two inputs to be meaningful).
//
// Real-world use: wide AND gates appear in address decoders
// (does this address match ALL the required bits?) and in
// instruction decode logic.
func ANDn(inputs ...int) int {
	result, _ := StartNew[int]("logic-gates.ANDn", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if len(inputs) < 2 {
				panic("logicgates: ANDn requires at least 2 inputs")
			}
			validateBits(inputs, "inputs")
			res := inputs[0]
			for _, v := range inputs[1:] {
				res = AND(res, v)
			}
			return rf.Generate(true, false, res)
		}).PanicOnUnexpected().GetResult()
	return result
}

// ORn returns 1 when AT LEAST ONE input is 1.
//
// This is the variadic (multi-input) version of OR. It chains
// two-input OR operations across all inputs using left folding:
//
//	ORn(a, b, c, d) = OR(OR(OR(a, b), c), d)
//
// Panics if fewer than 2 inputs are provided.
//
// Real-world use: wide OR gates appear in interrupt controllers
// (did ANY device signal an interrupt?) and bus contention logic.
func ORn(inputs ...int) int {
	result, _ := StartNew[int]("logic-gates.ORn", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if len(inputs) < 2 {
				panic("logicgates: ORn requires at least 2 inputs")
			}
			validateBits(inputs, "inputs")
			res := inputs[0]
			for _, v := range inputs[1:] {
				res = OR(res, v)
			}
			return rf.Generate(true, false, res)
		}).PanicOnUnexpected().GetResult()
	return result
}

// XORn computes the N-input XOR gate — reduces a sequence of bits via XOR.
//
// Returns 1 if an ODD number of inputs are 1 (odd parity).
//
// This is a left-fold over the two-input XOR gate:
//
//	XORn(a, b, c, d) = XOR(XOR(XOR(a, b), c), d)
//
// Key property: XORn is 1 when the count of 1-bits in the inputs is odd.
//
// # Applications
//
// XORn is the building block for parity computation. The Intel 8008 uses
// a Parity flag (P) that is 1 when the result has EVEN parity — an even
// number of 1-bits. In hardware:
//
//	P = NOT(XORn(bit0, bit1, ..., bit7))
//
// Because XORn = 1 means odd number of 1s, NOT(XORn) = 1 means even.
//
// This is physically implemented as a chain of XOR gates on the ALU output:
//
//	XOR(XOR(XOR(XOR(XOR(XOR(XOR(b0, b1), b2), b3), b4), b5), b6), b7)
//
// Then a NOT to invert (even parity = P = 1 on the 8008).
//
// # Usage
//
//	XORn(1, 0, 1, 0) // → 0 (even count of 1s → XOR chain = 0)
//	XORn(1, 1, 1, 0) // → 1 (odd count of 1s  → XOR chain = 1)
//	// For 8008 Parity flag: P = NOT(XORn(resultBits...))
//	// P=1 means even parity; P=0 means odd parity.
//
// Panics if fewer than 2 inputs are provided.
func XORn(inputs ...int) int {
	result, _ := StartNew[int]("logic-gates.XORn", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if len(inputs) < 2 {
				panic("logicgates: XORn requires at least 2 inputs")
			}
			validateBits(inputs, "inputs")
			// Left-fold: chain two-input XOR gates.
			// Each XOR asks "are these two bits different?"
			// After folding all bits, the result is 1 iff an odd number
			// of inputs were 1 — which is exactly the parity bit.
			res := inputs[0]
			for _, v := range inputs[1:] {
				res = XOR(res, v)
			}
			return rf.Generate(true, false, res)
		}).PanicOnUnexpected().GetResult()
	return result
}
