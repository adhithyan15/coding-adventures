# SWF00 — Flash Runtime Roadmap (ActionScript 1/2/3 & AVM1/AVM2 on LangVM)

## Status

Roadmap / master spec. Establishes the architecture, package decomposition, verification strategy,
and security defense matrix for executing unmodified historical Macromedia/Adobe Flash (`.swf`,
versions 1 through 10) files. The runtime reconstructs the Flash vector animation, display list,
audio stream, and ActionScript scripting systems by reusing this repository's existing
vector rendering (`P2D00`–`P2D10`), geometry (`G2D00`–`G2D03`), audio synthesis (`MUS00`–`MUS07`),
windowing (`UI11`), and virtual machine pipeline (`LANG00`–`LANG84`, `InterpreterIR`, `vm-core`).

---

## §1 Why Flash on LangVM — The Architectural Thesis

From 1996 through 2020, Flash was the predominant medium for independent web gaming, interactive
animation, and digital art. Tens of thousands of culturally significant games (*Alien Hominid*,
*Fancy Pants Adventures*, *Crush the Castle*, *Bloons*, *The Impossible Quiz*) were authored in
Flash. Following Flash Player's official end-of-life in December 2020, preserving this heritage
became an urgent archival priority.

Previous preservation attempts (Gnash, Lightspark, Ruffle) constructed bespoke monolithic engines
in C++ or Rust, often binding custom vector rasterizers, standalone garbage-collected heaps, and
bespoke JIT engines together. In contrast, this repository treats Flash **not as an isolated island,
but as a premier integration consumer of our foundational computing substrates**:

1. **Vector Art Lowers to `PaintInstructions` (`P2D00`):** Flash vector shapes (composed of
   quadratic Bézier curves, line styles, dual-sided fill contours, and linear/radial gradients)
   map directly to `point2d` (`G2D00`), `affine2d` (`G2D01`), `bezier2d` (`G2D02`), and `path-raster` (`P2D08`),
   emitting declarative `PaintScene` trees. Every `PaintVM` backend (ASCII terminal, software scanline,
   Metal, Direct2D, Cairo, Skia, WGPU, Canvas) gains instant Flash rendering support.
2. **DisplayList Diffing via `PaintVM.patch()` (`P2D01`):** Flash's scene graph (Stage, MovieClip,
   SimpleButton, TextField) assigns persistent character IDs and integer depth indices to active
   display objects. These map naturally onto `P2D00` stable instruction IDs, allowing `PaintVM.patch()`
   to calculate minimal draw call deltas between consecutive frames.
3. **ActionScript Lowers to `InterpreterIR` (`LANG01`) & `vm-core` (`LANG02`):**
   - ActionScript 1.0, 2.0, and 3.0 are dialects of the ECMAScript standard already codified in
     `versioned-ecmascript-typescript-grammars.md`.
   - Source code (`.as`) parses via `code/grammars/actionscript/` and lowers to `InterpreterIR`
     under the `LANG84` framework.
   - Compiled bytecode—both AVM1 actions in `DoAction`/`DoInitAction` tags and AVM2 ABC bytecode
     in `DoABC` tags—is lifted directly into `InterpreterIR` functions using neutral `LANG83`
     virtual machine operations, executed deterministically on `vm-core`.
4. **Zero Ambient Authority & Proven Capability Containment:** Flash was historically notorious
   for remote code execution and heap corruption vulnerabilities. By confining the entire simulation
   core within a capability cage (`"capabilities": []` under `VER08a`/`VER08b`) and validating all
   bytecode control-flow graphs prior to execution, this runtime eliminates entire categories of
   historical Flash exploits.

---

## §2 End-to-End Pipeline Architecture

The end-to-end playback pipeline decouples container parsing, vector lowering, bytecode execution,
scene composition, and host presentation:

