# System Board (S06)

The top-level integration package -- the actual simulated computer. It composes ROM/BIOS (S01), Bootloader (S02), Interrupt Handler (S03), OS Kernel (S04), Display (S05), and a RISC-V CPU into a complete system that boots to "Hello World."

## The Boot Sequence

```
PowerOn -> BIOS -> Bootloader -> KernelInit -> UserProgram -> Idle
```

1. **PowerOn**: Components instantiated, memory initialized
2. **BIOS**: Hardware info written to boot protocol (simulated in Go)
3. **Bootloader**: Real RISC-V code copies kernel from disk to RAM
4. **KernelInit**: Kernel creates processes, registers ISRs, starts scheduler
5. **UserProgram**: Hello-world runs, calls sys_write, calls sys_exit
6. **Idle**: Only idle process remains, system is done

## Usage

```go
board := systemboard.NewSystemBoard(systemboard.DefaultSystemConfig())
board.PowerOn()
trace := board.Run(100000)

snap := board.DisplaySnapshot()
fmt.Println(snap.Contains("Hello World"))  // true

fmt.Println(board.IsIdle())  // true
```

## Architecture

The SystemBoard uses a RISC-V simulator for instruction execution. The BIOS phase is simulated in Go (pre-loading boot protocol data), while the bootloader runs as real RISC-V machine code. The kernel operates at the Go level -- ecall traps are intercepted and dispatched to Go syscall handlers.

## Dependencies

- `rom-bios` (S01) -- BIOS firmware and ROM
- `bootloader` (S02) -- kernel loading code
- `interrupt-handler` (S03) -- ISR registry and interrupt controller
- `os-kernel` (S04) -- process management, scheduling, syscalls
- `display` (S05) -- text framebuffer
- `riscv-simulator` -- RISC-V CPU simulation
- `cpu-simulator` -- memory and register abstractions

## Test Coverage

88%+ coverage including the critical boot-to-Hello-World integration test.
