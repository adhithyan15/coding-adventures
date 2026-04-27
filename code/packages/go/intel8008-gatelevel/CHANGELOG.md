# Changelog

## [0.1.0] - 2026-04-12

### Added

- Implemented complete Intel 8008 gate-level simulator (`Intel8008GateLevel` type)
- `bits.go`: `IntToBits(value, width)`, `BitsToInt(bits)`, `ComputeParity(value)` using 7-gate XOR reduction tree + NOT gate
- `alu.go`: 8-bit `GateALU` wrapping `arithmetic.ALU(8)`:
  - `Add(a, b, carryIn)` — routes through 8 full adders (40 gates)
  - `Subtract(a, b, borrowIn)` — two's complement via 8 NOT gates + adder
  - `BitwiseAnd`, `BitwiseOr`, `BitwiseXor` — via arithmetic package AND/OR/XOR operations
  - `Increment(a)`, `Decrement(a)` — via adder chain
  - `RotateLeftCircular`, `RotateRightCircular` — RLC/RRC (bit rewiring, no adder)
  - `RotateLeftThroughCarry`, `RotateRightThroughCarry` — RAL/RAR (9-bit rotation)
  - `ComputeFlags(result, carry)` — zero via 8-input NOR, sign = bit 7 direct, parity via XOR tree
- `registers.go`: `RegisterFile` (7 × 8-bit D flip-flop registers) and `FlagRegister` (4-bit):
  - `HLAddress()` computes 14-bit M pseudo-register address via AND gate masking
  - Explicit panic for reg index 6 (M pseudo-register)
- `stack.go`: 8-level 14-bit `PushDownStack` where entry 0 is always the PC:
  - `Push(target)` — rotates stack down, saves return address, sets entry[0]=target
  - `Pop()` — rotates stack up, restores return address to entry[0]
  - `SetPC(addr)` — direct write to entry[0] (for JMP, no stack rotation)
  - `Increment(n)` — 14-bit incrementer via chain of XOR/AND half-adders, wraps at 0x3FFF
  - `ReadLevel(n)` — inspect any stack level for external use
- `decoder.go`: Combinational `Decode(opcode)` function using AND/OR/NOT gate logic:
  - `DecodedInstruction` struct with all control signals
  - Correct HLT detection for all 3 encodings (0x00, 0x76, 0xFF)
  - Group 01 disambiguation: JMP(0x7C), CAL(0x7E), Jcond(sss=000), Ccond(sss=010), IN(sss=001), MOV(else)
  - Group 00 disambiguation: rotates vs OUT vs MVI vs INR vs DCR vs Rcond
  - Group 10: 8-way ALU op decode via DDD field
  - Group 11: RST(sss=101), ALUimm(sss=100), RET(0xC7), HLT(0xFF)
  - ALU op constants: `ALUOpADD`, `ALUOpADC`, `ALUOpSUB`, `ALUOpSBB`, `ALUOpANA`, `ALUOpXRA`, `ALUOpORA`, `ALUOpCMP`
  - Condition codes: `CondFC`, `CondFZ`, `CondFS`, `CondFP`, `CondTC`, `CondTZ`, `CondTS`, `CondTP`
- `cpu.go`: `Intel8008GateLevel` CPU wiring all components:
  - `Run(program, maxSteps)` — calls `Reset()` then loops `Step()`
  - `Step()` — fetch (1 byte), decode, fetch2/fetch3, execute through gate components
  - All instructions: HLT, MOV, MVI, INR, DCR, ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP, ADI/SUI/ANI/XRI/ORI/CPI, RLC/RRC/RAL/RAR, OUT, IN, JMP, CAL, Jcond, Ccond, Rcond, RET, RST
  - `GateCount()` — estimated total gate count (~1118)
  - `SetInputPort(port, value)` / `GetOutputPort(port)` for I/O simulation
  - Note: `Reset()` preserves `inputPorts` (external hardware state)
- `gen_capabilities.go`: Operation/StartNew framework (copied from intel4004-gatelevel)
- 89.7% test coverage across 70+ test cases:
  - Unit tests for all ALU operations, bit helpers, register file, flag register, push-down stack
  - Decoder tests for every instruction family
  - CPU integration tests: HLT, MVI all registers, ADD 1+2, SUB, INR/DCR, MOV, memory access via M, JMP, CAL/RET, RST, conditional jumps, IN/OUT, RLC, RAL, ANA, XRA, CPI, gate count, multiply 4×5 example