```text
               Unmodified .swf File (FWS / CWS / ZWS)
                                 │
                                 ▼
                     ┌───────────────────────┐
                     │     swf-container     │ (SWF01)
                     │ Header & Tag Framing  │
                     └───────────┬───────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         │ Tag Stream            │ Tag Stream            │ Tag Stream
         ▼                       ▼                       ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ DefineShape1..4  │    │ DoAction / DoABC │    │   DefineSound    │
│ Bitmap Tags      │    │  (AVM1 / AVM2)   │    │ SoundStreamBlock │
└────────┬─────────┘    └────────┬─────────┘    └────────┬─────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│swf-shape-to-paint│    │  avm1/avm2-to-iir│    │flash-audio-bridge│ (SWF06)
│(SWF02) -> P2D00  │    │(SWF04) -> LANG01 │    │ -> MUS02 / MUS06 │
└────────┬─────────┘    └────────┬─────────┘    └────────┬─────────┘
         │                       │                       │
         │                       ▼                       │
         │              ┌──────────────────┐             │
         │              │     vm-core      │             │
         │              │ (LANG02 / LANG83)│             │
         │              └────────┬─────────┘             │
         │                       │                       │
         └───────────────┬───────┴───────────────────────┘
                         ▼
             ┌───────────────────────┐
             │   flash-display-list  │ (SWF05)
             │ Timeline, Stage, FSM  │
             └───────────┬───────────┘
                         │ Emits diffable PaintScene (P2D00)
                         ▼
             ┌───────────────────────┐
             │   flash-player-host   │ (SWF07)
             │ UI11 Window / Canvas  │
             └───────────────────────┘
```

---

## §3 Decomposed Sub-Packages (SWF01..SWF08)

The Flash runtime is decomposed into eight focused packages, organized strictly to isolate parsing,
rendering, execution, and host integration:

```text
SWF00: Flash Runtime Master Roadmap & Architecture Spec
SWF01: swf-container           ── Zero-dependency binary container & streaming tag parser
SWF02: swf-shape-to-paint      ── Dual-fill Bézier shape converter -> P2D00 PaintInstructions
SWF03: actionscript-compiler   ── Versioned AS1/2/3 grammars & AST -> InterpreterIR (LANG84)
SWF04: avm-bytecode-lifter     ── AVM1 & AVM2 bytecode lifter -> InterpreterIR / vm-core
SWF05: flash-display-list      ── Stage, MovieClip timeline, Button, TextField & scene diffing
SWF06: flash-audio-bridge      ── SWF sound tags & AS Sound API -> MUS02/MUS06 audio sinks
SWF07: flash-player-host       ── Top-level player host shell (UI11 Window Core & Web Canvas)
SWF08: swf-conformance-corpus  ── Synthetic .swf test suite & bit-exact frame digest assertions
```

### 1. `SWF01`: `swf-container`
- **Scope:** Zero-dependency, bounded binary reader for SWF headers and tagged data streams.
- **Header Parsing:**
  - Signature identification: `FWS` (uncompressed), `CWS` (zlib compressed via repo `IC04`/`flate` decoder), and `ZWS` (LZMA compressed).
  - Version byte: $1 \le \text{version} \le 10$.
  - FileLength: 32-bit integer (untrusted header value; strictly bounded during reading).
  - FrameSize: Bit-packed `RECT` structure (xmin, xmax, ymin, ymax) encoded in twips.
  - FrameRate: 8.8 fixed-point frames per second.
  - FrameCount: 16-bit total timeline frame count.
- **Tag Framing:**
  - Tag header decoding: Short tags ($\text{tag\_code} = \text{word} \gg 6$, $\text{length} = \text{word} \ \& \ 0\text{x}3\text{F}$) and Long tags ($\text{length} == 0\text{x}3\text{F}$, followed by signed 32-bit length).
  - Bounded Tag Iterator: Emits `SwfTag` slices without buffering entire movies into heap memory.
- **Tag Vocabulary:** `End`, `ShowFrame`, `SetBackgroundColor`, `DefineShape`, `DefineShape2`, `DefineShape3`, `DefineShape4`, `PlaceObject`, `PlaceObject2`, `PlaceObject3`, `RemoveObject`, `RemoveObject2`, `DefineSprite`, `DoAction`, `DoInitAction`, `DoABC`, `DefineBits`, `DefineBitsJPEG2`, `DefineBitsJPEG3`, `DefineBitsLossless`, `DefineBitsLossless2`, `DefineSound`, `StartSound`, `SoundStreamHead`, `SoundStreamHead2`, `SoundStreamBlock`.
- **Capability Cage:** `"capabilities": []`.

