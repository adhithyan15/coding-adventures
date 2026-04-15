# S07 — Real RISC-V Boot on QEMU

## Overview

The existing S-series proves the boot chain inside our simulated machine:
ROM/BIOS, bootloader, kernel, display, and a full boot-to-hello-world trace.
This spec starts the next phase: taking that understanding and making it boot
for real on QEMU.

The first real target is **RISC-V on QEMU's `virt` machine**, with a minimal
goal:

**Boot a custom Rust kernel and print `Hello World` over the emulated UART.**

This is not yet a full operating system. It is the smallest real bootable
artifact that establishes a true hardware contract:

- a real entry point
- a real bootloader
- a real kernel binary
- a real linker/memory layout
- a real emulator target
- a real output device

The design is intentionally staged. We want the first milestone to be real,
simple, and debuggable, while leaving a clean path toward fully custom
firmware, multiple architectures, and eventually a more capable OS.

## Why RISC-V First

RISC-V is the right first real target for this repository because:

- it matches the simulator track already built in this repo
- it avoids much of x86's historical boot complexity
- QEMU supports it well and provides a clean virtual platform
- the privilege model is understandable without legacy mode transitions
- it is a good foundation for an eventual cross-architecture OS strategy

The first concrete platform is:

- architecture: `riscv64`
- emulator: `qemu-system-riscv64`
- board: `virt`
- console: NS16550-compatible UART
- execution model: single hart, polling console, no MMU in v0

## Goals

- Boot a Rust `no_std` kernel on QEMU `virt`.
- Use a custom bootloader written in Rust.
- Print `Hello World` to the serial console.
- Keep the first kernel physically addressed and simple.
- Define a boot contract that can later be implemented on ARM and x86.
- Separate architecture-independent kernel logic from architecture/platform glue.

## Non-Goals

- Multi-user support
- Filesystem support
- Virtual memory or paging in v0
- Process scheduling in v0
- Interrupt-driven drivers in v0
- SMP or multi-hart support in v0
- VirtualBox support in the first milestone

## Boot Philosophy

We want to own the bootloader, but we do not need to own every earlier stage on
day one.

That leads to a two-layer strategy:

### Milestone A: Custom bootloader, borrowed machine firmware

Boot flow:

`QEMU reset -> OpenSBI (provided by QEMU/default firmware path) -> our bootloader -> our kernel`

This still gives us:

- a real bootloader that we own
- a real kernel that we own
- a real handoff contract
- a real QEMU boot path

It avoids immediately having to write M-mode firmware and SBI services before
we even have a working `Hello World`.

### Milestone B: Fully custom firmware path

Boot flow:

`QEMU reset -> our M-mode stage0/stage1 firmware -> our kernel`

This removes the borrowed firmware layer and gives us a truly from-scratch
boot chain. Milestone B should happen only after Milestone A is stable.

## System Decomposition

The real RISC-V boot path is split into five pieces.

### 1. Boot protocol crate

A small architecture-aware but OS-neutral contract shared by the bootloader and
kernel.

Responsibilities:

- define `BootInfo`
- define versioning and magic fields
- define memory-region descriptors
- define console and device-tree handoff fields

### 2. UART driver crate

Minimal output driver for the QEMU `virt` serial device.

Responsibilities:

- blocking transmit
- optional receive later
- no interrupts in v0
- formatting support for debug prints

### 3. RISC-V platform runtime crate

Lowest-level architecture code for:

- entry assembly
- stack setup
- zeroing `.bss`
- linker symbols
- jump conventions
- trap stub placeholders

This layer should be tiny and mechanical.

### 4. Bootloader crate

Owns loading and kernel handoff.

Responsibilities in v0:

- start from the firmware-provided entry point
- initialize a boot stack
- initialize UART for debug output
- locate the kernel image
- copy the kernel to its physical load address
- build a `BootInfo` struct
- jump to the kernel entry point

### 5. Kernel crate

