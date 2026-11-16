# sim-crt-vector-beam

Pure, deterministic, zero-allocation dual CRT beam simulator for Rust: raster deflection with phosphor persistence decay and oscilloscope / vector beam display stroke synthesis.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME15b`, `GAME00` §3.3).

---

## Overview

In the 1960s and 1970s, cathode-ray tube (CRT) displays formed the visual foundation of video games and computer graphics across two distinct deflection architectures:

1. **Raster CRT Displays (e.g. *Pong*, arcade cabinets, NTSC/PAL television):** An electron beam continuously sweeps left-to-right, top-to-bottom across the screen in fixed timing cycles comprising visible lines, horizontal blanking (HBlank), front porch, horizontal sync (HSync), back porch, and vertical blanking (VBlank). The luminescence of each scanline pixel depends on phosphor excitation and exponential persistence decay.
2. **Vector CRT / Oscilloscope Displays (e.g. DEC Type 30 on PDP-1 for *Spacewar!*, Atari Vector Generator for *Asteroids*):** Rather than scanning in a raster grid, magnetic or electrostatic deflection amplifiers steer the beam directly along arbitrary 2D vectors. Physical amplifier coils enforce a maximum slewing rate, Z-axis modulation controls beam blanking, and dwelling at a single coordinate deposits energy to produce bright spots.

`sim-crt-vector-beam` models both display architectures with deterministic, zero-heap-allocation simulation primitives on all stepping hot paths.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. Contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **`CAP002` (Zero Unsafe / Safe Rust):** Compiled with `#![forbid(unsafe_code)]`. Prohibits raw pointers, manual memory allocation, and FFI reachability.
- **Zero Allocation Hot Path (`VER07`):** `step_raster_beam`, `step_vector_beam`, `rasterize_vector_stroke_into`, and `decay_phosphor_buffer` operate with zero heap allocations, verified by `CountingAllocator` in microbenchmarks.
- **Strict Input Validation (`CWE-20`):** Validates finite positive dimensions, validates positive finite slew rates, checks Q16.16 phosphor decay bounds, and checks timing parameters.
- **Memory Safety & Bounds Checks (`CWE-125`, `CWE-787`):** Slice bounds checks prevent out-of-bounds reads/writes on caller-provided phosphor buffers.
- **Arithmetic Safety (`CWE-190`):** Uses saturating intensity accumulation on phosphor excitation.
- **26-CWE Weakness Closure (`VER15`):** Complete formal closure or exclusion matrix for all 26 canonical CWE classes.

---

## Core Types & Functions

### Raster CRT Simulation
- `RasterTiming`: Configurable horizontal and vertical porch and sync timing (`standard_ntsc()`, `standard_arcade()`).
- `PhosphorType`: Models P4 white, P31 green, P1 green, and custom Q16.16 decay factor phosphors.
- `BeamFlags`: Sweep status flags (`in_hblank`, `in_vblank`, `in_hsync`, `in_vsync`, `frame_completed`, `line_completed`, `visible_pixel`).
- `CrtRasterBeam`: Deflection stepper advancing beam dot-by-dot, exciting phosphor persistence buffers.
- `lower_raster_to_pixel_buffer`: Lowers phosphor buffer to `PixelContainer` (`pixel-container`).
- `lower_raster_to_paint_scene`: Lowers phosphor buffer into universal `PaintScene` IR (`paint-instructions`).

### Vector Beam Simulation
- `VectorStroke`: Defines 2D vectors and bright-spot dwell points (`new_vector_stroke`, `new_dwell_spot`).
- `VectorBeamPoint`: Point record with beam deflection state, blanking, intensity, and amplifier settling status.
- `VectorBeamDisplay`: Simulates deflection amplifiers with bounded slewing rate and Z-axis blanking modulation.
- `rasterize_vector_stroke_into`: Rasterizes vector strokes into a caller-supplied phosphor buffer with zero heap allocations.
- `lower_vector_strokes_to_paint_scene`: Lowers vector strokes directly to `PaintScene` (`paint-instructions`).
