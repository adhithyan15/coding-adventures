# S07 — Real OS Kernel

## Overview

This spec defines the first real OS kernel crate in the repository.

The milestone goal is:

**one shared kernel codebase, cross-compiled by Rust to both ARM and x86_64,
with both variants booting under QEMU**

The important point is not rich functionality yet. The important point is that
the repository gains a real kernel box with a stable shape.

## First Platform Strategy

To keep the kernel abstraction small, the first real kernel uses:

- UEFI as the firmware load contract
- QEMU as the machine
- Rust's built-in UEFI compilation targets

That gives us two immediate targets from the same kernel code:

- `aarch64-unknown-uefi`
- `x86_64-unknown-uefi`

## Why This Shape

This milestone is about the kernel box, not the bootloader box.

UEFI gives us a standard handoff path so we can focus on:

- the kernel crate
- the kernel lifecycle
- cross-target compilation
- shared code across architectures

without first inventing:

- a custom bootloader
- a custom firmware contract
- a custom machine loader

## Kernel Abstraction

The first `os-kernel` crate is intentionally tiny.

It defines:

- `KernelState`
  - `Constructed`
  - `Booted`
  - `Running`
- `Kernel`
  - `new()`
  - `boot()`
  - `enter_running_state()`

This gives us a real abstraction that later work can extend with:

- memory initialization
- traps
- allocators
- drivers
- process support

## Cross-Compilation Requirement

The same `os-kernel` Rust code should compile into:

- an ARM UEFI image
- an x86_64 UEFI image

Architecture-specific differences are handled by:

- Rust targets
- the UEFI ABI
- QEMU machine and firmware selection

The kernel logic itself should remain shared.

## Relationship to the Existing Simulator

The previous `os-kernel` package is simulator-oriented and remains useful.

To keep names honest:

- the simulator package becomes `os-kernel-simulator`
- the new real kernel package becomes `os-kernel`

## Validation

This milestone is complete when:

1. `os-kernel` builds for `aarch64-unknown-uefi`
2. `os-kernel` builds for `x86_64-unknown-uefi`
3. the ARM image boots under QEMU
4. the x86_64 image boots under QEMU
5. both reach the kernel running state

## Next Milestone

After this base kernel box is stable:

1. add richer visible output
2. formalize architecture-neutral kernel services
3. decide what firmware-independent boot contract should look like
4. only then revisit custom bootloader work