Owns the operating-system-facing side.

Responsibilities in v0:

- accept the boot contract
- initialize the console
- print `Hello World`
- spin forever

## Milestone Sequence

### M0: Real kernel only

Goal:

- Build a Rust `no_std` kernel that can print to the QEMU UART when entered.

Notes:

- handoff may be done by a tiny temporary shim
- this milestone validates linker script, runtime, panic handling, and console

### M1: Custom bootloader under OpenSBI

Goal:

- Boot our own bootloader on QEMU `virt`
- bootloader loads our kernel
- kernel prints `Hello World`

This is the first milestone this spec targets as the default implementation
path.

### M2: Separate kernel artifact loading

Goal:

- stop embedding the kernel bytes directly into the bootloader
- load a separate kernel image artifact

Possible mechanisms:

- appended payload
- flash image
- virtio block
- fw_cfg

### M3: Fully custom M-mode boot path

Goal:

- replace the borrowed firmware layer
- start from QEMU reset with our own early-stage firmware

### M4: Supervisor kernel services

Goal:

- traps
- timer
- basic allocator
- physical memory map parsing
- eventually process and syscall support

## Initial Recommended Design

For the first real bring-up, simplicity wins.

### Bootloader format

The bootloader is a Rust `no_std` binary with a tiny assembly entry stub.

For M1, it is loaded as the payload handed to us by QEMU's normal RISC-V boot
path. The bootloader runs in the privilege level and calling convention exposed
by that environment.

### Kernel format

Use a **flat binary at a fixed physical load address** for v0/v1.

Reason:

- much simpler than parsing ELF immediately
- keeps early bring-up focused
- easy to inspect in objdump/hexdump

Later milestone:

- move to ELF loading once the boot path is stable

### Kernel transport

For M1, the kernel binary may be linked into the bootloader as a byte blob.

Reason:

- avoids file-system and block-driver work
- avoids flash layout complexity in the first real milestone
- still exercises a real bootloader-to-kernel handoff

Later milestone:

- switch to separate on-disk or flash-backed artifacts

## Boot Contract

The bootloader and kernel must agree on one stable ABI.

### Kernel entry

The kernel entry symbol is:

- `kernel_main`

The initial calling convention for RISC-V is:

- `a0`: hart ID
- `a1`: device tree blob pointer if available, otherwise `0`
- `a2`: pointer to `BootInfo`
- `sp`: boot stack top prepared by the bootloader

### BootInfo v0

`BootInfo` should contain:

- magic
- version
- boot hart ID
- physical memory base
- physical memory size
- UART base address
- kernel physical start
- kernel physical end
- DTB pointer
- reserved fields for future expansion

Rules:

- bootloader owns populating the struct
- kernel must validate `magic` and `version`
- unknown future fields must be ignored if the version is still compatible

## QEMU `virt` Assumptions for v0

The QEMU `virt` platform is treated as a known bring-up environment.

Expected platform assumptions in v0:

- RAM starts at `0x8000_0000`
- a serial UART is available
- a device tree blob is available from firmware/QEMU handoff
- only hart 0 is used

Hardcoded assumptions are acceptable in v0 for:

- UART base address
- single-hart boot
- fixed kernel load address

But the design should keep a path open to:

- reading device addresses from the DTB
- supporting multiple harts
- moving from fixed physical assumptions to discovered platform data

## Initial Memory Layout

These addresses are policy, not a permanent ABI.

Suggested v0 layout:

```text
0x8000_0000  Bootloader image base / firmware-loaded payload region
0x8001_0000  Bootloader stack
0x8020_0000  Kernel load address
0x8021_0000  BootInfo + scratch data
0x1000_0000  UART MMIO base (QEMU virt expectation for v0)
```

Rules:

- bootloader and kernel must not overlap
- kernel image size must be asserted at build time
- stack ranges must be explicit in the linker script
- all addresses must live in one shared memory-layout module

## Bootloader Responsibilities in Detail

