# Changelog

## 0.1.0 — 2026-03-21

### Added
- `Kernel` type with `Boot()`, `HandleSyscall()`, `HandleTimer()`, `HandleKeyboard()`, `IsIdle()`
- `KernelConfig` with timer interval, max processes, and memory layout
- `ProcessControlBlock` with state machine (Ready, Running, Blocked, Terminated)
- `Scheduler` with round-robin algorithm and `ContextSwitch()`
- `MemoryManager` with region-based allocation, `FindRegion()`, `CheckAccess()`
- Syscall dispatch table: sys_exit (0), sys_write (1), sys_read (2), sys_yield (3)
- `RegisterAccess` and `MemoryAccess` interfaces for CPU decoupling
- `GenerateIdleProgram()` -- RISC-V machine code for the idle loop
- `GenerateHelloWorldProgram()` -- RISC-V machine code that prints "Hello World\n"
- `AddKeystroke()` for keyboard buffer management
- Comprehensive test suite with 90%+ coverage
