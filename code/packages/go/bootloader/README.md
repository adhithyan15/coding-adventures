# Bootloader (S02)

The bootloader is the second stage of the boot sequence, running after the BIOS (S01). It lives at address `0x00010000` and generates real RISC-V machine code that:

1. **Validates** the boot protocol magic number (`0xB007CAFE`) at `0x00001000`
2. **Copies** the kernel binary from the memory-mapped disk region to kernel RAM at `0x00020000`
3. **Sets** the stack pointer (`sp/x2`) to `0x0006FFF0`
4. **Jumps** to the kernel entry point at `0x00020000`

## How It Fits in the Stack

```
Power On
  -> ROM/BIOS (S01): hardware init, IDT, boot protocol
  -> Bootloader (S02): copy kernel, set stack, jump    <-- THIS PACKAGE
  -> OS Kernel (S04): processes, scheduling, syscalls
  -> User Programs: Hello World
```

## Usage

```go
config := bootloader.DefaultBootloaderConfig()
config.KernelSize = 4096
bl := bootloader.NewBootloader(config)

// Get raw machine code bytes
code := bl.Generate()

// Get annotated instructions for debugging
annotated := bl.GenerateWithComments()
for _, inst := range annotated {
    fmt.Printf("0x%08X: %s  ; %s\n", inst.Address, inst.Assembly, inst.Comment)
}
```

## Disk Image

The `DiskImage` type simulates persistent storage:

```go
disk := bootloader.NewDiskImage(2 * 1024 * 1024)
disk.LoadKernel(kernelBinary)
disk.LoadUserProgram(helloBinary, 0x00100000)
```

## Dependencies

- `riscv-simulator` -- encoding helpers for generating RISC-V instructions

## Test Coverage

97%+ coverage including execution tests on the simulated CPU.
