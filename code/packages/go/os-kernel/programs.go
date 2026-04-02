package oskernel

import (
	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

// =========================================================================
// RISC-V Program Generators
// =========================================================================
//
// These functions generate small RISC-V programs as raw machine code bytes.
// They are used by the kernel to create the idle and hello-world processes.

// GenerateIdleProgram creates the idle process binary.
//
// The idle process is an infinite loop that calls sys_yield:
//
//	loop:
//	    li  a7, 3       # a7 = 3 (sys_yield)
//	    ecall           # trap to kernel
//	    jal x0, loop    # loop forever
//
// This keeps the CPU busy when no real work exists. When the scheduler
// has no other process to run, it runs idle.
func GenerateIdleProgram() []byte {
	result, _ := StartNew[[]byte]("os-kernel.GenerateIdleProgram", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			instructions := []uint32{
				riscv.EncodeAddi(RegA7, 0, SysYield), // li a7, 3
				riscv.EncodeEcall(),                   // ecall
				riscv.EncodeJal(0, -8),               // jal x0, -8 (back to li)
			}
			return rf.Generate(true, false, riscv.Assemble(instructions))
		}).GetResult()
	return result
}

// GenerateHelloWorldProgram creates the hello-world process binary.
//
// The program stores "Hello World\n" in its data section, then:
//   1. Loads the address of the string data
//   2. Calls sys_write(fd=1, buf=&data, len=12)
//   3. Calls sys_exit(0)
//
// Memory layout within the process region (e.g., base = 0x00040000):
//
//	+0x000: code instructions
//	+0x100: "Hello World\n" (12 bytes of string data)
//
// The data offset of 0x100 (256 bytes) gives room for the code section.
func GenerateHelloWorldProgram(memBase uint32) []byte {
	result, _ := StartNew[[]byte]("os-kernel.GenerateHelloWorldProgram", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			dataOffset := uint32(0x100) // String data lives at memBase + 0x100
			dataAddr := memBase + dataOffset
			message := []byte("Hello World\n")

			// Build the instruction sequence.
			//
			// We need to load dataAddr into a1. Use LUI + ADDI for the full 32-bit address.
			var instructions []uint32

			// Load data address into a1 (x11).
			upper := dataAddr >> 12
			lower := dataAddr & 0xFFF
			if lower >= 0x800 {
				upper++ // Compensate for sign extension
			}

			instructions = append(instructions, riscv.EncodeLui(RegA1, int(upper))) // lui a1, upper
			if lower != 0 {
				signedLower := int(lower)
				if signedLower >= 0x800 {
					signedLower -= 0x1000
				}
				instructions = append(instructions, riscv.EncodeAddi(RegA1, RegA1, signedLower)) // addi a1, a1, lower
			}

			// a0 = 1 (stdout file descriptor)
			instructions = append(instructions, riscv.EncodeAddi(RegA0, 0, 1)) // li a0, 1

			// a2 = 12 (length of "Hello World\n")
			instructions = append(instructions, riscv.EncodeAddi(RegA2, 0, len(message))) // li a2, 12

			// a7 = 1 (sys_write)
			instructions = append(instructions, riscv.EncodeAddi(RegA7, 0, SysWrite)) // li a7, 1

			// ecall -- triggers syscall
			instructions = append(instructions, riscv.EncodeEcall()) // ecall

			// a0 = 0 (exit code)
			instructions = append(instructions, riscv.EncodeAddi(RegA0, 0, 0)) // li a0, 0

			// a7 = 0 (sys_exit)
			instructions = append(instructions, riscv.EncodeAddi(RegA7, 0, SysExit)) // li a7, 0

			// ecall -- triggers syscall
			instructions = append(instructions, riscv.EncodeEcall()) // ecall

			// Assemble code section.
			code := riscv.Assemble(instructions)

			// Build the full binary: code + padding + data.
			bin := make([]byte, int(dataOffset)+len(message))
			copy(bin, code)
			copy(bin[dataOffset:], message)

			return rf.Generate(true, false, bin)
		}).GetResult()
	return result
}

// GenerateHelloWorldBinary is an alias for GenerateHelloWorldProgram using
// the default user process base address (0x00040000).
func GenerateHelloWorldBinary() []byte {
	result, _ := StartNew[[]byte]("os-kernel.GenerateHelloWorldBinary", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			return rf.Generate(true, false, GenerateHelloWorldProgram(DefaultUserProcessBase))
		}).GetResult()
	return result
}
