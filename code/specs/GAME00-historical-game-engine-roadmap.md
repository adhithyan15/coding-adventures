# GAME00 — Historical Game Engine Roadmap (Mainframe, Arcade, Retro Tile, Flash on LangVM, 2.5D/3D & Flight Sim)

## Status

Roadmap / master spec. Surveys the technical and historical evolution of real-time computer
entertainment, interactive simulation, and game engines across six eras (1950s mainframe
simulations through hardware-accelerated 6-DOF flight simulation, including a dedicated
ActionScript/SWF Flash runtime on LangVM). It establishes a rigorous, layered reconstruction
strategy that reuses this repository's existing mathematical, geometric, vector rendering,
audio synthesis, CPU simulation, and virtual machine substrates while enforcing repository-wide
`VER-*` verification, microbenchmarking, capability containment, and per-language common-CVE/CWE
proof gates.

---

## §1 Why — Historical Reconstruction and the Two Principles

This repository's identity is **faithful historical reconstruction**: bringing back the seminal
systems that shaped computing so that developers and researchers can study, execute, and verify
them within a modern, literate, and safe codebase. Video games and interactive simulations were
historically the primary crucible for real-time graphics algorithms, fixed-point and floating-point
physics integration, audio synthesis engines, domain-specific virtual machines, and deterministic
network synchronization.

Two foundational architectural principles govern every item across all eras in this roadmap:

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

---

## §2 Progression Across Six Historical Eras

The roadmap progresses chronologically across six distinct historical eras, each introducing
fundamental advancements in simulation architecture, coordinate mathematics, rendering techniques,
and runtime execution:

```text
Era 0 (1950s–1960s): Mainframe & Teletype Turn-Based Simulations (Hamurabi, Lunar Lander)
  └── Pure state step + fixed-point math + rng + format-doc / paint-vm-ascii telemetry
Era 1 (1970s): Early Arcade 2D Vector & Raster (Pong, Breakout, Asteroids)
  └── Fixed-timestep loop + G2D00-G2D03 collision + P2D00 PaintScene + MUS00-07 chiptune
Era 2 (1980s–Early 2000s): 8-Bit/16-Bit/GBA Tile, Sprite & RPG Engine (Pokemon-style)
  └── Palette/Tilemap/Sprite OAM PPU + overworld/battle FSM + Z80/6502/ARM7TDMI bridge
Track 2.5 (Late 1990s–2000s): Unmodified Flash (.swf) & ActionScript on LangVM
  └── SWF container parser + AS1/AS2/AS3 on LANG84/InterpreterIR/vm-core + DisplayList -> P2D00
Era 3 (Early 1990s): 2.5D Raycasters & Software 3D Rasterizers (Wolf3D, Doom, Quake)
  └── Fixed-point DDA raycaster + BSP/portal + Z-buffered triangle rasterizer -> IMG00 PixelContainer
Era 4 (Mid 1990s–2000s+): Hardware 3D & 6-DOF Flight Simulator
  └── 6-DOF rigid-body/aerodynamics (PHY + matrix-ir) + terrain LOD + cockpit HUD + G00-G09/wgpu
```

### Era 0 (1950s–1960s): Mainframe & Teletype Turn-Based Simulations (`GAME01`–`GAME03`)
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
- **Substrate Primitives:**
  - `fixed-point` (`GAME01a`): Deterministic `Q16.16` and `Q32.32` fixed-point arithmetic with
    overflow detection, eliminating floating-point non-determinism across polyglot targets.
  - `sim-turn-core` (`GAME01b`): Generic turn-based execution harness providing command validation,
    history log rollbacks, and deterministic replay transcripts.
  - `game-hamurabi` (`GAME01c`): Complete 10-year Sumerian economic simulation running on pure integers.
  - `game-lunar-lander` (`GAME01d`): Discrete Euler-Verlet rocket descent with fuel consumption,
    gravitational acceleration, and trajectory printouts via `paint-vm-ascii` (`P2D02`).
  - Historical CPU Bridges (`GAME03`): Running the original Dartmouth BASIC and FOCAL source files
    on `pdp8-simulator` (`07aa`), `pdp10-simulator` (`07ab`), and `ge225-simulator` (`07g`).

