# sim-discrete-arcade-logic

Gate-level netlist simulator for 7400-series TTL ICs, master clock divider raster sync generator, and paddle potentiometer ADC (`GAME15a`).

## Overview

In the earliest era of video arcade games (1971–1975, beginning with Nolan Bushnell and Ted Dabney's *Computer Space* and Allan Alcorn's *Pong*), games contained no microprocessors, no RAM, and no framebuffers. Instead, games were constructed entirely out of discrete Transistor-Transistor Logic (TTL) 7400-series integrated circuits.

`sim-discrete-arcade-logic` provides bit-accurate, zero-heap-allocation simulation of these discrete circuits, supporting both:
1. **High-performance functional models** for cycle-accurate emulation.
2. **Gate-level netlist models** built on primitive logic gates via the `logic-gates` crate, verifying exact gate-level equivalence.

## Features

- **7400-Series TTL Integrated Circuits:**
  - `Ttl7400QuadNand`: 4x 2-input NAND gates.
  - `Ttl7402QuadNor`: 4x 2-input NOR gates.
  - `Ttl7474DualDFlipFlop`: dual edge-triggered D flip-flop with active-low async preset ($\overline{PRE}$) and clear ($\overline{CLR}$).
  - `Ttl7490DecadeCounter`: BCD decade counter (divide-by-2 and divide-by-5) with 0-reset and 9-set inputs.
  - `Ttl7493BinaryCounter`: 4-bit ripple binary counter (divide-by-2 and divide-by-8) with reset inputs.
  - `Ttl74153DualMux4to1`: dual 4-to-1 data multiplexer with active-low strobes ($\overline{1G}, \overline{2G}$).
- **`ArcadeSyncGenerator`:**
  - Historical arcade master clock divider chain from a 14.31818 MHz crystal oscillator.
  - 7.15909 MHz pixel dot clock (divide-by-2).
  - 455 pixel clocks per scanline ($\approx 15.734$ kHz horizontal line rate).
  - 262 scanlines per frame ($\approx 60.05$ Hz vertical frame rate).
  - Emits HSync, VSync, HBlank, VBlank, and `(dot_x, scanline_y)` raster coordinates.
- **`PaddlePotAdc`:**
  - Deterministic fixed-point R-C ramp charging comparator (555-timer model).
  - Maps potentiometer input $[0, 65535]$ to cycle-accurate trigger pulses when charging past $2/3 V_{cc}$.
- **Zero Allocations & Pure Capabilities:**
  - Fully `#![forbid(unsafe_code)]` with zero dynamic allocations (`required_capabilities.json: {"capabilities": []}`).
  - Defense in depth for CWE-20, CWE-369, and CWE-190.
