# GAME00 — Historical Game Engine Roadmap (Mainframe, Arcade, Retro Tile, Flash on LangVM, 2.5D/3D & Flight Sim)

> Active delivery priorities, capability/language status, verification limits, and discovered gaps are tracked in [`GAME-BACKLOG.md`](GAME-BACKLOG.md). This historical roadmap remains the catalog of target systems and their era context.

## Status

Roadmap / master spec. Surveys the technical and historical evolution of real-time computer
entertainment, interactive simulation, and game engines across six eras (1950s mainframe
simulations through hardware-accelerated 6-DOF flight simulation, including a dedicated
ActionScript/SWF Flash runtime on LangVM). It establishes a rigorous, layered reconstruction
strategy that reuses and continuously expands this repository's existing mathematical, geometric,
vector rendering (`PaintVM` `P2D00`–`P2D10`), windowing (`UI11`), audio synthesis (`MUS00`–`MUS07`),
and analysis/transcription (`MUS08`),
CPU simulation, and virtual machine substrates; pairs high-level pure simulation cores with
cycle-faithful functional and gate-level device simulators (PPU, APU, timer/DMA, cartridge mappers,
teletype, CRT/oscilloscope I/O) across Eras 0–2; and enforces repository-wide `VER-*` verification,
microbenchmarking, capability containment, and per-language common-CVE/CWE proof gates.

---

## §1 Why — Historical Reconstruction and Core Principles

This repository's identity is **faithful historical reconstruction**: bringing back the seminal
systems that shaped computing so that developers and researchers can study, execute, and verify
them within a modern, literate, and safe codebase. Video games and interactive simulations were
historically the primary crucible for real-time graphics algorithms, fixed-point and floating-point
physics integration, audio synthesis engines, domain-specific virtual machines, and deterministic
network synchronization.

Four foundational architectural principles govern every item across all eras in this roadmap:

### Principle 1: Pure Deterministic Simulation Core vs. Host Presentation Shell
Every game, simulation, and runtime engine is strictly divided into two distinct tiers:
1. **The Pure Deterministic Simulation Core (`portable` parity class):**
   - Implemented as a state transition function:
     $$\text{step}(\text{state}: \text{GameState}, \text{input}: \text{InputFrame}) \to \text{Result}<(\text{GameState}, \text{FrameOutput}), \text{GameError}>$$
   - Employs **zero ambient authority** (`"capabilities": []` in `required_capabilities.json`), avoiding all direct access to filesystem, network, wall-clock time, system entropy, or OS window handles.
   - All randomness is explicit and deterministic, parameterized via `rng::Pcg32` or `rng::Lcg` state held directly within `GameState`.
   - All arithmetic avoids cross-platform floating-point drift: either through fixed-point math (`Q16.16`/`Q32.32`) or strict IEEE-754 canonicalization.
   - Formally specified via Warrant (`VER04`/`VER04m` `state=explicit`), making game state machines verifiable for exhaustiveness, absence of panics, conservation invariants, and bounded execution steps (`VER07`/`VER20`).
2. **The Host Presentation & I/O Shell (`native-source` / `web-only` parity class):**
   - Connects the pure simulation core to the host platform via the repo's windowing (`UI11` `window-core`, `UI12`), event loop (`event-loop`), rasterizer (`paint-vm-runtime`, `P2D08` `path-raster`), and audio output (`MUS02` `audio-device-sink`, `MUS06` `web-audio-sink`).
   - Translates raw OS events into typed `InputFrame` records.
   - Consumes declarative `FrameOutput` (e.g., `PaintScene` from `P2D00` or raw pixel buffers from `IMG00`) and presents it to the user.

### Principle 2: Deep Substrate Reuse (Reconstruct the Engine, Not the Math or Host Plumbing)
A faithful game engine reconstruction should not duplicate the underlying numeric solvers,
geometry libraries, vector rasterizers, sound synthesizers, or bytecode interpreters that already
exist in this repository. Instead:
- Geometry and collision build on `point2d` (`G2D00`), `affine2d` (`G2D01`), `bezier2d` (`G2D02`), and `trig` (`PHY00`).
- Vector drawing lowers directly to `PaintInstructions` (`P2D00`) and `PaintVM` (`P2D01`), rendering identically in ASCII terminal (`P2D02`), software scanline (`P2D08`), and native GPU backends (Metal, Direct2D, Cairo, Skia, WGPU, OpenGL).
- Audio and chiptune sound synthesis lowers to `note-frequency` (`MUS00`), `portable-music-machine` (`MUS04`), and `unpitched-percussion` (`MUS07`).
- Scripting and Flash bytecode lower to `versioned-ecmascript-typescript-grammars.md`, `actionscript.grammar`, and the `InterpreterIR` (`LANG01`) / `vm-core` (`LANG02`) virtual machine pipeline.
- Matrix and 3D transformations leverage `matrix-ir` and `matrix-runtime` (`MX00`), with automatic dispatch to CPU or GPU accelerators.

### Principle 3: Continuous Substrate Expansion (Never Bypass PaintVM, UI11, or MUS)
As games advance from turn-based teletypes and vector arcades to complex tile engines, Flash runtimes,
and 3D flight simulators, host presentation and media synthesis must **never bypass** the repository's
foundational visual (`PaintVM`), windowing (`UI11`), or audio (`MUS`) subsystems through ad-hoc private
drivers or platform-specific backdoors. Instead, new rendering, input, and audio requirements must
drive formal upstream expansion of these foundational substrates:
- **`PaintInstructions` & `PaintVM` (`P2D00`–`P2D10`)**: Continuously expand to support retained-mode
  sprite and tile batching (`P2D09`), palette-indexed scanline blits (`P2D10`), and hardware blend/compositing
  modes (`SourceOver`, `Additive`, `Multiply`, `Screen`, `AlphaMask`, `InvertColor`), ensuring all eras
  benefit from declarative scene diffing (`PaintVM.patch()`) and uniform multi-backend rendering.
- **`UI11` Window Core (`UI11`, `UI11b`)**: Expand to support high-frequency frame and input cadence
  (60 Hz, 120 Hz, 144 Hz, 240 Hz vsync synchronizers), sub-millisecond polling for gamepads and analogue
  sticks, and deterministic `InputFrame` stream recording and replay.
- **Audio Stack**: Keep synthesis (`MUS00`–`MUS07`) and analysis/transcription (`MUS08`) aligned with their
  owning specifications. Use `audio-mixer-realtime` for real-time mixing and ring-buffer sink integration. Its Warrant covers
  mixer arithmetic and sample-rate operations.

### Principle 4: Dual Realization (Pure Software Core & Faithful Hardware Simulation Bridge)
Every historical game system across Eras 0–2 is realized along two complementary, mutually-verifying paths:
1. **The High-Level Pure Simulation Core**: A clean, idiomatic, portable state machine (`portable` parity class)
   implementing game rules with mathematical precision, ideal for algorithmic study, headless verification,
   and cross-language porting across all 18 Warrant rollout lanes.
2. **The Cycle-Faithful Device Simulator Bridge**: Low-level functional and gate-level device simulators
   (PPU, APU, timer/DMA, cartridge mapper, teletype/CRT/oscilloscope I/O) running unmodified historical
   binaries, ROMs, or discrete logic netlists atop this repository's CPU simulators (`pdp8`, `ge225`,
   `mos6502`, `z80`, `arm7tdmi`).
The pure simulation core and the hardware bridge form an **adversarial equivalence pair**: each frame's visual
digest and audio buffer produced by the high-level game state machine can be differentially verified against
the output of the cycle-accurate hardware simulator.

---

## §2 Progression Across Six Historical Eras

The roadmap progresses chronologically across six distinct historical eras, each introducing
fundamental advancements in simulation architecture, coordinate mathematics, rendering techniques,
and runtime execution:

```text
Era 0 (1950s–1960s): Mainframe & Teletype Turn-Based Simulations (Hamurabi, Lunar Lander)
  └── Hamurabi (`GAME01`), integer-scaled Lunar Lander (`GAME02`), and rng, with
      ASR-33 and PDP-8 / GE-225 telemetry I/O
Era 1 (1970s): Early Arcade 2D Vector & Raster (Pong, Breakout, Asteroids)
  └── Fixed-timestep loop + G2D00-G2D03 collision + Discrete-Logic TTL / CRT / Oscilloscope bridge
Era 2 (1980s–Early 2000s): 8-Bit/16-Bit/GBA Tile, Sprite & RPG Engine (Pokemon-style)
  └── Palette/Tilemap/Sprite OAM + NES/GB/GBA PPU/APU/DMA/Mapper bridges + PaintVM sprite batching
Track 2.5 (Late 1990s–2000s): Unmodified Flash (.swf) & ActionScript on LangVM
  └── SWF container parser + AS1/AS2/AS3 on LANG84/InterpreterIR/vm-core + DisplayList -> P2D00
Era 3 (Early 1990s): 2.5D Raycasters & Software 3D Rasterizers (Wolf3D, Doom, Quake)
  └── Fixed-point DDA raycaster + BSP/portal + Z-buffered triangle rasterizer -> IMG00 PixelContainer
Era 4 (Mid 1990s–2000s+): Hardware 3D & 6-DOF Flight Simulator
  └── 6-DOF rigid-body/aerodynamics (PHY + matrix-ir) + terrain LOD + cockpit HUD + G00-G09/wgpu
```

### Era 0 (1950s–1960s): Mainframe & Teletype Turn-Based Simulations (`GAME01a`–`GAME03b`)
- **Historical Context:** In the era of electromechanical teletypes (ASR-33) and time-sharing
  mainframes (PDP-8, PDP-10, GE-225), computer games were discrete, turn-based simulations of
  complex systems: agricultural economics (*The Sumerian Game* / *Hamurabi*), rocket descent
  dynamics (*Lunar Lander*), and wilderness survival (*Oregon Trail*). Output was printed line by line.