### Era 1 (1970s): Early Arcade, Fixed-Timestep Loop & 2D Collision (`GAME10`–`GAME14`)
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
- **Substrate Primitives:**
  - `game-clock` (`GAME10`): Deterministic fixed-step clock with frame-skip protection.
  - `collision2d` (`GAME11`): High-precision 2D intersection queries and reflection physics built on `point2d` (`G2D00`) and `affine2d` (`G2D01`).
  - `arcade-sfx` (`GAME12`): Parametric sound generator emitting raw PCM buffers into `MUS04`/`MUS07`.
  - `game-pong` & `game-breakout` (`GAME13`, `GAME14`): Reference arcade titles emitting `PaintScene` (`P2D00`) with stable IDs for efficient `PaintVM.patch()` diffing.

### Era 2 (1980s–Early 2000s): 8-Bit/16-Bit/GBA Tile, Sprite & RPG Engine (`GAME20`–`GAME26`)
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
- **Substrate Primitives:**
  - `palette-indexed` (`GAME20`): Indexed color formats and palette transition engines.
  - `tilemap2d` (`GAME21`): Multi-plane scrolling tile engine with tile-attribute collision maps.
  - `sprite-oam` (`GAME22`): Hardware-accurate sprite attribute manager and metasprite animator.
  - `rpg-grid-world` (`GAME23`): Deterministic overworld navigation, map loading, and dialogue parsing.
  - `rpg-battle-engine` (`GAME24`): Turn-based battle state machine with deterministic damage calculations.
  - `game-monster-rpg` (`GAME25`): A complete, playable top-down monster-collecting RPG vertical slice.
  - `gba-hardware-bridge` (`GAME26`): Optional bridge interfacing `arm7tdmi-simulator` (`07ax`) memory-mapped
    registers (`0x04000000`) with our native tile and sprite pipeline.

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

## §3 Substrate Reuse Matrix

The following table demonstrates how each era maps onto and reuses existing packages across this repository:

| Substrate Domain | Existing Packages | Reused In Eras / Packages | How Reused |
|---|---|---|---|
| **Deterministic Randomness** | `rng` (`PCG32`, `LCG`, `Xorshift64`) | All Eras (`GAME01`–`GAME45`, `SWF05`) | Explicit PRNG state carried in `GameState`; rejection-sampled range distributions. |
| **Fixed-Point & Scalars** | `numeric-tower`, `fixed-point` (`GAME01a`) | Eras 0, 1, 2, 3 (`GAME01`, `GAME11`, `GAME31`) | `Q16.16`/`Q32.32` math ensuring zero cross-language float divergence. |
| **2D Geometry & Transforms** | `point2d` (`G2D00`), `affine2d` (`G2D01`), `bezier2d` (`G2D02`), `arc2d` (`G2D03`), `trig` (`PHY00`) | Eras 1, 2, 2.5 (`GAME11`, `GAME22`, `SWF02`) | AABB collision, sprite transform matrices, Flash vector curves, trigonometric trajectory calculation. |
| **Vector IR & Diffing** | `P2D00` (`PaintInstructions`), `P2D01` (`PaintVM`), `P2D08` (`path-raster`) | Eras 1, 2, 2.5, 4 (`GAME13`, `GAME21`, `SWF02`, `GAME43`) | Declarative scene graphs; retained-mode scene diffing via `PaintVM.patch()`; scanline rasterization. |
| **ASCII / Doc Output** | `P2D02` (`paint-vm-ascii`), `P2D03`/`P2D04` (`format-doc`) | Era 0, 1 (`GAME01`, `GAME02`, `GAME13`) | Terminal game rendering, telemetry plots, dialogue boxes, retro teletype simulation. |
| **Pixel Containers & Codecs** | `IMG00` (`pixel-container`), `IMG03`–`IMG05` (compositing), `IC01`–`IC18` (codecs) | Eras 2, 3, 4 (`GAME20`, `GAME33`, `SWF01`) | Paletted framebuffers, software 3D Z-buffers, textures, embedded Flash bitmap decoding (PNG, JPEG, GIF). |
| **Windowing & Events** | `event-loop`, `UI11` (`window-core`), `window-appkit`, `window-win32`, `window-canvas` | All Eras (Host Shell) | OS window creation, input polling (keyboard, mouse, gamepad), canvas presentation. |
| **Audio & Music Synthesis** | `MUS00`–`MUS07` (`note-frequency`, `music-machine`, `unpitched-percussion`, `audio-device-sink`, `web-audio-sink`) | Eras 1, 2, 2.5, 4 (`GAME12`, `GAME25`, `SWF06`, `GAME45`) | Parametric arcade audio, retro chiptune background tracks, SWF streaming audio playback, turbine noise. |
| **CPU Simulators** | `07aa` (PDP-8), `07g` (GE-225), `07am` (6502), `07an` (Z80), `07ax` (ARM7TDMI) | Eras 0, 2 (`GAME03`, `GAME26`) | Executing original game binaries and interfacing memory-mapped hardware registers. |
| **Matrix & GPU Compute** | `matrix-ir`, `matrix-runtime` (`MX00`), `gpu-core`, `gpu-runtime` (`G00`–`G09`) | Eras 3, 4 (`GAME30`, `GAME40`, `GAME44`) | Transform pipelines, flight inertia tensor solving, hardware-accelerated 3D polygon rasterization. |
| **Grammar & LangVM Pipeline** | `LANG00`–`LANG84`, `InterpreterIR` (`LANG01`), `vm-core` (`LANG02`), `code/grammars/ecmascript/` | Track 2.5 (`SWF03`, `SWF04`, `SWF05`) | ActionScript 1/2/3 grammar parsing, CST lowering, AVM1/AVM2 bytecode lifting, execution on `vm-core`. |