### 2. `SWF02`: `swf-shape-to-paint`
- **Scope:** Converts SWF vector definition records into `P2D00` `PaintInstructions`.
- **Coordinate Conversion:** Transforms coordinates from twips ($1\text{ twip} = \frac{1}{20}\text{ pixel}$) to floating-point units.
- **Dual-Fill Edge Loop Reconstruction:**
  - SWF stores shapes as sequences of `StraightEdgeRecord` and `CurvedEdgeRecord` segments, each tagging `FillStyle0` (left side) and `FillStyle1` (right side).
  - Reconstructs closed directed polygon contours by pairing edges sharing identical fill styles, properly handling complex topology, self-intersections, and inner island cutouts (`FillRule::EvenOdd` or `NonZero`).
- **Lowering to `PaintInstructions` (`P2D00`):**
  - Quadratic Bézier curves: Evaluated via `bezier2d` (`G2D02`) and converted into `PathVerb::QuadTo`.
  - Solid Color Fills: Emit `PaintPath` with RGBA colors.
  - Gradient Fills: Emit `PaintGradient` supporting linear and radial gradients with up to 15 color stops (ratios $0..255$) and 2D focal point matrices.
  - Bitmap Fills: Clipped or tiled bitmap textures mapped to `PaintImage` references.
  - Line Styles: Emit `PaintStroke` with line width, miter limits, cap styles (`Round`, `None`, `Square`), and join styles (`Round`, `Bevel`, `Miter`).

### 3. `SWF03`: `actionscript-compiler`
- **Scope:** Source-level compiler for ActionScript 1.0, 2.0, and 3.0 scripts following `LANG84`.
- **Grammar Architecture (`code/grammars/actionscript/`):**
  - Managed releases: `as1.0`, `as2.0`, `as3.0` registered in `releases.csv`.
  - Lexer and Parser: Compiled via `grammar-tools` into `actionscript-lexer` and `actionscript-parser`.
- **Dialect Semantics:**
  - `as1.0`: Dynamic, prototype-based ECMAScript 3 subset with timeline verbs (`tellTarget`, `call`, `eval`).
  - `as2.0`: Adds compile-time classes, interfaces, and type annotations (`var x: Number`), lowered into prototype chains.
  - `as3.0`: Full ECMAScript 4 dialect with runtime classes, sealed objects, namespaces, packages, and strong typing.
- **CST Lowering:** `actionscript-iir-compiler` walks `GrammarASTNode` trees with strict `FrontendBudgets` to emit validated `InterpreterIR` (`LANG01`) modules.

### 4. `SWF04`: `avm-bytecode-lifter`
- **Scope:** Binary bytecode lifters translating compiled Flash bytecode into `InterpreterIR` (`LANG01`).
- **AVM1 Action Lifter (`avm1-to-iir`):**
  - Decodes action byte streams in `DoAction` and `DoInitAction` tags.
  - Stack and Arithmetic Actions: `ActionPush`, `ActionPop`, `ActionAdd2`, `ActionSubtract`, `ActionMultiply`, `ActionDivide`, `ActionEquals2`, `ActionLess2`, `ActionAnd`, `ActionOr`, `ActionNot`.
  - Object and Member Actions: `ActionGetVariable`, `ActionSetVariable`, `ActionGetMember`, `ActionSetMember`, `ActionCallFunction`, `ActionCallMethod`, `ActionNewObject`.
  - Control Flow Actions: `ActionBranchAlways`, `ActionBranchIfTrue`, `ActionDefineFunction`, `ActionDefineFunction2` (with activation register allocations).
  - Lifts stack operations into basic block static single assignment (SSA) or explicit operand stacks within `IIRModule`.