- **Architectural Mechanics:**
  - Discrete turn progression: The player inputs resource allocations or throttle commands; the engine
    applies differential formulas, updates state, and emits tabular teletype text.
  - Absence of real-time clocks: State advance occurs solely in response to valid command input.
  - Discrete PRNG events: Crop yields, plagues, rat infestations, and sensor noise are driven by
    pseudo-random number generation seeded explicitly.
  - Serial Terminal Emulation: Real-time pacing of 110-baud serial lines, 10 char/sec mechanical typing,
    paper tape punch/reader protocols, and mechanical carriage return delays.
- **Substrate Primitives:**
  - `fixed-point` (`GAME01a`, future proposal): Deterministic `Q16.16` and `Q32.32` arithmetic, sqrt, Cordic trig, and overflow flags.
  - `sim-turn-core` (`GAME01b`, future proposal): Generic turn-based execution harness for command validation, replay history, and deterministic transcripts.
  - `hamurabi` (`GAME01`): Deterministic Sumerian economic simulation, with `rng` as its specified direct dependency.
  - `lunar-lander` (`GAME02`): Discrete fixed-point rocket descent simulation, with `paint-vm-ascii` (`P2D02`)
    in the presentation bridge.
  - `sim-teletype-asr33` (`GAME03a`): Teletype ASR-33 terminal simulator modeling 110-baud serial I/O,
    10 char/sec mechanical print cycle, 8-level paper tape punch/reader, and carriage return delays.
  - `sim-pdp8-ge225-io-bridge` (`GAME03b`): I/O bus and teletype register interface connecting
    `pdp8-simulator` (`07aa`) and `ge225-simulator` (`07g`) to terminal I/O. Polling loops print telemetry from
    the modern game packages. `GAME03` plans original-program execution on the same simulated systems.

### Era 1 (1970s): Early Arcade, Fixed-Timestep Loop & 2D Collision (`GAME10`–`GAME15c`)
- **Historical Context:** Discrete transistor-transistor logic (TTL) and early microprocessors (Intel 8080,
  MOS 6502) gave birth to the commercial coin-operated arcade industry (*Pong*, *Breakout*, *Space Invaders*,
  *Asteroids*). These games introduced continuous real-time movement, fixed-rate simulation clocks,
  and geometric collision response.
- **Architectural Mechanics:**
  - Fixed-timestep accumulator loop: Decouples physical simulation updates (`dt_fixed = 1/60s`) from variable
    render frame rates, interpolating visual state via remainder fraction $\alpha \in [0, 1)$.
  - Bounded frame skipping: Implements maximum sub-step limits to guard against the algorithmic "spiral of death"
    (CWE-400) under CPU starvation.
  - Continuous swept 2D collision: Ray-vs-AABB, swept-circle vs swept-AABB, and time-of-impact (TOI)
    calculations to prevent tunneling at high velocities.
  - Simple parametric chiptune audio: Square-wave pitch envelopes and linear feedback shift register (LFSR)
    pseudo-noise bursts for paddle hits, wall bounces, and explosions.
  - Discrete Logic & CRT Timing: Netlists of 7400-series counters, gates, and flip-flops driving 15.75 kHz
    horizontal and 60 Hz vertical CRT beam deflection without CPU or framebuffers.
- **Substrate Primitives:**
  - `game-clock` (`GAME10`): Deterministic fixed-step clock with frame-skip protection, specified in [`GAME10`](GAME10-fixed-step-clock-contract.md).
  - `collision2d` (`GAME11`): High-precision 2D intersection queries and reflection physics built on `point2d` (`G2D00`) and `affine2d` (`G2D01`).
  - `arcade-sfx` (`GAME12`): Parametric sound generator emitting raw PCM buffers into `MUS04`/`MUS07`.
  - `game-pong` & `game-breakout` (`GAME13`, `GAME14`): Reference arcade titles emitting `PaintScene` (`P2D00`) with stable IDs for efficient `PaintVM.patch()` diffing.
  - `sim-discrete-arcade-logic` (`GAME15a`): Gate-level netlist simulator for 7400-series TTL ICs, clock dividers generating 15.75 kHz HSync / 60 Hz VSync raster timing, and paddle potentiometer ADC.
  - `sim-crt-vector-beam` (`GAME15b`): CRT raster beam deflection, horizontal/vertical blanking intervals, phosphor persistence decay modeling, and oscilloscope vector display stroke synthesis.
  - `sim-discrete-audio-timer` (`GAME15c`): 555-timer square-wave generator, LFSR noise generator, and diode-shaped analog filtering for arcade audio cues.

### Era 2 (1980s–Early 2000s): 8-Bit/16-Bit/GBA Tile, Sprite & RPG Engine (`GAME20`–`GAME29b`)
- **Historical Context:** Home consoles and handhelds (NES, SNES, Game Boy, Game Boy Advance) utilized
  specialized Picture Processing Units (PPUs) with dedicated hardware memory for palettes, character
  generator pattern tables (tiles), multi-plane background scrolling, and Object Attribute Memory (OAM)
  hardware sprites. This architecture powered the golden age of top-down 2D Role-Playing Games (*Pokémon*,
  *The Legend of Zelda*, *Final Fantasy*).
- **Architectural Mechanics:**
  - Indexed Color & Palettes: Sub-byte pixel formats (2bpp, 4bpp, 8bpp) mapping into 16-color or 256-color
    palette tables, supporting hardware color-zero transparency, palette cycling, and screen fades.
  - Scrolling Tilemaps: Background layers assembled from $8\times 8$ or $16\times 16$ tile indices with horizontal/vertical
    mirroring flags, coarse and fine pixel scrolling, and hardware viewport culling.
  - Sprite OAM: Prioritized tables of active actor sprites supporting metasprite grouping, priority layering
    behind/above background tiles, and affine rotation/scaling.
  - Grid RPG Systems: Tile-based step movement, encounter rate step counters, warp trigger zones,
    dialogue tree layout engines, and turn-based battle state machines with type effectiveness matrices.
  - Hardware Device Simulation: Scanline/cycle-accurate simulation of NES Ricoh 2C02 PPU / 2A03 APU, Game Boy
    LCD controller / Sound APU, and GBA MMIO registers (`0x04000000`), Mode 0–5 PPU, and 4-channel DMA controller.
- **Substrate Primitives:**
  - `palette-indexed` (`GAME20`): Indexed color formats and palette transition engines.
  - `tilemap2d` (`GAME21`): Multi-layer scrolling background engine with tile-attribute collision maps.
  - `sprite-oam` (`GAME22`): Hardware-accurate sprite attribute manager and metasprite animator.
  - `rpg-grid-world` (`GAME23`): Deterministic overworld navigation, map loading, and dialogue parsing.
  - `rpg-battle-engine` (`GAME24`): Turn-based battle state machine with deterministic damage calculations.
  - `game-monster-rpg` (`GAME25`): A complete, playable top-down monster-collecting RPG vertical slice.
  - `gba-hardware-bridge` (`GAME26`): Optional bridge interfacing `arm7tdmi-simulator` (`07ax`) memory-mapped
    registers (`0x04000000`) with our native tile and sprite pipeline.
  - `sim-nes-ppu` (`GAME27a`): Ricoh 2C02 PPU simulation with cycle-accurate scanline timing, nametable mirroring, 64-sprite OAM table, sprite 0 hit detector, and palette RAM.
  - `sim-nes-apu-mappers` (`GAME27b`): Ricoh 2A03 APU simulation (pulse, triangle, noise, DMC) and cartridge mappers (NROM, MMC1, MMC3 with scanline IRQ counter).
  - `sim-gameboy-ppu` (`GAME28a`): LR35902 LCD controller PPU simulation with Modes 0–3 scanline timing, 160×144 LCD viewport, window layer, background tilemap, and 40 OAM sprites.
  - `sim-gameboy-apu-mbc` (`GAME28b`): Game Boy 4-channel sound controller (pulse, wave RAM, noise) + MBC1/MBC2/MBC3 (RTC)/MBC5 mappers and hardware timers (`DIV`, `TIMA`).
  - `sim-gba-ppu-dma` (`GAME29a`): ARM7TDMI GBA Video Controller (Modes 0–5, affine rotation/scaling, windowing, alpha blending) and 4-channel DMA controller mapped to `0x04000000`.
  - `sim-gba-audio-timer` (`GAME29b`): GBA DirectSound FIFO A/B with 8-bit PCM DMA mixing, legacy DMG sound channels, and cascading hardware timers 0–3.
  - Substrate Expansion Hooks: Lowers retained-mode tile/sprite batches directly into `P2D09` (`paint-batch-tiles`),
    indexed scanline blits into `P2D10` (`paint-palette-blend`), high-cadence inputs into `UI11b` (`window-cadence-input`),
    and multi-channel chiptune/PCM audio through `audio-mixer-realtime`. Its Warrant covers formal arithmetic
    and sample-rate operations.

### Track 2.5 (Late 1990s–2000s): Unmodified Flash (.swf) & ActionScript on LangVM (`SWF00`–`SWF08`)
- **Historical Context:** Macromedia/Adobe Flash revolutionized web animation and indie gaming. Millions of
  browser games were authored in vector art and programmed in ActionScript 1.0, 2.0, and 3.0. Running these
  games today without proprietary plugins or bloated C++ runtimes requires a clean, sandboxed,
  specification-compliant engine.