---

## §4 Mandatory VER-* Verification & Microbenchmark Gates

Every package produced under the `GAME*` and `SWF*` umbrellas must satisfy the repository's
strict formal verification, resource budgeting, and security isolation criteria:

### 1. Warrant Specifications (`VER04`, `VER04j`, `VER04m`, `VER27`, `VER30`)
- **Single Source of Truth:** Pure simulation logic must be specified in a formal `.warrant` document
  (`code/specs/<pkg>.warrant`). Test fixtures and assertions are mechanically derived via `warrant-grammar` (`VER04h`).
- **Stateful Objects (`VER04m` `state=explicit`):** Game state machines model step transitions as:
  ```text
  step(state: GameState, input: InputFrame) -> Result<(GameState, FrameOutput), GameError>
  ```
  Every state transition must satisfy explicit state invariants (`state_inv`).
- **Sabotage Mutation Resistance:** Warrant evidence generation must include negative-control mutations
  (`missing-error`, `off-by-one`, `field-zero`, `frame-leak`) and verify that mutations exit with non-zero failure codes.
- **Layer Contract Linking (`VER29`):** Higher-level game crates link down to foundational contracts
  (e.g., `assume platform: contract.rng.deterministic_bounds`, `assume platform: contract.point2d.aabb_containment`).

### 2. Microbenchmarks & Zero-Allocation Budgets (`VER07`)
- **Hot-Loop Zero Allocation:** Game loop ticks (`step()`, `tick()`, `render_frame()`) must not allocate
  dynamic heap memory during ongoing play.
- **Deterministic Hard Budgets:** Verified using `install_counting_allocator!()` asserting:
  ```rust
  max_alloc_bytes: Some(0)
  ```
- **Step Count Bounds:** Benchmarks enforce deterministic WIR/VM step execution bounds (`max_steps`),
  preventing algorithmic stalls across platforms.
- **Timing Robustness:** Wall-clock microbenchmarks report median and normalized Median Absolute Deviation (nMAD),
  ensuring CI runners never fail due to background host load fluctuations.

### 3. Capability Containment Cage (`Spec 13`, `VER08a`, `VER08b`)
- **Zero Ambient Authority:** Simulation packages declare:
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
  and explicit fixed-point saturation.
- **CWE-400 / CWE-770 (Resource Exhaustion & Unbounded Loops):** Bounded delta-time accumulators,
  maximum iteration caps on raycasters, and execution step limits (`FrontendBudgets`).
- **CWE-409 (Decompression Bombs):** Streaming decompression with strict ratio caps (max 50:1) on all SWF assets.
- **CWE-125 / CWE-787 (Out-of-Bounds Read/Write):** Explicit slice range validation on tilemaps,
  sprite tables, and vertex buffers.
- **CWE-1321 (Prototype Pollution):** Frozen prototypes and null-prototype property maps in ActionScript runtimes.

---

## §5 Per-Language Common-CVE / CWE Proof Gates (Across 18 Warrant Lanes)

