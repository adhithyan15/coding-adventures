// Package arm1gatelevel provides a gate-level simulator for the ARM1 processor.
//
// Every arithmetic operation routes through actual logic gate functions — AND,
// OR, XOR, NOT — chained into adders, then into a 32-bit ALU. The barrel
// shifter is built from multiplexer trees. Registers are built from D flip-flops.
//
// This is NOT the same as the behavioral simulator (arm1-simulator package).
// Both produce identical results for any program. The difference is the
// execution path:
//
//	Behavioral:  opcode → match statement → host arithmetic → result
//	Gate-level:  opcode → decoder gates → barrel shifter muxes → ALU gates → adder gates → logic gates → result
//
// # Architecture
//
// The gate-level simulator composes packages from layers below:
//   - logic-gates: AND, OR, XOR, NOT, MUX, D flip-flop, register
//   - arithmetic: half_adder, full_adder, ripple_carry_adder, ALU
//   - arm1-simulator: types, condition codes, instruction encoding helpers
package arm1gatelevel
