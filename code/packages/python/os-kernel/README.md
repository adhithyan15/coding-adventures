# coding-adventures-os-kernel

S04 OS Kernel -- minimal monolithic kernel with process management, scheduler, and syscalls.

## Overview

The OS kernel provides process creation, a round-robin scheduler, and a small set of
system calls. It runs on top of the RISC-V simulator and is loaded into memory by
the bootloader.

## Installation

```bash
pip install -e ".[dev]"
```

## Testing

```bash
pytest
```

## Where It Fits

This package sits at layer S04 of the coding-adventures stack. The boot sequence is:

    ROM-BIOS -> Bootloader -> **OS Kernel** -> User programs