Following the polyglot evidence requirements (`VER04i`, `VER04l`), whenever a `GAME*` package is implemented
or ported across the 18 Warrant rollout lanes, it must include dedicated proof tests addressing the
specific security failure modes of that language ecosystem:

| Language Lane | Primary Vulnerability Hazards | Mandatory Per-Package Defense & Proof Gate |
|---|---|---|
| **Rust** | **CWE-190** (release-mode overflow wrap), **CWE-400** (panic unwinding vs typed error), **CAP002** (`unsafe`) | `#![forbid(unsafe_code)]`; explicit `checked_*` arithmetic on all coordinates/resources; release-profile CI test run (`cargo test --release`) verifying overflow rejection; `VER08b` seccomp/Landlock sandbox tests. |
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

## §6 Decomposed Backlog Table (GAME01..GAME45, SWF00..SWF08)

The following backlog defines the item-by-item decomposition across all six eras:

| Spec ID | Package / Milestone Name | Era / Track | Scope & Deliverables | Prerequisites | Parity Class | Target Tier |
|---|---|---|---|---|---|---|
| **`GAME00`** | `game-engine-roadmap` | All | Master roadmap, architectural split, substrate matrix, verification rules. | None | `spec-only` | N/A |
| **`GAME01a`** | `fixed-point` | Era 0 | `Q16.16` and `Q32.32` deterministic arithmetic, sqrt, Cordic trig, overflow flags. | None | `portable` | Tier 0 |
| **`GAME01b`** | `sim-turn-core` | Era 0 | Turn-based state machine traits, command parser, replay history log. | None | `portable` | Tier 0 |
| **`GAME01c`** | `game-hamurabi` | Era 0 | 10-year Sumerian economic simulation, integer state, plague/rats/harvest formulas. | `rng`, `fixed-point` | `portable` | Tier 0 |
| **`GAME01d`** | `game-lunar-lander` | Era 0 | Discrete Euler-Verlet rocket descent physics, fuel consumption, terminal telemetry. | `fixed-point`, `P2D02` | `portable` | Tier 0 |
| **`GAME02`** | `lunar-lander-sim` | Era 0 | Full Lunar Lander milestone spec: 1D/2D vectors, gravity, burn rates, telemetry strip. | `GAME01d` | `spec-only` | N/A |
| **`GAME03`** | `sim-mainframe-bridge` | Era 0 | Host bridges executing original FOCAL/BASIC code on PDP-8/GE-225 CPU simulators. | `07aa`, `07g`, `PL05` | `native-source` | Tier 1 |
| **`GAME10`** | `game-clock` | Era 1 | Fixed-timestep accumulator loop, frame-skip cap (spiral-of-death guard), alpha lerp. | None | `portable` | Tier 0 |
| **`GAME11`** | `collision2d` | Era 1 | AABB-AABB, circle-AABB, swept raycast/segment TOI, contact normal calculation. | `G2D00`, `G2D01` | `portable` | Tier 0 |
| **`GAME12`** | `arcade-sfx` | Era 1 | Parametric square-wave pitch sweeps, LFSR white-noise generator, PCM synthesis. | `MUS00`, `MUS04` | `portable` | Tier 1 |
| **`GAME13`** | `game-pong` | Era 1 | Deterministic 2-player Pong state machine, paddle deflection, `PaintScene` output. | `GAME10`, `GAME11`, `P2D00` | `portable` | Tier 0 |
| **`GAME14`** | `game-breakout` | Era 1 | Brick grid data structure, ball reflection, score multiplier, chiptune sound cues. | `GAME11`, `GAME12`, `P2D00` | `portable` | Tier 0 |
| **`GAME20`** | `palette-indexed` | Era 2 | 2bpp/4bpp/8bpp pixel formats, palette fading, color-0 transparency, blit to `IMG00`. | `IMG00` | `portable` | Tier 0 |
| **`GAME21`** | `tilemap2d` | Era 2 | Multi-layer scrolling background engine, H/V tile flipping, viewport culling. | `GAME20`, `P2D00` | `portable` | Tier 0 |
| **`GAME22`** | `sprite-oam` | Era 2 | 128-sprite hardware OAM table, priority sorting, metasprite animation sequencer. | `G2D01`, `GAME20` | `portable` | Tier 0 |
| **`GAME23`** | `rpg-grid-world` | Era 2 | Grid movement FSM, step encounter counters, warp triggers, dialogue boxes. | `GAME21`, `P2D03` | `portable` | Tier 0 |
| **`GAME24`** | `rpg-battle-engine` | Era 2 | Turn-based creature battle state machine, type effectiveness chart, stat stages. | `rng`, `GAME20` | `portable` | Tier 0 |
| **`GAME25`** | `game-monster-rpg` | Era 2 | Complete playable GBA/Pokémon-style RPG vertical slice with chiptune music. | `GAME22`–`GAME24`, `MUS04` | `portable` | Tier 1 |
| **`GAME26`** | `gba-hardware-bridge` | Era 2 | Memory-mapped I/O register bridge linking `arm7tdmi-simulator` to tile/sprite engine. | `07ax`, `GAME21`, `GAME22` | `native-source` | Tier 2 |
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

