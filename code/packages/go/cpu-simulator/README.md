# CPU Simulator (Go Port)

**Layer 3 of the computing stack** — models generic CPU execution without tying it to a specific ISA (Instruction Set Architecture).

## How does it work?

A CPU is a dumb but incredibly fast state machine: it reads numbers from memory (instructions), figures out what those numbers mean (decoding), uses the ALU to do math or move data (executing), and repeats forever.

This package provides the generic "shell" for this:
1. **Memory**: An array of bytes holding instructions and data.
2. **Registers**: Tiny, fast storage variables inside the CPU.
3. **Pipeline (Fetch-Decode-Execute)**: The three-stage loop that drives computation.
4. **Decoder & Executor Interfaces**: The CPU delegates the actual meaning of instructions to an ISA simulator (like `arm-simulator` or `riscv-simulator`), making this core extremely reusable.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator"

// 1. Create decoder and executor (defined by the ISA)
decoder := &MyDecoder{}
executor := &MyExecutor{}

// 2. Initialize a 32-bit CPU with 16 registers and 64KB memory
cpu := cpusimulator.NewCPU(decoder, executor, 16, 32, 65536)

// 3. Load a machine code program into memory
program := []byte{0x93, 0x00, 0x10, 0x00} // addi x1, x0, 1
cpu.LoadProgram(program, 0)

// 4. Run the program (fetches, decodes, and executes)
traces := cpu.Run(100)

for _, trace := range traces {
    fmt.Println(trace.FormatPipeline())
}
```
