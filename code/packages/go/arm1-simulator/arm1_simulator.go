// Package arm1simulator provides a behavioral simulator for the ARM1
// processor — the first ARM chip, designed by Sophie Wilson and Steve Furber
// at Acorn Computers in 1984-1985.
//
// The ARM1 was a 32-bit RISC processor with just 25,000 transistors. It
// famously worked correctly on its very first power-on (April 26, 1985).
// Its accidental low power consumption (~0.1W) later made the ARM architecture
// dominant in mobile computing, with over 250 billion chips shipped.
//
// This package implements the complete ARMv1 instruction set:
//   - 16 data processing operations (AND, EOR, SUB, RSB, ADD, ADC, SBC, RSC,
//     TST, TEQ, CMP, CMN, ORR, MOV, BIC, MVN)
//   - Load/store (LDR, STR, LDRB, STRB with pre/post-indexed addressing)
//   - Block transfer (LDM, STM with all four stacking modes)
//   - Branch (B, BL)
//   - Software interrupt (SWI)
//   - Conditional execution on every instruction (16 condition codes)
//   - Inline barrel shifter (LSL, LSR, ASR, ROR, RRX)
//   - 4 processor modes with banked registers (USR, FIQ, IRQ, SVC)
//
// # Usage
//
//   cpu := arm1simulator.New(1024 * 1024) // 1 MiB memory
//   cpu.LoadProgram(machineCode, 0)
//   traces := cpu.Run(10000)
//
// For the gate-level version that routes every operation through logic gates,
// see the arm1-gatelevel package.
package arm1simulator
