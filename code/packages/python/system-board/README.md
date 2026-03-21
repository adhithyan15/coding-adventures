# coding-adventures-system-board

S06 System Board -- complete simulated computer from power-on to Hello World.

## Overview

The system board wires every component together: CPU core, memory, ROM/BIOS,
bootloader, display, interrupt handler, and OS kernel. It models a full
power-on sequence ending with a user-visible "Hello World" on the display.

## Installation

```bash
pip install -e ".[dev]"
```

## Testing

```bash
pytest
```

## Where It Fits

This is the top-level integration package (layer S06) of the coding-adventures
computer stack. It composes all lower layers into a working simulated machine.
