# OS Kernel (S04)

A minimal monolithic kernel that manages two processes (idle and hello-world), handles system calls, and drives a round-robin scheduler via timer interrupts.

## How It Fits in the Stack

```
SystemBoard (S06) -- top-level integration
  -> Core (D05) -- executes RISC-V instructions
  -> Interrupt Handler (S03) -- delivers events to kernel
  -> OS Kernel (S04)         <-- THIS PACKAGE
       -> Process Table (idle PID 0, hello-world PID 1)
       -> Scheduler (round-robin via timer interrupts)
       -> Memory Manager (region-based allocation)
       -> Syscall Handler (sys_exit, sys_write, sys_read, sys_yield)
  -> Display (S05) -- framebuffer for sys_write output
```

## Design

The kernel operates at the Go level -- it intercepts ecall traps and handles them in Go code. The hello-world and idle programs are real RISC-V machine code. This is a pragmatic simplification that demonstrates the full OS concept.

## System Calls

| Number | Name      | Args                         | Description                    |
|--------|-----------|------------------------------|--------------------------------|
| 0      | sys_exit  | a0 = exit code               | Terminate current process      |
| 1      | sys_write | a0=fd, a1=buf, a2=len        | Write to display (fd=1)        |
| 2      | sys_read  | a0=fd, a1=buf, a2=maxlen     | Read from keyboard (fd=0)      |
| 3      | sys_yield | (none)                       | Voluntarily give up CPU        |

## Usage

```go
ic := interrupthandler.NewInterruptController()
driver := display.NewDisplayDriver(display.DefaultDisplayConfig(), displayMem)
kernel := oskernel.NewKernel(oskernel.DefaultKernelConfig(), ic, driver)
kernel.Boot()
```

## Dependencies

- `interrupt-handler` (S03) -- ISR registration and dispatch
- `display` (S05) -- framebuffer for sys_write
- `riscv-simulator` -- encoding helpers for program generation

## Test Coverage

90%+ coverage across process management, scheduling, syscalls, memory manager, and program generation.