- **Architectural Mechanics:**
  - Vector Graphics Pipeline: SWF quadratic Bézier curves and dual-sided fill contours lower directly onto
    `G2D01` (`affine2d`), `G2D02` (`bezier2d`), `P2D08` (`path-raster`), and `P2D00` (`PaintInstructions`).
  - Hierarchical DisplayList: Interactive scene graph of `MovieClip`, `SimpleButton`, `TextField`, and `Shape`
    instances with timeline frame advancement and depth sorting, diffed across frames via `PaintVM.patch()`.
  - Bytecode Lifting to LangVM: Both AVM1 actions (`DoAction`) and AVM2 ABC bytecode (`DoABC`) are lifted
    directly into validated `InterpreterIR` (`LANG01`) modules executed by `vm-core` (`LANG02`).
  - Source Compilation: ActionScript 1/2/3 source files are parsed via `actionscript.grammar` and lowered
    to `InterpreterIR` following the `LANG84` specification.
- **Dedicated Master Specification:** See [`SWF00-flash-runtime-roadmap.md`](SWF00-flash-runtime-roadmap.md)
  for the complete architectural specification, tag parser design, and vulnerability defense matrix.

### Era 3 (Early 1990s): 2.5D Raycasters & Software 3D Rasterizers (`GAME30`–`GAME35`)
- **Historical Context:** Before dedicated 3D accelerator cards existed, games pushed CPU boundaries using
  clever pseudo-3D and software rasterization techniques: raycasting on orthogonal 2D grids (*Wolfenstein 3D*),
  Binary Space Partitioning (BSP) trees with 1D horizontal portal clipping (*Doom*), and full 6-DOF
  perspective-correct software triangle rasterization (*Quake*).
- **Architectural Mechanics:**
  - Digital Differential Analyzer (DDA) Grid Raycasting: Casts rays through 2D grid cells to locate wall
    intersections, rendering vertical screen slices with texture coordinate scaling.
  - 2D BSP & Sector Portal Traversal: Front-to-back traversal of convex 2D sectors, clipping view frustum
    portals against screen-space horizontal occlusion arrays.
  - Software 3D Pipeline: Homogeneous 4D clipping (Sutherland-Hodgman) against the view frustum, perspective
    projection ($x/w, y/w$), sub-pixel edge-function triangle rasterization, 16/32-bit depth buffering ($Z$ or $1/w$),
    perspective-correct affine texture mapping ($\frac{u}{z}, \frac{v}{z}, \frac{1}{z}$), and Gouraud lighting.
- **Substrate Primitives:**
  - `math3d` (`GAME30`): Fixed-point and `f64` 3D vector, matrix, and quaternion math.
  - `raycast-dda` (`GAME31`): Grid raycasting engine with wall, floor/ceiling, and billboard sprite rendering.
  - `bsp-portal25d` (`GAME32`): 2D sector BSP tree generator, portal clipper, and sector visibility builder.
  - `raster3d-soft` (`GAME33`): Pure software Z-buffered perspective triangle rasterizer outputting to `PixelContainer` (`IMG00`).
  - `game-labyrinth3d` (`GAME34`): Playable 2.5D/3D first-person dungeon crawler with deterministic frame digests.
  - `bsp3d-quake-c-bridge` (`GAME35`): 3D BSP traversal and QuakeC bytecode execution bridge on `InterpreterIR`.

### Era 4 (Mid 1990s–2000s+): Hardware 3D & 6-DOF Flight Simulators (`GAME40`–`GAME45`)
- **Historical Context:** The advent of dedicated 3D graphics hardware (3dfx Voodoo, OpenGL, Direct3D)
  allowed simulators to abandon software rendering tricks and simulate true six-degrees-of-freedom (6-DOF)
  physics over expansive terrain meshes with realistic aerodynamics (*subLOGIC Flight Simulator*,
  *Microsoft Flight Simulator*, *Falcon 4.0*).
- **Architectural Mechanics:**
  - 6-DOF Rigid-Body Dynamics: Symplectic Velocity-Verlet or Runge-Kutta 4 (RK4) integration of linear momentum,
    angular momentum, body inertia tensors, and quaternion orientation.
  - Aerodynamic Coefficients: Calculation of dynamic pressure ($q = \frac{1}{2}\rho v^2$), angle of attack ($\alpha$),
    sideslip angle ($\beta$), and lookup tables for non-linear lift, drag, and pitching moment coefficients ($C_L, C_D, C_m$).
  - International Standard Atmosphere (ISA): Elevation-dependent air density, temperature, and pressure models.
  - Continuous Terrain Level of Detail (LOD): Procedural heightmap synthesis with quadtree/ROAM mesh refinement.
  - Cockpit Glass Displays: Flight instrumentation (attitude indicator, altimeter tape, airspeed indicator,
    horizontal situation indicator) rendered as vector overlays via `P2D00` `PaintInstructions`.
- **Substrate Primitives:**
  - `rigidbody-6dof` (`GAME40`): 6-DOF rigid-body physics integrator with ground contact models.
  - `aero-flight-model` (`GAME41`): Standard atmosphere and aerodynamic surface calculation engine.
  - `terrain-lod` (`GAME42`): Heightfield generation and view-dependent terrain mesh decimator.
  - `cockpit-hud-paint` (`GAME43`): Primary Flight Display (PFD) and Heads-Up Display (HUD) vector graphics.
  - `render3d-gpu-bridge` (`GAME44`): Hardware 3D renderer interfacing with `gpu-runtime` (`G00`–`G09`) and `wgpu`/`metal`/`vulkan`, featuring seamless deterministic CPU fallback to `raster3d-soft` (`GAME33`).
  - `game-flight-sim` (`GAME45`): Complete playable 6-DOF flight simulator with black-box flight recorder.

---

## §3 Functional & Gate-Level Device Simulators (Hardware Bridges Across Eras 0–2)

To satisfy the repository's identity of uncompromising historical reconstruction, every game from
Eras 0–2 is supported not only by a pure high-level simulation core, but also by a **cycle-faithful
hardware device simulator**. This architecture bridges games directly onto equivalent historical
hardware simulators (`pdp8` and `ge225` for Era 0, discrete-logic and CRT for Era 1, `mos6502` NES PPU/APU,
`z80` Game Boy, and `arm7tdmi` GBA MMIO/PPU/DMA for Era 2).

### §3.1 Architectural Role: Dual-Track Execution & Ground-Truth Verification
The high-level pure simulation core and the hardware device simulator form an **adversarial verification pair**:
- The pure simulation core runs natively in polyglot environments with zero dependencies, executing rules
  via deterministic fixed-point math and explicit state machines.
- The hardware device simulator runs unmodified original ROMs, machine code, or netlists, simulating
  every hardware register, bus cycle, DMA transfer, scanline dot, and audio DAC step.