The bootloader is intentionally simple, but still real.

### Required in v0

- define the entry point
- set up a valid stack
- zero its own `.bss`
- initialize UART
- print boot diagnostics
- copy kernel bytes to the kernel load address
- flush instruction visibility if required by the architecture/runtime
- prepare `BootInfo`
- jump to kernel entry

### Not required in v0

- parsing a filesystem
- generic ELF parsing
- paging setup
- interrupt controller setup
- SBI implementation
- dynamic memory allocation

### Recommended debug messages

- `bootloader: start`
- `bootloader: uart ok`
- `bootloader: kernel copied`
- `bootloader: jumping to kernel`

These messages are part of the bring-up strategy, not noise.

## Kernel Responsibilities in Detail

The first kernel is deliberately tiny.

### Required in v0

- define a freestanding entry point
- set up `.bss` if the runtime layer does not do it earlier
- initialize console output
- validate `BootInfo`
- print `Hello World`
- enter a spin loop

### Recommended debug messages

- `kernel: start`
- `kernel: boot info ok`
- `Hello World`

### Panic behavior

The panic handler should:

- print a panic prefix to UART
- print message/location if available
- spin forever

## Build Artifacts

The build should produce explicit, inspectable outputs.

### Required artifacts

- bootloader ELF
- bootloader disassembly
- kernel ELF
- kernel flat binary
- kernel disassembly

### Later artifacts

- combined flash image
- payload-appended boot image
- DTB dump for inspection

## Testing Strategy

Testing should happen at three levels.

### Unit tests

For:

- `BootInfo` serialization/layout
- UART register helpers
- linker-symbol helpers where possible

### Host-side artifact tests

For:

- symbol presence
- section boundaries
- image size limits
- fixed load-address assertions

### QEMU integration tests

For:

- boot completes without trap/panic
- serial output contains `Hello World`
- bootloader messages appear in order

The first automated success condition is:

`QEMU exits or times out after producing serial output containing Hello World`

## Cross-Architecture Strategy

The long-term goal is not "rewrite the OS from scratch for every ISA."
The goal is:

- one kernel design
- one boot contract family
- one architecture-neutral core
- thin architecture/platform adapters

That means the code should separate into:

### Architecture-neutral core

- boot info parsing
- formatting/logging facade
- kernel initialization sequence
- future scheduler, allocator, and drivers behind traits

### Architecture-specific runtime

- entry assembly
- register conventions
- linker scripts
- trap stubs
- cache/TLB fences when needed

### Platform-specific layer

- QEMU `virt`
- real boards later
- UART addresses
- interrupt-controller addresses
- timer wiring

For ARM later, the same kernel core should survive while only the runtime,
linker, and platform layer change.

## Package Direction

The current simulated packages should remain intact. The real boot track should
be introduced as a parallel line, not a replacement.

Suggested package split:

- `boot-protocol`
- `uart-16550`
- `riscv-rt`
- `riscv-qemu-bootloader`
- `riscv-qemu-kernel`

This keeps the real target separate from the existing simulated:

- `bootloader`
- `rom-bios`
- `os-kernel`
- `system-board`

## First Definition of Done

This spec's first meaningful completion point is:

1. `cargo build` produces a bootloader and kernel artifact
2. `qemu-system-riscv64` boots the image on `virt`
3. serial output shows bootloader diagnostics
4. serial output shows `Hello World`
5. the system spins without crashing

That is enough to say:

**we have a real OS boot path**

even though the system still has no scheduler, no virtual memory, and no user
program support yet.

## Recommended Immediate Next Step

Implement Milestone M0 and M1 in this order:

1. build the UART driver and RISC-V runtime glue
2. bring up a minimal `no_std` kernel that prints on UART
3. add the custom bootloader
4. embed the kernel binary into the bootloader for the first handoff
5. boot it under QEMU `virt`

That sequence gives the fastest path to a real, visible result while preserving
the long-term from-scratch plan.