- **AVM2 ABC Lifter (`avm2-to-iir`):**
  - Decodes ActionScript Bytecode (`DoABC`) structures: Constant Pool (`int`, `uint`, `double`, `string`, `namespace`, `multiname`), Method Signatures, Metadata, Instance/Class Traits, Scripts, and Method Bodies.
  - Opcodes: `getlocal`, `setlocal`, `pushstring`, `pushint`, `add`, `subtract`, `multiply`, `divide`, `findpropstrict`, `getproperty`, `setproperty`, `callproperty`, `constructprop`, `initproperty`.
  - Verifies operand stack heights and local register types across all basic blocks before emitting `IIRModule` functions for execution on `vm-core` (`LANG02`).

### 5. `SWF05`: `flash-display-list`
- **Scope:** Interactive scene graph, timeline state machine, and retained-mode draw call diffing.
- **Display Object Hierarchy:**
  - `Stage`: The root rendering canvas containing global frame settings and stage dimensions.
  - `MovieClip`: Self-contained timeline entity with child display list, current frame index, total frames, and play state (`Play`, `Stop`).
  - `SimpleButton`: Interactive 4-state control (`Up`, `Over`, `Down`, `HitTest`).
  - `TextField`: Dynamic and input text renderer with variable bindings.
  - `MorphShape`: Tweened vector shapes interpolated across frame ratios.
- **Timeline Frame Tick Algorithm:**
  1. Increment internal frame counter for active movie clips.
  2. Execute frame-associated script actions (`DoAction`).
  3. Process `PlaceObject2`/`PlaceObject3` tags: update affine transforms (`G2D01`), color transforms (`CXFORM`), blend modes, and clipping masks.
  4. Process `RemoveObject2` tags: detach objects from specified depth layers.
  5. Dispatch `onEnterFrame` / `Event.ENTER_FRAME` notifications into `vm-core`.
  6. Process user input (mouse move, mouse down, key presses) from `UI11`, executing hit tests using `PointInPath` from `P2D08`.
  7. Traverse active display tree in depth order, accumulate hierarchical coordinate matrices, and construct a new `PaintScene` (`P2D00`) with stable character IDs.
  8. Perform `PaintVM.patch(old_scene, new_scene)` to generate minimal rendering patches.

### 6. `SWF06`: `flash-audio-bridge`
- **Scope:** Audio tag decoding and runtime playback synchronization.
- **Audio Formats:**
  - Uncompressed PCM (8-bit unsigned, 16-bit signed little-endian).
  - ADPCM: Flash custom 2-bit, 3-bit, 4-bit, and 5-bit adaptive delta pulse-code modulation.
  - MP3 frame extraction from streaming tags.
- **Audio Routing:**
  - Event Sounds: `DefineSound` tags cached and triggered on demand by `StartSound` tags or ActionScript `Sound.start()`.
  - Streaming Sounds: Interleaved `SoundStreamBlock` payloads synchronized with the visual frame clock.
  - Lowering: Streams decoded PCM samples directly into `MUS02` (`audio-device-sink`) or `MUS06` (`web-audio-sink`).

### 7. `SWF07`: `flash-player-host`
- **Scope:** Integrated application shell uniting container, display list, virtual machine, and host platform.
- **Host Targets:**
  - Desktop Native: Binds to `UI11` (`window-core`, `window-win32`, `window-appkit`) with rendering via `paint-vm-runtime` (Metal, Direct2D, Cairo, Skia, WGPU).
  - Web Browser: Compiles to WebAssembly, rendering to HTML5 `<canvas>` via `window-canvas` and routing sound to `MUS06`.
  - Headless CI Runner: Executes SWF files offscreen, verifying exact deterministic frame digests without requiring an active display server.

### 8. `SWF08`: `swf-conformance-corpus`
- **Scope:** Curated test suite of synthetic, clean-room `.swf` files testing every layer of the runtime.
- **Corpus Test Matrix:**
  - `corpus-01-shapes`: Static lines, quadratic Bézier curves, linear/radial gradients, and hole cutouts.
  - `corpus-02-timeline`: Multi-frame animation loops, `gotoAndPlay()`, `gotoAndStop()`, and nested clips.
  - `corpus-03-buttons`: Mouse hit testing, cursor tracking, and button state transitions.
  - `corpus-04-avm1-math`: AVM1 arithmetic, stack operations, variable scoping, and function closures.
  - `corpus-05-avm2-oop`: AVM2 class hierarchies, interface dispatch, and exception handling.
  - `corpus-06-streaming-audio`: Frame-by-frame synchronization of streaming ADPCM audio blocks.