- Automated conformance tests run both engines concurrently with synchronized `InputFrame` sequences,
  verifying that game event milestones, score tallies, state transitions, and audio/visual output digests
  match with bit-exact fidelity.

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                    Dual-Track Conformance Architecture                       │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│                        Deterministic Input Sequence                          │
│                                     │                                        │
│                 ┌───────────────────┴───────────────────┐                    │
│                 ▼                                       ▼                    │
│    ┌───────────────────────────┐           ┌────────────────────────────┐    │
│    │ High-Level Simulation     │           │ Cycle-Faithful Hardware    │    │
│    │ Core (Pure State Machine) │           │ Simulator Bridge           │    │
│    │ (GAME01, GAME13, GAME25)  │           │ (CPU + PPU + APU + MMIO)   │    │
│    └────────────┬──────────────┘           └────────────┬───────────────┘    │
│                 │                                       │                    │
│                 ▼                                       ▼                    │
│         High-Level Output                       Emulated Device Output       │
│      (PaintScene / AudioPCM)                 (Scanline Frame / AudioPCM)     │
│                 │                                       │                    │
│                 └───────────────────┬───────────────────┘                    │
│                                     ▼                                        │
│                    Differential Conformance Verifier                         │
│                    (Bit-Exact Equivalence Assertion)                         │
└──────────────────────────────────────────────────────────────────────────────┘
```

### §3.2 Era 0 Mainframe Device Simulators (`GAME03a`, `GAME03b`)
- **ASR-33 Teletype Terminal Simulator (`sim-teletype-asr33`, `GAME03a`):**
  - Models electromechanical terminal operation: 110 baud serial communication (1 start bit, 8 data bits,
    2 stop bits at 10 characters per second).
  - Simulates physical carriage return (`\r`) and line feed (`\n`) timing delays, mechanical printhead travel,
    and mechanical bell chime (`\x07`).
  - Full 8-level perforated paper tape reader and punch simulator, enabling loading of original paper tape dumps.
- **PDP-8 & GE-225 CPU I/O Bus Bridge (`sim-pdp8-ge225-io-bridge`, `GAME03b`):**
  - Bridges `pdp8-simulator` (`07aa`) I/O transfer instructions: Teleprinter Keyboard/Reader (Device 03: `KSF`,
    `KCC`, `KRS`, `KRB`) and Teleprinter Printer/Punch (Device 04: `TSF`, `TCF`, `TPC`, `TLS`), including
    hardware interrupt request flags (`ION`/`IOF`).
  - Bridges `ge225-simulator` (`07g`) Datanet terminal controller and console typewriter channels.
  - Polling loops read and print telemetry from modern game packages through modeled terminal I/O.
    `GAME03` plans execution of original FOCAL and Dartmouth BASIC programs on these simulated systems.

### §3.3 Era 1 Discrete-Logic & Display Simulators (`GAME15a`, `GAME15b`, `GAME15c`)
- **Discrete TTL Gate-Level Logic Simulator (`sim-discrete-arcade-logic`, `GAME15a`):**
  - Netlist simulation of 7400-series TTL integrated circuits (7400 NAND, 7402 NOR, 7474 D flip-flops,
    7490 decade counters, 7493 4-bit binary counters, 74153 multiplexers).
  - Master clock divider chains generating historical 15.75 kHz horizontal sync (HSync) and 60 Hz vertical
    sync (VSync) timing pulses directly from a 14.31818 MHz crystal oscillator.
  - Simulates paddle potentiometer analog-to-digital timing via 555-timer / R-C ramp comparator circuits.
- **CRT Raster Beam Deflection & Oscilloscope Vector Display (`sim-crt-vector-beam`, `GAME15b`):**
  - Raster CRT Beam Deflection: Electron beam scanline sweep, horizontal blanking (HBlank), vertical blanking
    (VBlank), front/back porch intervals, and P4 white / P31 green phosphor persistence decay modeling.
  - Oscilloscope / Vector Beam Display: $X/Y$ deflection amplifier simulation, $Z$-axis intensity modulation
    (beam blank/unblank), beam slewing rates, and bright-spot dwell timing for vector arcade classics
    (*Asteroids*, *Spacewar!* on DEC Type 30 CRT).
- **Discrete Arcade Audio & Coin Mechanism (`sim-discrete-audio-timer`, `GAME15c`):**
  - Analog audio circuit modeling: 555 astable multivibrators generating paddle deflection tones (approx 480 Hz)
    and wall collision tones (approx 240 Hz).
  - Shift-register pseudorandom noise generation with diode-shaped filtering for explosion sounds.
  - Mechanical coin acceptor switch debounce and credit latching logic.

### §3.4 Era 2 Console & Handheld Functional Device Simulators (`GAME27a`–`GAME29b`)
- **MOS 6502 + NES Hardware Bridge (`sim-nes-ppu`, `sim-nes-apu-mappers`, `GAME27a`, `GAME27b`):**
  - *Ricoh 2C02 PPU (`GAME27a`):* 240 visible scanlines, 341 master PPU dots per scanline, even/odd frame
    cycle skip, VBlank NMI at scanline 241 dot 1. 64-entry Object Attribute Memory (OAM) with hardware
    8-sprite-per-scanline evaluation and overflow flag. Sprite 0 hit pixel-accurate timing detector.
    Full nametable scrolling engine with coarse and fine scroll registers (`v`, `t`, `x`, `w`) and
    internal palette RAM (`$3F00`–`$3F1F`) with color emphasis bit emulation.
  - *Ricoh 2A03 APU & Mappers (`GAME27b`):* Pulse 1 and Pulse 2 channels with 4 duty cycle modes (12.5%,
    25%, 50%, 75%) and hardware frequency sweep units. Triangle channel with linear counter and 32-step DAC.
    Noise channel with 15-bit and 9-bit looped LFSR polynomials. Delta Modulation Channel (DMC) executing
    DMA memory reads from CPU space. 4-step (60 Hz) and 5-step (48 Hz) frame sequencers.
  - *Cartridge Mappers:* NROM (Mapper 0), MMC1 (Mapper 1, serial shift-register banking), and MMC3 (Mapper 4,
    scanline IRQ counter clocked by PPU address line A12, PRG/CHR banking).
- **Z80 / LR35902 + Game Boy Hardware Bridge (`sim-gameboy-ppu`, `sim-gameboy-apu-mbc`, `GAME28a`, `GAME28b`):**
  - *LCD Controller PPU (`GAME28a`):* Pixel Processing Unit cycle state machine: Mode 2 (OAM Search, 80 dots),
    Mode 3 (Pixel Transfer, 172–289 dots with pixel FIFO), Mode 0 (H-Blank, 85–202 dots), Mode 1 (V-Blank,
    4560 dots for scanlines 144–153). LCD Status (`STAT`) interrupts. 160×144 LCD viewport, Window plane,
    Background tilemap, and 40 OAM sprites (10 sprites/line limit, priority by X-coordinate). DMG 4-shade palettes
    (`BGP`, `OBP0`, `OBP1`).
  - *Sound Controller APU, Timers & MBC (`GAME28b`):* 4-channel sound processor: CH1 (pulse with frequency sweep
    and volume envelope), CH2 (pulse with envelope), CH3 (32-entry 4-bit custom wave RAM), CH4 (LFSR noise with
    7-bit/15-bit polynomial); stereo panning (`NR51`), master volume (`NR50`). Memory Bank Controllers: MBC1
    (up to 2MB ROM / 32KB RAM), MBC2 (built-in 512×4-bit RAM), MBC3 (with Real-Time Clock registers), MBC5.
    Hardware timers: `DIV` (16384 Hz clock), `TIMA` with selectable prescalers (4096, 262144, 65536, 16384 Hz),
    `TMA` auto-reload, and timer overflow interrupt.
- **ARM7TDMI + Game Boy Advance Hardware Bridge (`sim-gba-ppu-dma`, `sim-gba-audio-timer`, `GAME29a`, `GAME29b`):**
  - *Video Display Controller & 4-Channel DMA (`GAME29a`):* Video modes: Modes 0, 1, 2 (tiled text and affine
    transform background layers), Mode 3 (16-bit 240×160 RGB555), Mode 4 (8-bit paletted 240×160 double-buffered),
    Mode 5 (16-bit 160×128 double-buffered). 128 OAM sprites with affine rotation/scaling. Window layers (`WIN0`,
    `WIN1`, `WINOBJ`). Hardware alpha blending and brightness fading (`BLDCNT`, `BLDALPHA`, `BLDY`). 4-channel DMA
    controller: DMA0–DMA3 supporting 16-bit and 32-bit transfers, immediate, VBlank, HBlank, and sound FIFO transfer
    triggers, auto-repeat and address reload modes. Full memory-mapped register space at `0x04000000`–`0x040003FF`.
  - *DirectSound & Timers (`GAME29b`):* Sound controller: legacy DMG channels 1–4, DirectSound Channels A and B
    with 8-bit PCM FIFO buffers fed by DMA, stereo panning and volume controls. Timers 0–3 with cascade prescalers
    (1, 64, 256, 1024 cycles) and timer overflow IRQs.

### §3.5 Device Simulator Formal Verification & Security Hardening
Every device simulator package must satisfy repository-wide verification gates:
- **Rust Safety:** `#![forbid(unsafe_code)]` across all simulator cores.
- **Warrant Specifications (`VER04m`):** Hardware registers, DMA state transitions, and scanline/cycle
  counters specified as formal state machines (`state=explicit`) with bounded step theorems (`VER20`).
- **Zero-Allocation Hot Loops (`VER07`):** Scanline rendering, cycle stepping, and audio buffer generation
  must not allocate dynamic heap memory during execution (`max_alloc_bytes: Some(0)`).
- **Capability Confinement (`VER08a`/`VER08b`):** Declares `"capabilities": []` in `required_capabilities.json`;
  simulators operate purely on caller-supplied memory buffers with zero ambient authority.
- **Common-CVE / CWE Proof Gates:** Dedicated test suites verifying:
  - **CWE-190:** MMIO address decoding wrap and scanline/cycle counter arithmetic.
  - **CWE-125 / CWE-787:** Strict memory bounds checks on VRAM, OAM, CRAM/palette RAM, wave RAM, and cartridge bank switching.
  - **CWE-400 / CWE-835:** Guaranteed termination on cycle-stepping loops and DMA transfer chains.
  - **CWE-843:** Strict alignment enforcement on 16-bit and 32-bit MMIO/DMA bus transfers.

---

## §4 Continuous Substrate Expansion: PaintVM, UI11, and MUS

