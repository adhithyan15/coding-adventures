# coding-adventures-bootloader

S02 Bootloader -- generates RISC-V machine code to load the OS kernel from disk into RAM.

## Overview

The bootloader is the first software that runs when the simulated computer powers on.
It produces a small RISC-V program that copies the kernel image from a disk device
into main memory and then jumps to the kernel entry point.

## Installation

```bash
pip install -e ".[dev]"
```

## Testing

```bash
pytest
```

## Where It Fits

This package sits at layer S02 of the coding-adventures stack, between the
ROM/BIOS firmware (S01) and the OS kernel (S04). The boot sequence is:

    ROM-BIOS -> **Bootloader** -> OS Kernel