- **Assertion Mode:** Each test specifies exact SHA-256 digests of emitted `PaintScene` structures for frames $1..N$.

---

## §4 Vector Graphics Lowering: SWF Shapes to PaintInstructions

SWF encodes vector drawings using an incremental, edge-based record structure that must be
transformed into modern closed paths.

### 1. Coordinate Spaces & Twips
All coordinates inside SWF shape definitions are encoded in *twips* ($1\text{ twip} = \frac{1}{20}\text{ point} = \frac{1}{20}\text{ pixel}$ at 72 DPI).
Conversion to standard pixel coordinates follows:
$$x_{\text{pixel}} = \frac{x_{\text{twip}}}{20.0}, \quad y_{\text{pixel}} = \frac{y_{\text{twip}}}{20.0}$$

### 2. Quadratic Bézier Formulation
SWF curves are quadratic Bézier segments defined by an anchor point $P_0$, a control point $P_1$,
and an end point $P_2$:
$$B(t) = (1-t)^2 P_0 + 2(1-t)t P_1 + t^2 P_2, \quad t \in [0, 1]$$
In `P2D00` (`PaintInstructions`), this maps directly to:
```rust
PathVerb::MoveTo(P0);
PathVerb::QuadTo(P1, P2);
```

### 3. Color Transforms (`CXFORM` / `CXFORMWITHALPHA`)
Flash applies hierarchical color transformations to display objects, parameterized by multiplier
($M$) and additive ($A$) terms for each RGBA channel:
$$R' = \text{clamp}\left(\frac{R \cdot M_R}{256} + A_R, 0, 255\right)$$
In `P2D00`, this lowers to a `PaintGroup` node carrying a `ColorTransform` property, evaluated
during scanline rasterization (`P2D08`) or via fragment shader constants in GPU backends (`P2D06`).

### 4. Masking & Clipping Depths
When a `PlaceObject2` tag specifies `clip_depth: Some(D)`, the object at the current depth serves
as a stencil mask for all subsequent display objects up to depth $D$. In `P2D00`, this lowers to:
```rust
PaintInstruction::PaintClip {
    clip_path: mask_path,
    content: Box::new(grouped_children),
}
```

---

## §5 ActionScript Execution via LangVM: AVM1 & AVM2 to InterpreterIR

ActionScript bytecode is lifted into `InterpreterIR` (`LANG01`) modules, enabling uniform execution,
optimization, and verification across all supported backends.

### 1. AVM1 Action Mapping Table
AVM1 is an untyped, stack-based bytecode architecture. Bytecode sequences inside `DoAction` tags
are translated into `IIRModule` basic blocks:

| AVM1 Action Opcode | Semantics | Target `InterpreterIR` (`LANG01`) Lowering |
|---|---|---|
| `ActionPush` | Pushes immediate value (string, float, int, register, bool) | `IIRInst::PushConst(val)` |
| `ActionPop` | Discards top of stack | `IIRInst::Pop` |
| `ActionAdd2` | Polymorphic addition (numeric sum or string concat) | `IIRInst::CallBuiltin("flash.lang.add2", [a, b])` |
| `ActionSubtract` | Numeric subtraction | `IIRInst::BinOp(Sub, a, b)` |
| `ActionMultiply` | Numeric multiplication | `IIRInst::BinOp(Mul, a, b)` |
| `ActionDivide` | IEEE-754 division | `IIRInst::BinOp(Div, a, b)` |
| `ActionEquals2` | Loose equality (`==`) comparison | `IIRInst::BinOp(Eq, a, b)` |
| `ActionGetVariable`| Resolves variable name through scope chain | `IIRInst::CallBuiltin("flash.vm.get_var", [name])` |
| `ActionSetVariable`| Assigns variable name in current scope | `IIRInst::CallBuiltin("flash.vm.set_var", [name, val])` |
| `ActionGetMember`  | Dynamic property access `obj[prop]` | `IIRInst::LoadMember(obj, prop)` |
| `ActionSetMember`  | Dynamic property assignment `obj[prop] = val` | `IIRInst::StoreMember(obj, prop, val)` |
| `ActionCallMethod` | Method invocation on object | `IIRInst::CallMethod(obj, method_name, args)` |
| `ActionBranchAlways`| Unconditional jump | `IIRInst::Branch(target_block)` |
| `ActionBranchIfTrue`| Conditional jump if top of stack is truthy | `IIRInst::BranchCond(cond, true_block, false_block)` |
| `ActionDefineFunction2`| Function declaration with activation registers | Lowers to a distinct `IIRFunction` with mapped local registers |