Under no circumstances may a historical game engine or runtime track introduce private rendering
backends, ad-hoc OS window hooks, or standalone audio drivers to bypass repository substrates.
Instead, advancing through historical eras drives continuous **upstream expansion** of `PaintInstructions`
(`P2D00`), `PaintVM` (`P2D01`–`P2D10`), `UI11` Window Core (`UI11`, `UI11b`), and audio synthesis
(`MUS00`–`MUS07`) plus analysis/transcription (`MUS08`).

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                    Continuous Substrate Expansion Flow                       │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│    New Era Requirements                                                      │
│    (Tilemaps, Palettes, Sprites, Blending, 240Hz, Gamepads, Audio Mixing)    │
│                               │                                              │
│                               ▼                                              │
│                  Formal Upstream Substrate PR                                │
│                               │                                              │
│            ┌──────────────────┼──────────────────┐                           │
│            ▼                  ▼                  ▼                           │
│     PaintVM Expansion    UI11 Expansion     MUS Expansion                    │
│     • P2D09 Batch Tiles  • UI11b Cadence    • audio-mixer-realtime                 │
│     • P2D10 Blend/Blit   • Gamepad / Poll   • Multi-Channel Voices           │
│     • Retained Diffing   • Replay Streams   • Zero-Alloc Ring Sinks          │
│            │                  │                  │                           │
│            └──────────────────┼──────────────────┘                           │
│                               ▼                                              │
│                  Repository-Wide Availability                                │
│        (Used by All Game Engines, Flash Runtimes, and Tooling)               │
└──────────────────────────────────────────────────────────────────────────────┘
```

### §4.1 Visual Subsystem Expansion (`PaintInstructions` `P2D00` & `PaintVM` `P2D01`–`P2D10`)
To support arcade rasterizers, console tilemaps, sprite engines, Flash display lists, and 3D flight HUDs:
- **Retained-Mode Sprite and Tile Batching (`paint-batch-tiles`, `P2D09`):**
  - Introduces `DrawTileBatch`, `DrawSpriteBatch`, and `DrawMetasprite` primitives into `PaintInstructions`.
  - Encapsulates tile atlas handles, source UV rectangles, destination coordinates, mirroring flags (H-flip,
    V-flip), priority layers, and palette IDs in contiguous flat arrays.
  - Retains declarative diffing via `PaintVM.patch()`: the VM diffs tile batches by stable layer/chunk keys,
    emitting minimal draw calls and dirty-rectangle updates only for modified tiles.
- **Palette-Indexed & Scanline Blits (`paint-palette-blend`, `P2D10`):**
  - First-class support for sub-byte indexed color formats (1bpp, 2bpp, 4bpp, 8bpp) mapping through dynamic
    palette lookup tables directly inside `P2D08` (`path-raster`) and native GPU backends.
  - Scanline-level blit instructions (`BlitScanlineIndexed`, `BlitPaletteTile`) enabling cycle-accurate
    horizontal blanking palette swaps, water ripple effects, and full-screen color fades without re-expanding
    framebuffers to full 32-bit RGBA.
- **Hardware Blend & Compositing Modes (`paint-palette-blend`, `P2D10`):**
  - Expands `PaintInstructions` to support Porter-Duff compositing operators and advanced hardware blend modes:
    `SourceOver`, `Additive`, `Multiply`, `Screen`, `AlphaMask`, `InvertColor`, and fractional alpha fades.
  - Matches the exact mathematical blend models required by Game Boy Advance hardware registers (`BLDCNT`),
    Flash `DisplayObject.blendMode`, and modern GPU pipelines.
- **Sub-Pixel Bézier & Contour Rasterization (`path-raster`, `P2D08`):**
  - High-performance rasterization of quadratic and cubic Bézier contours with anti-aliasing, supporting
    arbitrary Flash vector fills and crisp Primary Flight Display (PFD) HUD instruments.

### §4.2 Windowing & Event Subsystem Expansion (`UI11` Window Core & `UI11b` `window-cadence-input`)
To deliver arcade responsiveness and zero-latency player interaction:
- **High-Frequency Frame Cadence (`UI11b`):**
  - Extends `UI11` windowing to support high-refresh presentation (60 Hz, 120 Hz, 144 Hz, 240 Hz) with
    vsync timestamp synchronization and precision frame-interval locks.
  - Integrates seamlessly with `game-clock` (`GAME10`) fixed-timestep accumulator loops.
- **Sub-Millisecond Input Polling & Gamepad Integration (`UI11b`):**
  - Decouples input sampling from presentation frame rate, polling keyboard, mouse, and gamepads at
    sub-millisecond cadence to eliminate input lag.
  - Standardizes gamepad abstractions: dual analogue thumbsticks with radial deadzone normalization,
    analogue triggers, 16-button digital bitmasks, and hot-plug event notifications.
- **Deterministic Input Streams (`UI11b`):**
  - Native serialization and deserialization of `InputFrame` event sequences.
  - Enables bit-identical session recording, automated fuzz testing, golden replay validation, and
    multiplayer rollback netcode.
- **Integer-Ratio Scaling & CRT Presentation:**
  - Standardized integer-ratio scaling (1x, 2x, 3x, 4x CRT integer scaling) and phosphor/scanline shader
    presentation modes across `window-canvas`, `window-win32`, and `window-appkit`.

### §4.3 Audio Substrates (`MUS00`–`MUS08` and `audio-mixer-realtime`)
To power parametric arcade chiptunes, multi-voice console APUs, and streaming Flash audio:
- **Real-Time Multi-Channel Software Audio Mixer (`audio-mixer-realtime`):**
  - Zero-allocation audio mixer combining arbitrary simultaneous audio streams (background chiptune music,
    sound effects, streaming PCM, software synthesizers) into interleaved stereo 48 kHz / 44.1 kHz 32-bit float
    or 16-bit integer PCM buffers.
  - Per-channel volume attenuation, stereo panning, pitch shifting, and master limiting.
- **Polyphonic Chiptune & Synthesizer Extensions (`MUS04`, `MUS07`):**
  - First-class generative voices added to `MUS04`/`MUS07`: pulse waves with selectable duty cycles (12.5%,
    25%, 50%, 75%), triangle waves with linear envelopes, LFSR pseudo-random noise with short/long period
    modes, 4-bit/8-bit custom wavetable synthesis, and ADPCM decoders.
- **Zero-Latency Device Sink Hooks (`audio-mixer-realtime`):**
  - Lock-free, double-buffered ring buffers connecting the real-time mixer directly to platform sinks
    (`MUS02` `audio-device-sink`, `MUS06` `web-audio-sink`).
  - Strict zero dynamic heap allocation in the audio rendering callback (`VER07`), preventing audio buffer
    underruns and glitches.

---

## §5 Substrate Reuse Matrix

The following table demonstrates how each era maps onto and reuses existing packages across this repository,
incorporating the continuous expansions of `PaintVM`, `UI11`, the audio substrates, and hardware device simulators:

| Substrate Domain | Packages / Milestones | Reused In Eras / Packages | How Reused |
|---|---|---|---|
| **Deterministic Randomness** | `rng` (`PCG32`, `LCG`, `Xorshift64`) | All Eras (`GAME01`–`GAME45`, `SWF05`) | Explicit PRNG state carried in `GameState`; rejection-sampled range distributions. |
| **Fixed-Point & Scalars** | Integer-scaled physics (`GAME02`), `numeric-tower` | Eras 0, 1, 2, 3 (`GAME02`, `GAME11`, `GAME31`) | Scaled integer math for deterministic simulation. |
| **2D Geometry & Transforms** | `point2d` (`G2D00`), `affine2d` (`G2D01`), `bezier2d` (`G2D02`), `arc2d` (`G2D03`), `trig` (`PHY00`) | Eras 1, 2, 2.5 (`GAME11`, `GAME22`, `SWF02`) | AABB collision, sprite transform matrices, Flash vector curves, trigonometric trajectory calculation. |
| **Vector IR, Batching & Blits** | `P2D00` (`PaintInstructions`), `P2D01` (`PaintVM`), `P2D08` (`path-raster`), `P2D09` (`paint-batch-tiles`), `P2D10` (`paint-palette-blend`) | Eras 1, 2, 2.5, 4 (`GAME13`, `GAME21`, `SWF02`, `GAME43`) | Declarative scene graphs; retained-mode tile/sprite batching; indexed-color scanline blits; hardware blend modes; retained-mode scene diffing via `PaintVM.patch()`. |
| **ASCII / Doc Output** | `P2D02` (`paint-vm-ascii`), `P2D03`/`P2D04` (`format-doc`) | Era 0, 1 (`GAME01`, `GAME02`, `GAME13`) | Terminal game rendering, telemetry plots, dialogue boxes, retro teletype simulation. |
| **Pixel Containers & Codecs** | `IMG00` (`pixel-container`), `IMG03`–`IMG05` (compositing), `IC01`–`IC18` (codecs) | Eras 2, 3, 4 (`GAME20`, `GAME33`, `SWF01`) | Paletted framebuffers, software 3D Z-buffers, textures, embedded Flash bitmap decoding (PNG, JPEG, GIF). |
| **Windowing, Cadence & Events** | `event-loop`, `UI11` (`window-core`), `UI11b` (`window-cadence-input`), `window-appkit`, `window-win32`, `window-canvas` | All Eras (Host Shell) | OS window creation, 60/120/240 Hz vsync scheduling, sub-ms input polling (keyboard, gamepad, mouse), deterministic input stream recording. |
| **Audio Synthesis & Analysis** | `MUS00`–`MUS08` | Eras 1, 2, 2.5, 4 (`GAME12`, `GAME25`, `SWF06`, `GAME45`) | Synthesis and chiptune packages (`MUS00`–`MUS07`) plus analysis/transcription (`MUS08`). |
| **Real-Time Audio Mixer** | `audio-mixer-realtime` (`audio-mixer-realtime.warrant`) | Eras 1, 2, 2.5, 4 (`GAME12`, `GAME25`, `SWF06`, `GAME45`) | Multi-channel software mixing and ring-buffer device-sink integration. |
| **CPU & Hardware Simulators** | `07aa` (PDP-8), `07g` (GE-225), `07j` (MOS 6502), `07k` (Z80), `07ax` (ARM7TDMI), `sim-teletype-asr33`, `sim-discrete-arcade-logic`, `sim-crt-vector-beam`, `sim-nes-ppu`, `sim-nes-apu-mappers`, `sim-gameboy-ppu`, `sim-gameboy-apu-mbc`, `sim-gba-ppu-dma` | Eras 0, 1, 2 (`GAME03a`–`GAME03b`, `GAME15a`–`GAME15c`, `GAME26`–`GAME29b`) | Cycle-accurate device emulation, MMIO register decoding, PPU/APU synthesis, CRT raster/vector beam deflection, ASR-33 teletype serial communication. |
| **Matrix & GPU Compute** | `matrix-ir`, `matrix-runtime` (`MX00`), `gpu-core`, `gpu-runtime` (`G00`–`G09`) | Eras 3, 4 (`GAME30`, `GAME40`, `GAME44`) | Transform pipelines, flight inertia tensor solving, hardware-accelerated 3D polygon rasterization. |
| **Grammar & LangVM Pipeline** | `LANG00`–`LANG84`, `InterpreterIR` (`LANG01`), `vm-core` (`LANG02`), `code/grammars/ecmascript/` | Track 2.5 (`SWF03`, `SWF04`, `SWF05`) | ActionScript 1/2/3 grammar parsing, CST lowering, AVM1/AVM2 bytecode lifting, execution on `vm-core`. |

---

## §6 Mandatory VER-* Verification & Microbenchmark Gates

Every package produced under the `GAME*`, `SWF*`, device simulation, and substrate expansion umbrellas
must satisfy the repository's strict formal verification, resource budgeting, and security isolation criteria:

### 1. Warrant Specifications (`VER04`, `VER04j`, `VER04m`, `VER27`, `VER30`)
- **Single Source of Truth:** Pure simulation logic and hardware device state machines must be specified
  in a formal `.warrant` document (`code/specs/<pkg>.warrant`). Test fixtures and assertions are mechanically
  derived via `warrant-grammar` (`VER04h`).
- **Stateful Objects (`VER04m` `state=explicit`):** Game and hardware state machines model step transitions as:
  ```text
  step(state: State, input: Input) -> Result<(State, Output), Error>
  ```
  Every state transition must satisfy explicit state invariants (`state_inv`).
- **Sabotage Mutation Resistance:** Warrant evidence generation must include negative-control mutations
  (`missing-error`, `off-by-one`, `field-zero`, `frame-leak`) and verify that mutations exit with non-zero failure codes.
- **Layer Contract Linking (`VER29`):** Higher-level game crates link down to foundational contracts
  (e.g., `assume platform: contract.rng.deterministic_bounds`, `assume platform: contract.point2d.aabb_containment`).

### 2. Microbenchmarks & Zero-Allocation Budgets (`VER07`)
- **Hot-Loop Zero Allocation:** Game loop ticks (`step()`, `tick()`, `render_frame()`), device simulation cycles,
  and audio synthesis callbacks must not allocate dynamic heap memory during ongoing play.
- **Deterministic Hard Budgets:** Verified using `install_counting_allocator!()` asserting:
  ```rust
  max_alloc_bytes: Some(0)
  ```
- **Step Count Bounds:** Benchmarks enforce deterministic WIR/VM/hardware step execution bounds (`max_steps`),
  preventing algorithmic stalls across platforms (`VER20`).
- **Timing Robustness:** Wall-clock microbenchmarks report median and normalized Median Absolute Deviation (nMAD),
  ensuring CI runners never fail due to background host load fluctuations.

### 3. Capability Containment Cage (`Spec 13`, `VER08a`, `VER08b`)
- **Zero Ambient Authority:** Pure simulation and device simulation packages declare:
  ```json
  {
    "capabilities": []
  }
  ```
  in `required_capabilities.json`. Pure simulation libraries are physically barred from acquiring handles to
  `fs`, `net`, `proc`, `env`, `ffi`, `time`, `stdin`, `stdout`, `rand`, `thread`, or `ui`.
- **Static Enforcement (`VER08a`):** Static capability analyzers verify zero undeclared capabilities (`CAP001`),
  zero unexcepted `unsafe` blocks or FFI declarations (`CAP002`), and fail closed on opaque macros (`CAP003`).
- **Runtime Sandboxing (`VER08b`):** On Linux (`BUILD_linux` with `RCG_STRICT=1`), test runners activate
  `seccomp-bpf` + `Landlock` + `strace` sandboxing following Phase A initialization, rejecting unauthorized syscalls.

### 4. 26-CWE Weakness Matrix Closure (`VER15`, `VER20`)
Every package must account for all 26 weakness classes from `weakness-registry-v1.json` within its
`assurance_report.json`:
- **CWE-190 (Integer Overflow):** Bounded inputs, checked arithmetic (`checked_add`, `checked_mul`),
  explicit fixed-point saturation, and MMIO address offset wrap protection.
- **CWE-400 / CWE-770 (Resource Exhaustion & Unbounded Loops):** Bounded delta-time accumulators,
  maximum iteration caps on raycasters, bounded cycle steps on hardware simulators, and execution step limits (`FrontendBudgets`).
- **CWE-409 (Decompression Bombs):** Streaming decompression with strict ratio caps (max 50:1) on all SWF assets.
- **CWE-125 / CWE-787 (Out-of-Bounds Read/Write):** Explicit slice range validation on tilemaps,
  sprite tables, vertex buffers, VRAM, OAM, and palette RAM.
- **CWE-843 (Type Confusion & Alignment):** Enforced memory alignment on 16/32-bit MMIO/DMA bus transfers
  and strict numeric typing without implicit coercions.
- **CWE-1321 (Prototype Pollution):** Frozen prototypes and null-prototype property maps in ActionScript runtimes.

---

## §7 Per-Language Common-CVE / CWE Proof Gates (Across 18 Warrant Lanes)

Following the polyglot evidence requirements (`VER04i`, `VER04l`), whenever a `GAME*`, `SWF*`, device simulation,
or substrate package is implemented or ported across the 18 Warrant rollout lanes, it must include dedicated
proof tests addressing the specific security failure modes of that language ecosystem:

| Language Lane | Primary Vulnerability Hazards | Mandatory Per-Package Defense & Proof Gate |
|---|---|---|
| **Rust** | **CWE-190** (release-mode overflow wrap), **CWE-400** (panic unwinding vs typed error), **CAP002** (`unsafe`) | `#![forbid(unsafe_code)]`; explicit `checked_*` arithmetic on all coordinates/resources/MMIO; release-profile CI test run (`cargo test --release`) verifying overflow rejection; `VER08b` seccomp/Landlock sandbox tests. |
| **C / C++** | **CWE-125/787** (OOB memory read/write), **CWE-416/415** (use-after-free/double-free), **CWE-190** (signed overflow UB), **CWE-476** (NULL pointer dereference) | Zero dynamic allocation in `step()` (caller-allocated structs); `-Wall -Wextra -Werror` + ASan/UBSan clean; unsigned promotion before multiply; explicit NULL checks on all public API pointers. |
| **TypeScript / JS** | **CWE-1321** (Prototype Pollution via `__proto__`/`constructor`), **CWE-843** (`Number` $> 2^{53}-1$ truncation, `NaN`/`Infinity` propagation) | `Object.freeze` / null-prototype objects; `Number.isSafeInteger` and `BigInt` guards rejecting `NaN`, `Infinity`, and `-0` at input boundaries; fresh array non-aliasing validation (`VER04i` §5.2). |
| **Python** | **CWE-843** (`bool` subclass of `int`: `True == 1`), **CWE-400** (arbitrary-precision integer DoS), **CWE-94** (`eval`/`pickle`) | Explicit type checking (`type(x) is int`, rejecting `bool`); value range guards ($-2^{63} \le x < 2^{63}$) before arithmetic; static analysis blocking `eval`/`exec`/`pickle`. |
| **Go** | **CWE-190** (silent two's-complement overflow on `int`), **CWE-362** (slice backing array aliasing / data races) | Explicit overflow preconditions before coordinate/resource calculation; defensive copying of slice parameters to prevent caller corruption; `go vet` + `-race` clean. |
| **Java / Kotlin** | **CWE-190** (silent 32-bit `int` overflow; lack of unsigned primitives in Java), **CWE-502** (`ObjectInputStream` deserialization) | `Math.addExact`/`Math.multiplyExact` or `long` promotion with range checks; zero `java.io.Serializable` usage; immutable record/data-class state representations. |
| **C# / F# (.NET)** | **CWE-190** (`unchecked` default integer wraparound), **CWE-502** (binary formatters) | Code compiled within explicit `checked { ... }` blocks; immutable `record` state types; zero binary serialization; memory-safe array slicing (`ReadOnlySpan<T>`). |
| **Ruby / Perl / Lua** | **CWE-843** (implicit string-to-number coercions, Lua float/int conflation), **CWE-1333** (ReDoS in text parsing) | Strict numeric type validation (rejecting string coersion, floats, `nil`/`undef`); linear-time command scanning without backtracking regexes; 64-bit integer masking in Lua/Perl. |
| **Elixir / Haskell / OCaml** | **CWE-400** (Elixir atom table exhaustion, Haskell unbounded thunk space leaks) | Ban conversion of untrusted strings to atoms (`String.to_existing_atom` or binary matching only); strict fields (`!`) on Haskell game state records preventing 60fps thunk buildup. |
| **Swift / Dart** | **CWE-190 / CWE-400** (Swift traps on integer overflow causing process crash; Dart JS-number truncation) | Validate bounds *prior* to arithmetic so invalid orders return typed `Result::Err` rather than trapping; pin 64-bit integer behavior in Dart native tests. |

---

## §8 Decomposed Backlog Table (GAME01..GAME45, SWF00..SWF08, Device Simulators & Substrates)

The following backlog defines the item-by-item decomposition across all eras, device simulators, and substrate expansions:

| Spec ID | Package / Milestone Name | Era / Track | Scope & Deliverables | Prerequisites | Parity Class | Target Tier |
|---|---|---|---|---|---|---|
| **`GAME00`** | `game-engine-roadmap` | All | Master roadmap, architectural split, substrate matrix, verification rules. | None | `spec-only` | N/A |
| **`GAME01a`** | `fixed-point` | Era 0 | Future proposal: `Q16.16`/`Q32.32` arithmetic, sqrt, Cordic trig, and overflow flags. | None | `portable` | Tier 0 |
| **`GAME01b`** | `sim-turn-core` | Era 0 | Future proposal: reusable turn-based command validation, history, and replay harness. | None | `portable` | Tier 0 |
| **`GAME01`** | `hamurabi` | Era 0 | Mainframe turn-based resource-allocation simulation. | `rng` | `portable` | Tier 0 |
| **`GAME02`** | `lunar-lander` | Era 0 | Discrete fixed-point descent simulation, with `paint-vm-ascii` used by the presentation bridge. | None (core), `P2D02` (presentation bridge) | `portable` | Tier 0 |
| **`GAME03`** | `sim-mainframe-bridge` | Era 0 | Future goal: execute original FOCAL and Dartmouth BASIC game programs on PDP-8/GE-225 simulators. Current `GAME03b` scope polls and prints telemetry from modern game packages. | `07aa`, `07g`, `GAME03a`, `GAME03b` | `native-source` | Tier 1 |
| **`GAME03a`** | `sim-teletype-asr33` | Era 0 | Teletype ASR-33 terminal simulator: serial I/O, paper tape, mechanical timing, baud rate limiter. | `format-doc`, `P2D02` | `portable` | Tier 0 |
| **`GAME03b`** | `sim-pdp8-ge225-io-bridge` | Era 0 | I/O bus & teletype register interface connecting `pdp8-simulator` and `ge225-simulator` to terminal I/O. | `07aa`, `07g`, `GAME03a` | `native-source` | Tier 1 |
| **`GAME10`** | `game-clock` | Era 1 | Fixed-timestep accumulator loop, frame-skip cap (spiral-of-death guard), alpha lerp. | None | `portable` | Tier 0 |
| **`GAME11`** | `collision2d` | Era 1 | AABB-AABB, circle-AABB, swept raycast/segment TOI, contact normal calculation. | `G2D00`, `G2D01` | `portable` | Tier 0 |
| **`GAME12`** | `arcade-sfx` | Era 1 | Parametric square-wave pitch sweeps, LFSR white-noise generator, PCM synthesis. | `MUS00`, `MUS04` | `portable` | Tier 1 |
| **`GAME13`** | `game-pong` | Era 1 | Deterministic 2-player Pong state machine, paddle deflection, `PaintScene` output. | `GAME10`, `GAME11`, `P2D00` | `portable` | Tier 0 |
| **`GAME14`** | `game-breakout` | Era 1 | Brick grid data structure, ball reflection, score multiplier, chiptune sound cues. | `GAME11`, `GAME12`, `P2D00` | `portable` | Tier 0 |
| **`GAME15a`** | `sim-discrete-arcade-logic` | Era 1 | Discrete TTL 7400-series logic simulator, sync generator 15.75 kHz/60 Hz, clock divider, paddle potentiometers. | None | `portable` | Tier 0 |
| **`GAME15b`** | `sim-crt-vector-beam` | Era 1 | CRT raster beam deflection, oscilloscope vector display simulator, phosphor decay model, beam blanking. | `P2D00`, `P2D08` | `portable` | Tier 0 |
| **`GAME15c`** | `sim-discrete-audio-timer` | Era 1 | 555-timer, LFSR noise, and analog filter simulation for coin-op audio. | `MUS00`, `MUS04` | `portable` | Tier 1 |
| **`GAME20`** | `palette-indexed` | Era 2 | 2bpp/4bpp/8bpp pixel formats, palette fading, color-0 transparency, blit to `IMG00`. | `IMG00` | `portable` | Tier 0 |
| **`GAME21`** | `tilemap2d` | Era 2 | Multi-layer scrolling background engine, H/V tile flipping, viewport culling. | `GAME20`, `P2D00` | `portable` | Tier 0 |
| **`GAME22`** | `sprite-oam` | Era 2 | 128-sprite hardware OAM table, priority sorting, metasprite animation sequencer. | `G2D01`, `GAME20` | `portable` | Tier 0 |
| **`GAME23`** | `rpg-grid-world` | Era 2 | Grid movement FSM, step encounter counters, warp triggers, dialogue boxes. | `GAME21`, `P2D03` | `portable` | Tier 0 |
| **`GAME24`** | `rpg-battle-engine` | Era 2 | Turn-based creature battle state machine, type effectiveness chart, stat stages. | `rng`, `GAME20` | `portable` | Tier 0 |
| **`GAME25`** | `game-monster-rpg` | Era 2 | Complete playable GBA/Pokémon-style RPG vertical slice with chiptune music. | `GAME22`–`GAME24`, `MUS04` | `portable` | Tier 1 |
| **`GAME26`** | `gba-hardware-bridge` | Era 2 | Memory-mapped I/O register bridge linking `arm7tdmi-simulator` to tile/sprite engine. | `07ax`, `GAME21`, `GAME22` | `native-source` | Tier 2 |
| **`GAME27a`** | `sim-nes-ppu` | Era 2 | Ricoh 2C02 PPU: scanline/cycle timing, nametable mirroring, 64-sprite OAM, sprite 0 hit, palette RAM. | `07j`, `GAME20` | `portable` | Tier 0 |
| **`GAME27b`** | `sim-nes-apu-mappers` | Era 2 | Ricoh 2A03 APU: pulse, triangle, noise, DMC + MMC1/MMC3 cartridge mappers & IRQ timers. | `GAME27a`, `MUS04` | `portable` | Tier 0 |
| **`GAME28a`** | `sim-gameboy-ppu` | Era 2 | LR35902 LCD controller PPU: Mode 0–3 scanline timing, 160x144 LCD, window/BG, OAM sprite priority. | LR35902-compatible execution (`07k` is a related substrate), `GAME20` | `portable` | Tier 0 |
| **`GAME28b`** | `sim-gameboy-apu-mbc` | Era 2 | Game Boy 4-channel APU + MBC1/MBC2/MBC3/MBC5 memory banking controllers & timers. | `GAME28a`, `MUS04` | `portable` | Tier 0 |
| **`GAME29a`** | `sim-gba-ppu-dma` | Era 2 | ARM7TDMI GBA Video Controller: Modes 0–5, affine background/sprite transforms, 4-channel DMA controller, MMIO registers. | `07ax`, `GAME20`, `GAME22` | `portable` | Tier 0 |
| **`GAME29b`** | `sim-gba-audio-timer` | Era 2 | GBA DirectSound FIFO A/B, legacy sound channels, cascading timers, sound DMA mixing. | `GAME29a`, `MUS04` | `portable` | Tier 1 |
| **`SWF00`** | `flash-runtime-roadmap` | Track 2.5 | Master spec for SWF v1–v10 & ActionScript 1/2/3 execution on LangVM. | None | `spec-only` | N/A |
| **`SWF01`** | `swf-container` | Track 2.5 | Zero-dependency binary parser for `.swf` headers, RECTs, and tags (`FWS`/`CWS`/`ZWS`). | `IC04` (deflate) | `portable` | Tier 0 |
| **`SWF02`** | `swf-shape-to-paint` | Track 2.5 | SWF dual-fill Bézier shape lowering to `P2D00` `PaintPath` and `PaintGradient`. | `G2D02`, `P2D00`, `P2D08` | `portable` | Tier 0 |
| **`SWF03`** | `actionscript-compiler` | Track 2.5 | Versioned AS1/AS2/AS3 grammars, AST parser, and `InterpreterIR` compiler. | `LANG84`, `InterpreterIR` | `portable` | Tier 1 |
| **`SWF04`** | `avm-bytecode-lifter` | Track 2.5 | Lifter translating AVM1 actions and AVM2 ABC bytecode into `InterpreterIR` modules. | `LANG01`, `LANG83` | `portable` | Tier 0 |
| **`SWF05`** | `flash-display-list` | Track 2.5 | Flash Timeline, MovieClip, SimpleButton, depth sorting, `PaintVM.patch()` scene diff. | `SWF02`, `P2D01` | `portable` | Tier 1 |
| **`SWF06`** | `flash-audio-bridge` | Track 2.5 | SWF audio tag decoder (PCM/ADPCM) and ActionScript `Sound` bridge to `MUS02`/`MUS06`. | `MUS01`, `MUS02`, `MUS06` | `portable` | Tier 1 |
| **`SWF07`** | `flash-player-host` | Track 2.5 | Integrated runtime shell bridging container, VM, display list, and `UI11` window. | `SWF01`–`SWF06`, `UI11` | `native-source` | Tier 1 |
| **`SWF08`** | `swf-conformance-corpus` | Track 2.5 | Conformance test suite of synthetic `.swf` binaries with exact frame `PaintScene` digests. | `SWF07` | `portable` | Tier 0 |
| **`GAME30`** | `math3d` | Era 3 | Fixed-point and `f64` `Vec3`, `Vec4`, `Mat4` (projection/view), and `Quat` (SLERP). | `fixed-point` | `portable` | Tier 0 |
| **`GAME31`** | `raycast-dda` | Era 3 | 2.5D grid DDA wall raycaster, floor/ceiling plane renderer, billboard sprites. | `GAME30`, `IMG00` | `portable` | Tier 0 |
| **`GAME32`** | `bsp-portal25d` | Era 3 | 2D sector BSP tree generator, 1D portal occlusion clipper, dynamic vis-lists. | `GAME30`, `point2d` | `portable` | Tier 1 |
| **`GAME33`** | `raster3d-soft` | Era 3 | Software triangle rasterizer: homogeneous clipping, barycentrics, Z-buffer, 1/z UV mapping. | `GAME30`, `IMG00` | `portable` | Tier 0 |
| **`GAME34`** | `game-labyrinth3d` | Era 3 | Playable 2.5D/3D dungeon crawler with deterministic frame digests. | `GAME31`, `GAME33`, `UI11` | `portable` | Tier 1 |
| **`GAME35`** | `bsp3d-quake-c-bridge` | Era 3 | 3D BSP traversal and QuakeC bytecode execution bridge on `InterpreterIR`. | `GAME33`, `LANG01` | `portable` | Tier 2 |
| **`GAME40`** | `rigidbody-6dof` | Era 4 | 6-DOF rigid-body dynamics integrator (RK4/Verlet), inertia tensor, quaternion attitude. | `GAME30`, `PHY` | `portable` | Tier 0 |
| **`GAME41`** | `aero-flight-model` | Era 4 | ISA atmospheric density model, airfoil lift/drag/moment polar curves, control surfaces. | `GAME40` | `portable` | Tier 0 |
| **`GAME42`** | `terrain-lod` | Era 4 | Heightfield synthesis, quadtree/ROAM continuous mesh decimation, runway placement. | `GAME30`, `matrix-ir` | `portable` | Tier 1 |
| **`GAME43`** | `cockpit-hud-paint` | Era 4 | Primary Flight Display (attitude, airspeed, altimeter, pitch ladder) via `P2D00`. | `P2D00`, `P2D01` | `portable` | Tier 0 |
| **`GAME44`** | `render3d-gpu-bridge` | Era 4 | Hardware 3D pipeline bridging to `gpu-runtime` (`G00`–`G09`) with fallback to `raster3d-soft`. | `GAME33`, `G00` | `native-source` | Tier 1 |
| **`GAME45`** | `game-flight-sim` | Era 4 | Complete playable 6-DOF flight simulator with black-box flight recorder and replay. | `GAME40`–`GAME44`, `UI11` | `portable` | Tier 1 |
| **`P2D09`** | `paint-batch-tiles` | Substrate | Retained-mode sprite/tile batching, metasprite layout, atlas indexing, `PaintVM.patch()` delta compression. | `P2D00`, `P2D01` | `portable` | Tier 0 |
| **`P2D10`** | `paint-palette-blend` | Substrate | Indexed-color scanline blits, hardware blend modes (Add/Multiply/Screen/AlphaMask), Porter-Duff compositing. | `P2D08`, `P2D09` | `portable` | Tier 0 |
| **`UI11b`** | `window-cadence-input` | Substrate | High-frequency 60/120/240 Hz vsync frame scheduling, sub-millisecond gamepad/input polling, deterministic input streams. | `UI11` | `native-source` | Tier 0 |
| **`MUS08`** | `music-analysis-and-transcription` | Substrate | See [`MUS08`](MUS08-music-analysis-and-transcription.md) for the analysis and transcription specification. | As specified in `MUS08` | `portable` | Tier 0 |
| **Warrant** | `audio-mixer-realtime` | Substrate | Real-time multi-channel PCM/chip audio mixer and ring-buffer device sinks. Its [`audio-mixer-realtime.warrant`](audio-mixer-realtime.warrant) covers formal arithmetic and sample-rate operations. | `pcm-audio`, `audio-device-sink` | `portable` | Tier 0 |

---

## §9 Execution Sequence & Rollout Plan

To maintain rigorous build-tool selection boundaries and preserve rapid CI turnaround, development
proceeds in strictly staged milestones:

### Milestone 1: Master Architectural Roadmaps (Current Worktree: `feat/game00-device-sim-paintvm`)
- Author normative roadmap specifications:
  - `code/specs/GAME00-historical-game-engine-roadmap.md` (this document)
  - `code/specs/SWF00-flash-runtime-roadmap.md` (Flash on LangVM deep dive)
  - `code/specs/GAME01-hamurabi-mainframe-sim.md` (Hamurabi normative state machine and formulas)
  - `code/specs/GAME02-lunar-lander-fixed-point-sim.md` (Lunar Lander fixed-point physics and ASCII telemetry)
- Verify that `build-tool --diff-base origin/main` identifies 0 affected code packages (instant PR validation).

### Milestone 2: Rust Reference Core & Warrant Verification (`game01-rust-warrant`)
- Land `code/specs/hamurabi.warrant` with formal conservation theorems and failure arm coverage.
- Land `code/packages/rust/hamurabi` with 95%+ coverage, `capabilities: []`, `VER07` zero-alloc benchmarks,
  and `VER08b` runtime sandboxing tests.
- Generate canonical cross-language fixture corpus `code/specs/fixtures/hamurabi-warrant-v1/`.

### Milestone 3: Polyglot Rollout Across 18 Warrant Lanes
- **Wave 1:** Python (`WD-101`), TypeScript (`WD-102`), Go.
- **Wave 2:** Ruby, Elixir, Lua, Perl.
- **Wave 3:** C, C++, Java, Kotlin, C#, F#, Dart, Swift, Haskell, OCaml.
- Register all consumers in `code/programs/go/build-tool/repository_inputs.go`.

### Milestone 4: Substrate Upstream Expansion (`P2D09`, `P2D10`, `UI11b`, `audio-mixer-realtime`)
- Land retained-mode tile/sprite batching (`P2D09`) and palette-indexed scanline blits with hardware blend modes (`P2D10`).
- Land high-frequency frame cadence and deterministic gamepad input streams (`UI11b`).
- Integrate the `audio-mixer-realtime` package with game audio paths. Its Warrant covers arithmetic and sample-rate operations.

### Milestone 5: Functional & Gate-Level Device Simulators (Eras 0–2)
- Land ASR-33 teletype and PDP-8/GE-225 CPU I/O bridges (`GAME03a`, `GAME03b`).
- Land discrete TTL logic, CRT raster/vector beam deflection, and analog audio timer (`GAME15a`, `GAME15b`, `GAME15c`).
- Land NES, Game Boy, and GBA PPU/APU/DMA/mapper bridges (`GAME27a`, `GAME27b`, `GAME28a`, `GAME28b`, `GAME29a`, `GAME29b`).

### Milestone 6: High-Level Game Engines & Playable Vertical Slices
- Land reference arcade titles (`GAME10`–`GAME14`).
- Land 2D RPG world, battle engine, and monster RPG vertical slice (`GAME20`–`GAME25`).
- Land Flash runtime on LangVM (`SWF00`–`SWF08`).
- Land 2.5D raycaster and software 3D rasterizer (`GAME30`–`GAME35`).
- Land 6-DOF flight simulation and hardware 3D renderer (`GAME40`–`GAME45`).

---

## §10 References

### Internal Specifications & Substrates
- **Warrant & Verification:** [`VER00`](VER00-verified-shipping-program.md), [`VER04`](VER04-warrant-language.md), [`VER04i`](VER04i-polyglot-warrant-evidence.md), [`VER04j`](VER04j-warrant-v1-extensions.md), [`VER04m`](VER04m-warrant-stateful-objects.md), [`VER07`](VER07-microbenchmarks.md), [`VER08a`](VER08a-rust-capability-analyzer.md), [`VER08b`](VER08b-runtime-capability-conformance.md), [`VER15`](VER15-weakness-registry.md), [`VER20`](VER20-resource-timing-proofs.md), [`VER29`](VER29-layer-contract-linking.md).
- **2D Vector & Rasterization:** [`P2D00`](P2D00-paint-instructions.md), [`P2D01`](P2D01-paint-vm.md), [`P2D02`](P2D02-paint-vm-ascii.md), [`P2D08`](P2D08-path-raster.md), [`G2D00`](G2D00-point2d.md), [`G2D01`](G2D01-affine2d.md), [`G2D02`](G2D02-bezier2d.md).
- **Substrate Expansions:** `P2D09` (`paint-batch-tiles`), `P2D10` (`paint-palette-blend`), `UI11b` (`window-cadence-input`), and `audio-mixer-realtime` (Warrant: `audio-mixer-realtime.warrant`).
- **Windowing & Events:** [`UI11`](UI11-window-core.md), [`UI12`](UI12-window-backends.md).
- **Virtual Machines & Grammars:** [`LANG01`](LANG01-interpreter-ir.md), [`LANG02`](LANG02-vm-core.md), [`LANG84`](LANG84-grammar-driven-versioned-frontends.md), [`versioned-ecmascript-typescript-grammars.md`](versioned-ecmascript-typescript-grammars.md).
- **Audio & Sinks:** [`MUS00`](MUS00-note-frequency.md), [`MUS01`](MUS01-note-to-sound-signal-chain.md), [`MUS04`](MUS04-portable-music-machine.md), [`MUS07`](MUS07-unpitched-percussion.md), [`MUS08`](MUS08-music-analysis-and-transcription.md), [`MUS02`](MUS02-audio-device-sink.md), [`MUS06`](MUS06-web-audio-sink.md), and the separate [`audio-mixer-realtime.warrant`](audio-mixer-realtime.warrant).
- **Hardware Simulators:** [`CPU-SIMULATOR-ROADMAP.md`](CPU-SIMULATOR-ROADMAP.md) (`07aa` PDP-8, `07g` GE-225, [`07j` MOS 6502](07j-mos6502-simulator.md), [`07k` Z80](07k-z80-simulator.md), `07ax` ARM7TDMI).

### Historical Literature & Primary Sources
1. Ahl, David H. (1973). *101 BASIC Computer Games*. Digital Equipment Corporation.
2. Storer, Jim (1969). *Lunar Landing Game*. FOCAL-69, Lexington High School / DEC.
3. Dyba, Richard H. (1964). *The Sumerian Game: An Early Management Simulation*. Board of Cooperative Educational Services.
4. Alcorn, Allan (1972). *Pong*. Atari Inc. Engineering Schematics (Discrete TTL 7400-series).
5. Digital Equipment Corporation (1970). *PDP-8/e, PDP-8/m, & PDP-8/a Small Computer Handbook & Peripheral Device Specifications*.
6. General Electric Information Systems (1963). *GE-225 Information Processing System Reference Manual*.
7. MOS Technology Inc. (1976). *MCS6500 Microcomputer Family Hardware Manual*.
8. Nintendo Co., Ltd. (1985). *Family Computer / Nintendo Entertainment System Architecture & Ricoh 2C02 / 2A03 Technical Specifications*.
9. Nintendo Co., Ltd. (1989). *Game Boy Programming Manual & LR35902 LCD/Sound Controller Specification*.
10. Nintendo Co., Ltd. (2001). *Game Boy Advance Architecture Reference Manual (ARM7TDMI MMIO, PPU Modes, DMA)*.
11. Russell, Stuart & Norvig, Peter. *A Modern Approach to State Estimation and Control*.
12. Abrash, Michael (1997). *Michael Abrash's Graphics Programming Black Book Special Edition*. The Coriolis Group.
13. Adobe Systems Inc. (2008). *ActionScript 3.0 Language and Syntax Reference* & *SWF File Format Specification Version 10*.
14. Etkin, Bernard & Reid, Lloyd Duff (1996). *Dynamics of Flight: Stability and Control*. John Wiley & Sons.