---

## §7 Execution Sequence & Rollout Plan

To maintain rigorous build-tool selection boundaries and preserve rapid CI turnaround, development
proceeds in strictly staged milestones:

### Milestone 1: Master Architectural Roadmaps (Current Worktree: `feat/game00-roadmap`)
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

---

## §8 References

### Internal Specifications & Substrates
- **Warrant & Verification:** [`VER00`](VER00-verified-shipping-program.md), [`VER04`](VER04-warrant-language.md), [`VER04i`](VER04i-polyglot-warrant-evidence.md), [`VER04j`](VER04j-warrant-v1-extensions.md), [`VER04m`](VER04m-warrant-stateful-objects.md), [`VER07`](VER07-microbenchmarks.md), [`VER08a`](VER08a-rust-capability-analyzer.md), [`VER08b`](VER08b-runtime-capability-conformance.md), [`VER15`](VER15-weakness-registry.md), [`VER20`](VER20-resource-timing-proofs.md), [`VER29`](VER29-layer-contract-linking.md).
- **2D Vector & Rasterization:** [`P2D00`](P2D00-paint-instructions.md), [`P2D01`](P2D01-paint-vm.md), [`P2D02`](P2D02-paint-vm-ascii.md), [`P2D08`](P2D08-path-raster.md), [`G2D00`](G2D00-point2d.md), [`G2D01`](G2D01-affine2d.md), [`G2D02`](G2D02-bezier2d.md).
- **Virtual Machines & Grammars:** [`LANG01`](LANG01-interpreter-ir.md), [`LANG02`](LANG02-vm-core.md), [`LANG84`](LANG84-grammar-driven-versioned-frontends.md), [`versioned-ecmascript-typescript-grammars.md`](versioned-ecmascript-typescript-grammars.md).
- **Audio & Sinks:** [`MUS00`](MUS00-note-frequency.md), [`MUS01`](MUS01-note-to-sound-signal-chain.md), [`MUS04`](MUS04-portable-music-machine.md), [`MUS07`](MUS07-unpitched-percussion.md), [`MUS02`](MUS02-audio-device-sink.md), [`MUS06`](MUS06-web-audio-sink.md).
- **Hardware Simulators:** [`CPU-SIMULATOR-ROADMAP.md`](CPU-SIMULATOR-ROADMAP.md) (`07aa` PDP-8, `07g` GE-225, `07ax` ARM7TDMI).

### Historical Literature & Primary Sources
1. Ahl, David H. (1973). *101 BASIC Computer Games*. Digital Equipment Corporation.
2. Storer, Jim (1969). *Lunar Landing Game*. FOCAL-69, Lexington High School / DEC.
3. Dyba, Richard H. (1964). *The Sumerian Game: An Early Management Simulation*. Board of Cooperative Educational Services.
4. Alcorn, Allan (1972). *Pong*. Atari Inc.
5. Russell, Stuart & Norvig, Peter. *A Modern Approach to State Estimation and Control*.
6. Abrash, Michael (1997). *Michael Abrash's Graphics Programming Black Book Special Edition*. The Coriolis Group.
7. Adobe Systems Inc. (2008). *ActionScript 3.0 Language and Syntax Reference* & *SWF File Format Specification Version 10*.
8. Etkin, Bernard & Reid, Lloyd Duff (1996). *Dynamics of Flight: Stability and Control*. John Wiley & Sons.