### 2. AVM2 ABC Bytecode Mapping
AVM2 is a typed, register-and-stack hybrid bytecode architecture. Bytecode sequences inside `DoABC`
tags map directly to structured `InterpreterIR` operations:
- `getlocal_N` / `setlocal_N` $\to$ `IIRInst::LoadLocal(reg_n)` / `IIRInst::StoreLocal(reg_n, val)`.
- `findpropstrict` $\to$ `IIRInst::CallBuiltin("flash.vm.find_property", [multiname])`.
- `callproperty` $\to$ `IIRInst::CallVirtual(method_id, args)`.
- `constructprop` $\to$ `IIRInst::NewInstance(class_id, args)`.

---

## §6 DisplayList, Event Dispatch, and Audio Synchronization

The playback loop advances each frame through a strictly ordered pipeline, ensuring deterministic
behavior:

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                        Single Frame Tick Execution                        │
├───────────────────────────────────────────────────────────────────────────┤
│ 1. Time Accumulation: Advance movie timeline clocks.                     │
│ 2. Tag Application: Process PlaceObject/RemoveObject tags for new frame. │
│ 3. Script Execution: Execute frame actions (DoAction) on vm-core.         │
│ 4. Lifecycle Events: Dispatch onEnterFrame / ENTER_FRAME handlers.       │
│ 5. User Input Dispatch: Process mouse/keyboard events via UI11 hit tests.│
│ 6. Audio Sync: Advance streaming audio buffers via flash-audio-bridge.   │
│ 7. Display Tree Traversal: Flatten active DisplayList into PaintScene.   │
│ 8. Diff & Present: Execute PaintVM.patch() and render dirty regions.     │
└───────────────────────────────────────────────────────────────────────────┘
```

---

## §7 SWF-Specific Vulnerability Hardening (CVE / CWE Defense Matrix)

Flash Player was historically subject to hundreds of high-severity CVEs involving memory safety
violations, buffer overflows, and sandbox escapes. The `SWF*` suite eliminates these failure
classes by construction:

### 1. CWE-409 (Decompression Bombs / Zip Bombs)
- **Vulnerability Mechanism:** A maliciously crafted `.swf` file declaring `CWS` (zlib) or `ZWS` (LZMA)
  compression may present a tiny compressed file size (e.g., 20 KB) that inflates into tens of gigabytes
  of memory, causing host out-of-memory crashes.
- **Mandatory Defenses in `SWF01` (`swf-container`):**
  - Untrusted FileLength: The 32-bit `FileLength` in the SWF header is treated as an untrusted hint.
  - Hard Allocation Cap: Total uncompressed buffer allocations are capped at `max_uncompressed_bytes = 256 MB`.
  - Expansion Ratio Enforcement: The decompressor streams data in 64 KB chunks, continuously calculating
    the ratio $\text{bytes}_{\text{out}} / \text{bytes}_{\text{in}}$. If the expansion ratio exceeds `50:1`
    beyond the first 1 MB of output, decompression terminates with `SwfError::DecompressionBombDetected`.

### 2. CWE-400 / CWE-770 (Algorithmic DoS & Cyclic Resource Exhaustion)
- **Vulnerability Mechanism:** Malicious scripts containing infinite loops (`while(true){}`), recursive
  movie clip instantiations, or circular display list hierarchies designed to freeze the host process.
- **Mandatory Defenses in `SWF04` / `SWF05`:**
  - `FrontendBudgets` VM Step Limit: Frame script execution on `vm-core` enforces a hard instruction budget
    (`max_steps = 2_000_000` steps per frame tick). If the counter exhausts, execution suspends with
    `SwfError::ScriptTimeout`, allowing the frame to render without hanging the host.
  - Display Tree Nesting Bound: Maximum movieclip hierarchy depth is restricted to `max_depth = 64`. Any
    tag attempting to place a sprite beyond this depth returns `SwfError::HierarchyDepthExceeded`.
  - Call Stack Depth Bound: ActionScript execution restricts function call recursion to `max_stack_depth = 256`.

### 3. CWE-125 / CWE-787 (Out-of-Bounds Memory Read & Write)
- **Vulnerability Mechanism:** Corrupted tag lengths reading past buffer boundaries, or malformed bytecode
  jumping into the middle of multi-byte instructions or popping empty operand stacks.
- **Mandatory Defenses in `SWF01` / `SWF04`:**
  - Bounded Slice Reader: Every parser operation in `swf-container` operates on bounded sub-slices. Slices
    are checked against tag boundary lengths, physically preventing over-reads.
  - Bytecode CFG Verification: Prior to lifting into `InterpreterIR`, `avm-bytecode-lifter` validates all
    branch offsets (`ActionBranchAlways`, `ActionBranchIfTrue`). Targets must land strictly on decoded
    instruction boundaries. Jumping into immediate operands returns `SwfError::InvalidBranchTarget`.
  - Abstract Stack Simulation: Operand stack depths are verified for each basic block. Any path resulting
    in negative stack height triggers a compile-time rejection (`SwfError::StackUnderflow`).

### 4. CWE-1321 (Prototype Pollution in ActionScript 1 & 2)
- **Vulnerability Mechanism:** ActionScript 1/2 permits dynamic assignment to `Object.prototype`,
  `Function.prototype`, and `_global`. Malicious SWF files could overwrite core built-ins to compromise
  host container bridges.
- **Mandatory Defenses in `SWF04` / `SWF05`:**
  - Isolated Prototype Environments: Each SWF movie instance receives its own isolated prototype registry.
  - Frozen Native Built-ins: Host-provided core classes (`Object`, `Array`, `String`, `Math`, `MovieClip`)
    are marked immutable. User scripts cannot overwrite native dispatch tables.
  - Null-Prototype Dictionaries: Internal state tables are constructed without prototype inheritance.

### 5. CWE-94 / CWE-78 (Code Injection & Command Execution)
- **Vulnerability Mechanism:** Flash supported legacy commands such as `fscommand("exec", cmd)` and
  `getURL("javascript:...")` to launch OS processes or inject browser scripts.
- **Mandatory Defenses in `SWF07`:**
  - Zero Ambient Authority: The runtime core declares `"capabilities": []`.
  - Disabled Subprocess Launch: `fscommand("exec")` is permanently disabled in the core; calls return
    no-ops or emit diagnostic audit events.
  - Restricted URI Schemes: External URL navigation (`getURL`, `navigateToURL`) is strictly sanitized,
    blocking `javascript:`, `file:`, and `data:` schemes.

---

## §8 Verification & Conformance Gates (VER-*, Warrant, Corpus)

To achieve verified status, the `SWF*` suite implements rigorous verification evidence:

1. **Warrant Formal Specifications (`VER04` / `VER04m`):**
   - Pure packages (`swf-container`, `swf-shape-to-paint`, `avm-bytecode-lifter`) specify their complete
     contracts in formal `.warrant` files.
   - Tag decoding and shape tessellation invariants are proven against negative sabotage mutations (`VER04h`).
2. **Zero-Allocation Hot-Loop Benchmarks (`VER07`):**
   - Microbenchmarks for display list traversal, timeline updates, and `PaintScene` generation must pass
     `install_counting_allocator!()` asserting `max_alloc_bytes: Some(0)` during steady-state animation playback.
3. **Capability Cage Enforcement (`VER08a`, `VER08b`):**
   - `swf-container`, `swf-shape-to-paint`, `actionscript-compiler`, and `avm-bytecode-lifter` declare
     `"capabilities": []` in `required_capabilities.json`.
   - Verified via static analysis (`ca-capability-analyzer`) and runtime Linux sandboxing (`seccomp-bpf` + `Landlock`).
4. **Weakness Registry Closure (`VER15`):**
   - Every package publishes an `assurance_report.json` closing all 26 CWE classes in `weakness-registry-v1.json`.

---

## §9 Backlog Decomposition & Implementation Waves

The implementation proceeds in four distinct waves:

- **Wave 1: Container & Shape Tessellation (`SWF01`, `SWF02`, `SWF08` Subset):**
  - Implement `swf-container` (`FWS`/`CWS`/`ZWS` parsing, tag iterator, bounding box reader).
  - Implement `swf-shape-to-paint` (dual-fill reconstruction, Bézier curves, solid/gradient fills $\to$ `P2D00`).
  - Render static vector art from synthetic SWF files into `paint-vm-ascii` and software pixel buffers.
- **Wave 2: Bytecode Lifter & Virtual Machine Integration (`SWF04` AVM1, `SWF03` AS1/2):**
  - Implement `avm1-to-iir` lifter, translating AVM1 action blocks into `InterpreterIR` (`LANG01`).
  - Bind `vm-core` (`LANG02`) execution to Flash variable scopes and object prototypes.
  - Land `actionscript.grammar` for source ActionScript compilation (`LANG84`).
- **Wave 3: DisplayList, Interactive Timeline & Audio (`SWF05`, `SWF06`):**
  - Implement `Stage`, `MovieClip`, `SimpleButton`, and depth-sorted rendering diffs via `PaintVM.patch()`.
  - Connect mouse/keyboard input from `UI11` (`window-core`).
  - Implement `flash-audio-bridge`, decoding ADPCM/PCM tags and streaming to `MUS02`/`MUS06`.
- **Wave 4: AVM2 / ActionScript 3 & Host Shell Integration (`SWF04` AVM2, `SWF07`, `SWF08` Complete):**
  - Implement `avm2-to-iir` for `DoABC` bytecode and ActionScript 3 OOP execution.
  - Build `flash-player-host` for desktop windowing and WebAssembly browser embedding.
  - Finalize the full `swf-conformance-corpus` test suite with bit-exact frame digest assertions.

---

## §10 References

### Internal Specifications & Substrates
- **Vector Graphics & Rendering:** [`P2D00`](P2D00-paint-instructions.md), [`P2D01`](P2D01-paint-vm.md), [`P2D02`](P2D02-paint-vm-ascii.md), [`P2D08`](P2D08-path-raster.md), [`G2D01`](G2D01-affine2d.md), [`G2D02`](G2D02-bezier2d.md).
- **Virtual Machines & Grammars:** [`LANG01`](LANG01-interpreter-ir.md), [`LANG02`](LANG02-vm-core.md), [`LANG84`](LANG84-grammar-driven-versioned-frontends.md), [`versioned-ecmascript-typescript-grammars.md`](versioned-ecmascript-typescript-grammars.md).
- **Audio Synthesis:** [`MUS00`](MUS00-note-frequency.md), [`MUS01`](MUS01-note-to-sound-signal-chain.md), [`MUS02`](MUS02-audio-device-sink.md), [`MUS06`](MUS06-web-audio-sink.md).
- **Verification & Security:** [`VER04`](VER04-warrant-language.md), [`VER04m`](VER04m-warrant-stateful-objects.md), [`VER07`](VER07-microbenchmarks.md), [`VER08a`](VER08a-rust-capability-analyzer.md), [`VER08b`](VER08b-runtime-capability-conformance.md), [`VER15`](VER15-weakness-registry.md).

### Primary Format & Specification References
1. Adobe Systems Inc. (2008). *SWF File Format Specification Version 10*.
2. Adobe Systems Inc. (2007). *ActionScript 3.0 Bytecode (ABC) File Format Specification*.
3. Adobe Systems Inc. (2006). *ActionScript 2.0 Language Reference*.
4. ECMA International (1999). *Standard ECMA-262: ECMAScript Language Specification, 3rd Edition*.
5. W3C (2003). *Scalable Vector Graphics (SVG) 1.1 Specification* (Quadratic Bézier & Fill Rules).
