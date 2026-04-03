# P2D01 — PaintVM: The Dispatch-Table Virtual Machine

## Overview

PaintVM is the execution engine for PaintInstructions (P2D00). It is a
**dispatch-table virtual machine**: each instruction kind maps to a handler
function, and the VM routes every instruction to its handler at runtime.

This design pattern will be familiar from the `virtual-machine` package already
in this repo. That VM dispatches bytecode opcodes. PaintVM dispatches paint
instruction kinds. The principle is identical:

```
opcode → handler     (bytecode VM)
kind   → handler     (paint VM)
```

The PaintVM is NOT a monolithic renderer. It is a **framework** — a skeleton
that backends fill in. A PaintVM instance with no registered handlers would
crash on the first instruction. That is correct behavior; see the
"UnknownInstructionError" section.

### PaintVM's place in the stack

```
PaintScene (P2D00)
     |
     ▼
PaintVM.execute(scene, ctx)          ← immediate mode: redraw everything
PaintVM.patch(old, new, ctx)         ← retained mode: repaint only changes
     |
     ▼
Dispatch table: kind → handler
     |
     ├── "rect"      → rect_handler(instruction, ctx, vm)
     ├── "ellipse"   → ellipse_handler(instruction, ctx, vm)
     ├── "path"      → path_handler(instruction, ctx, vm)
     ├── "glyph_run" → glyph_run_handler(instruction, ctx, vm)
     ├── "group"     → group_handler(instruction, ctx, vm)
     ├── "line"      → line_handler(instruction, ctx, vm)
     ├── "clip"      → clip_handler(instruction, ctx, vm)
     ├── "gradient"  → gradient_handler(instruction, ctx, vm)
     ├── "image"     → image_handler(instruction, ctx, vm)
     └── "layer"     → layer_handler(instruction, ctx, vm)

PaintVM.export(scene, options?)              ← pixel output: renders to offscreen, returns PixelContainer
     |
     ▼
PixelContainer (P2D00)
     |
     ▼
ImageCodec.encode(pixels)                   ← codec layer (paint-codec-png, paint-codec-webp, ...)
```

Each backend creates a PaintVM instance, registers its handlers, and exports
a factory function. Users of the backend call `createSvgVM()` (or equivalent)
and get a fully configured VM ready to execute or patch PaintScenes.

---

## Public API

```typescript
// PaintVM is generic over TContext — the backend's rendering context type.
// The VM itself does not know what TContext is. It only passes it to handlers.
//
// Examples:
//   SVG backend:      TContext = SVGElement
//   Canvas backend:   TContext = CanvasRenderingContext2D
//   Metal backend:    TContext = MTLCommandEncoder  (via Rust FFI)
//   Terminal backend: TContext = StringBuffer
interface PaintVM<TContext> {

  // Register a handler for a specific instruction kind.
  //
  // Calling register() twice for the same kind is a programming error —
  // it throws immediately (at registration time, not at execution time).
  // This catches typos and accidental double-registration during development.
  register(kind: string, handler: PaintHandler<TContext>): void;

  // Immediate mode: clear the context and execute all instructions in scene.
  //
  // This is the "dumb but correct" rendering path:
  //   1. Clear the entire context to scene.background.
  //   2. Walk scene.instructions in order, dispatch each one.
  //
  // Use execute() when:
  //   - The scene changes completely between frames (game, animation).
  //   - Simplicity is more important than performance.
  //   - You don't want to maintain old/new scene state.
  execute(scene: PaintScene, context: TContext): void;

  // Retained mode: diff old and new scenes, execute only the changes.
  //
  // Use patch() when:
  //   - Most of the scene is stable across frames.
  //   - Performance matters (e.g., 60fps charts with live data).
  //   - The producer has assigned stable ids to dynamic instructions.
  //
  // See "patch() algorithm" below for the full diffing contract.
  patch(old: PaintScene, new: PaintScene, context: TContext): void;

  // Pixel export: render the scene to an internal offscreen buffer and return
  // the raw pixels as a PixelContainer (P2D00).
  //
  // export() does NOT use or modify the live TContext. It allocates its own
  // internal offscreen surface, runs execute() against it, reads back pixels,
  // and returns them. The caller's context is untouched.
  //
  // Use export() when:
  //   - You need to write the scene to an image file (PNG, WebP, JPEG, etc.).
  //   - You need to pass pixels to a codec, a test comparator, or a hash function.
  //   - You need a pixel-accurate snapshot without affecting the live render surface.
  //
  // The returned PixelContainer is owned by the caller — the VM does not retain it.
  // Pass it to an ImageCodec.encode() to compress it into a specific format.
  //
  // Example:
  //   const pixels = vm.export(scene, { scale: 2 });  // 2× pixel density
  //   const bytes  = pngCodec.encode(pixels);
  //   fs.writeFileSync("output.png", bytes);
  export(scene: PaintScene, options?: ExportOptions): PixelContainer;
}

// Options for export().
interface ExportOptions {
  scale?: number;
  // Pixel density multiplier. Default: 1.0.
  // At scale 2.0, a scene with width=400 produces a 800-pixel-wide PixelContainer.
  // Use 2.0 or 3.0 for high-DPI (Retina) exports.

  channels?: 3 | 4;
  // Number of channels in the output PixelContainer. Default: 4 (RGBA).
  // Use 3 (RGB) when the format doesn't support alpha (e.g., JPEG).

  bit_depth?: 8 | 16;
  // Bits per channel. Default: 8.
  // Use 16 for HDR pipelines or lossless precision.

  color_space?: "srgb" | "display-p3" | "linear-srgb";
  // Output color space. Default: "srgb".
}

// A handler for a single instruction kind.
// It receives:
//   instruction — the fully typed PaintInstruction (union member)
//   context     — the backend's rendering context (TContext)
//   vm          — the PaintVM itself, so container handlers can recurse
//
// Container instructions (PaintGroup, PaintClip) MUST call vm.dispatch()
// on each child. The VM does not recurse automatically — handlers are
// responsible for recursing into children.
type PaintHandler<TContext> = (
  instruction: PaintInstruction,
  context: TContext,
  vm: PaintVM<TContext>
) => void;

// Thrown when execute() or patch() encounters an instruction kind that has
// no registered handler. This is always a programming error.
class UnknownInstructionError extends Error {
  kind: string;    // the instruction kind that had no handler
  // message: "No handler registered for instruction kind: 'svg:marker'"
}
```

### The TContext generic — why it exists

The VM is generic over `TContext` so that each backend's handler functions
can operate directly on the backend's native context type without any casting.

Without the generic, handlers would need to cast:

```typescript
// Without generic — error prone:
function handleRect(instruction: PaintRect, context: unknown, vm: PaintVM) {
  const ctx = context as CanvasRenderingContext2D;  // cast needed every time
  ctx.fillRect(instruction.x, ...);
}
```

With the generic:

```typescript
// With generic — type-safe:
function handleRect(
  instruction: PaintRect,
  context: CanvasRenderingContext2D,
  vm: PaintVM<CanvasRenderingContext2D>
) {
  context.fillRect(instruction.x, ...);  // no cast needed
}
```

The generic makes handler type signatures self-documenting: the function signature
tells you exactly which backend it's for.

---

## execute() Algorithm

```
function execute(scene: PaintScene, context: TContext):
  // Step 1: Clear the entire rendering surface.
  // This is backend-specific (clear DOM nodes, reset canvas, etc.).
  // The background color comes from the scene.
  clear(context, scene.background, scene.width, scene.height)

  // Step 2: Walk the top-level instructions in declaration order.
  // Instructions are rendered back-to-front (painter's algorithm):
  // the first instruction in the array is painted first (furthest back).
  for instruction in scene.instructions:
    dispatch(instruction, context)

function dispatch(instruction: PaintInstruction, context: TContext):
  handler = table[instruction.kind]
  if handler is null or undefined:
    throw new UnknownInstructionError(instruction.kind)
  handler(instruction, context, self)
```

### Container nodes handle their own recursion

The VM's `dispatch()` function does NOT automatically recurse into children.
Container instructions (PaintGroup, PaintClip) are responsible for recursing
into their children by calling `vm.dispatch()` themselves.

This means a group handler looks like this:

```typescript
// Example PaintGroup handler for the Canvas backend
function handleGroup(
  instruction: PaintGroup,
  ctx: CanvasRenderingContext2D,
  vm: PaintVM<CanvasRenderingContext2D>
) {
  ctx.save();                               // save current state
  if (instruction.transform) {
    ctx.transform(...instruction.transform); // apply the affine matrix
  }
  if (instruction.opacity !== undefined) {
    ctx.globalAlpha = instruction.opacity;
  }
  for (const child of instruction.children) {
    vm.dispatch(child, ctx);               // recurse into each child
  }
  ctx.restore();                           // restore state (undoes transform + opacity)
}
```

The VM hands control entirely to the handler. The handler decides whether and
how to recurse. This design allows backends to:

- Skip children based on visibility (e.g., culling off-screen groups).
- Inject pre/post rendering steps around children.
- Handle groups differently (e.g., SVG wraps in `<g>`, terminal just recurses).

The important constraint: `vm.dispatch()` must be called on each child,
not direct handler invocation. This ensures the dispatch table is used and
unknown kinds are caught even in nested contexts.

---

## patch() Algorithm — The Diff

patch() implements a structural diff between two PaintScene versions. It executes
only the minimal set of backend operations needed to transform the rendered output
of `old` into the rendered output of `new`.

### Why patch() is worth the complexity

Consider a dashboard with 200 chart elements. Sixty times per second, three data
points update — three bars change height. execute() would repaint all 200 elements.
patch() repaints only the three that changed. For complex scenes, this is a 60×
reduction in rendering work.

### The algorithm

```
function patch(old_scene: PaintScene, new_scene: PaintScene, context: TContext):

  // Step 1: Build identity maps.
  // An identity map groups instructions by their id field.
  // Instructions without id are not included in the identity map —
  // they are handled positionally in step 3.
  old_by_id: Map<string, PaintInstruction> = index_by_id(old_scene.instructions)
  new_by_id: Map<string, PaintInstruction> = index_by_id(new_scene.instructions)

  // Step 2: Deletions — ids present in old but absent in new.
  // These instructions have been removed from the scene.
  for id in old_by_id.keys():
    if id not in new_by_id:
      erase(old_by_id[id], context)

  // Step 3: Insertions and updates — walk the new scene's instruction list.
  for (i, new_instr) in enumerate(new_scene.instructions):
    if new_instr.id exists AND new_instr.id in old_by_id:
      // Stable identity node — same id exists in old and new.
      old_instr = old_by_id[new_instr.id]
      if deep_equal(new_instr, old_instr):
        // Identical — no rendering work needed. Skip.
        pass
      else:
        // Modified — update in place.
        update(old_instr, new_instr, context)
    else:
      // No id, or id is new.
      // Use positional diffing: compare with the instruction at position i in old.
      if i < old_scene.instructions.length:
        old_instr = old_scene.instructions[i]
        if deep_equal(new_instr, old_instr):
          pass  // same, skip
        else:
          update(old_instr, new_instr, context)
      else:
        // Position i doesn't exist in old — this is a new instruction.
        insert(new_instr, i, context)
```

### The three backend operations

`erase`, `update`, and `insert` are abstract backend operations. Their
implementations are NOT defined in this spec — each backend defines them.

```
erase(instruction, context)
  → Remove the visual output of this instruction from the context.
  → SVG backend: remove the DOM node with matching id attribute.
  → Canvas backend: repaint the instruction's bounding box with the background.

update(old_instruction, new_instruction, context)
  → Replace the visual output of old_instruction with that of new_instruction.
  → SVG backend: update attributes on the existing DOM node.
  → Canvas backend: erase(old) then dispatch(new).

insert(instruction, position, context)
  → Add a new instruction at the given position (for z-order correctness).
  → SVG backend: insertBefore() the SVG node at the right position.
  → Canvas backend: may require repainting instructions above position.
```

The diff algorithm is backend-agnostic — it only calls these three operations.
A backend that implements these three operations correctly gets patch() for free.

### Limitations of the diff

The current algorithm is O(n) in the number of instructions (not O(n²)).
It does NOT detect:

- **Reordering** — if instruction A moves from position 3 to position 7, the
  diff treats positions 3 and 7 as independent updates. A future LCS-based diff
  (like Myers' diff algorithm) could detect reordering. This is deferred to P2D06.

- **Cross-container moves** — if a node moves from one PaintGroup to another,
  the current diff treats it as a deletion from the first group and an insertion
  in the second. Semantically correct, but not the most efficient path.

These limitations are acceptable for the initial implementation. The fix is to
add ID-based tracking that searches the entire tree, which belongs in the scene
graph layer (P2D06), not in the VM.

---

## UnknownInstructionError — The Right Choice

When the dispatch table has no handler for an instruction kind, PaintVM throws
`UnknownInstructionError` immediately. There is no fallback, no silent skip,
no default handler.

This is a deliberate design choice. Here is why.

### Backends are specializations, not general-purpose engines

A terminal renderer cannot render a PaintGradient. Terminals (in general) support
no more than 256 colors and no sub-character precision. A gradient would require
interpolating across pixel positions that don't exist in a terminal.

The terminal backend can choose to:
1. **Register no gradient handler** — any scene that uses gradients crashes.
2. **Register a degradation handler** — replace the gradient with a flat color.

Option 1 is the default. Option 2 is opt-in.

This means: if you're building a barcode renderer and you accidentally use a
gradient (because you copy-pasted from a chart renderer), the terminal backend
crashes immediately during development, telling you exactly which instruction
was unhandled. Without this crash, the gradient would silently disappear and
you'd spend an hour debugging why the barcode output looks wrong.

### Correct behavior is loud failure

```
// Option A — crash immediately (PaintVM default):
UnknownInstructionError: No handler registered for instruction kind: 'gradient'
  at PaintVM.dispatch (paint-vm.ts:42)
  at PaintVM.execute (paint-vm.ts:28)
  at renderBarcode (barcode-renderer.ts:91)

// Option B — silent skip (wrong):
// [no output, no error, gradient is invisible]
// Developer spends hours debugging missing color band in barcode
```

Option A finds the bug in 5 seconds. Option B hides it indefinitely.

This matches the principle of **making invalid states unrepresentable**: a Metal VM
that encounters an `svg:marker` instruction is in an invalid state — it should
fail loudly, not silently produce wrong output.

### Opt-in graceful degradation

For backends that need to survive unknown instruction kinds (e.g., a logging
proxy that forwards instructions to multiple backends), the backend can register
a catch-all handler using a wildcard mechanism:

```typescript
// Opt-in catch-all — NOT registered by default
vm.register("*", (instruction, context, vm) => {
  console.warn(`Skipping unknown instruction kind: ${instruction.kind}`);
});
```

This is opt-in. No backend registers a catch-all by default. The crash is the
correct default.

---

## Backend Implementation Guide

To create a new PaintVM backend, follow these steps:

### Step 1: Decide on TContext

Choose the context type that your backend's handlers will operate on.

```typescript
// SVG backend example
type MyContext = SVGElement;  // the root <svg> node

// Canvas backend example
type MyContext = CanvasRenderingContext2D;

// Terminal backend example
type MyContext = { buffer: string[][]; width: number; height: number };
```

### Step 2: Implement clear()

`clear()` is called by `execute()` before any instructions are dispatched.
It initializes the context to a blank state with the given background color.

```typescript
// Canvas backend clear()
function clear(ctx: CanvasRenderingContext2D, background: string, w: number, h: number) {
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = background;
  ctx.fillRect(0, 0, w, h);
}

// SVG backend clear()
function clear(root: SVGElement, background: string, w: number, h: number) {
  // Remove all children
  while (root.firstChild) root.removeChild(root.firstChild);
  root.setAttribute("width", String(w));
  root.setAttribute("height", String(h));
  root.style.background = background;
}
```

### Step 3: Write handlers for each supported kind

Each handler function takes `(instruction, context, vm)` and produces the
rendering output for one instruction. Container handlers recurse via `vm.dispatch()`.

```typescript
// Minimal Canvas rect handler
function handleRect(
  instr: PaintRect,
  ctx: CanvasRenderingContext2D,
  _vm: PaintVM<CanvasRenderingContext2D>
) {
  if (instr.fill) {
    ctx.fillStyle = instr.fill;
    ctx.fillRect(instr.x, instr.y, instr.width, instr.height);
  }
  if (instr.stroke) {
    ctx.strokeStyle = instr.stroke;
    ctx.lineWidth = instr.stroke_width ?? 1;
    ctx.strokeRect(instr.x, instr.y, instr.width, instr.height);
  }
}
```

### Step 4: Implement erase(), update(), insert() for patch() support

If your backend supports patch(), implement these three operations.
A backend that only supports execute() can skip them.

### Step 5 (optional): Implement export()

`export()` is optional — backends that have no way to read back pixels (e.g., a
live SVG DOM or a terminal) can leave it unimplemented and throw
`ExportNotSupportedError`.

Backends that can read back pixels (Canvas via `getImageData`, Metal via
`MTLTexture.getBytes`, Cairo via `cairo_image_surface_get_data`) should implement
it by:

1. Allocating an offscreen surface at `scene.width * scale` × `scene.height * scale`.
2. Creating an internal context for that surface.
3. Calling `execute(scene, offscreen_context)`.
4. Reading back the pixels from the offscreen surface.
5. Wrapping them in a `PixelContainer` and returning.

```typescript
// Canvas backend export() — uses OffscreenCanvas
export function exportScene(
  vm: PaintVM<CanvasRenderingContext2D>,
  scene: PaintScene,
  options: ExportOptions = {}
): PixelContainer {
  const scale = options.scale ?? 1.0;
  const w = Math.round(scene.width  * scale);
  const h = Math.round(scene.height * scale);

  const offscreen = new OffscreenCanvas(w, h);
  const ctx = offscreen.getContext("2d")!;
  if (scale !== 1.0) ctx.scale(scale, scale);

  vm.execute(scene, ctx);

  const imageData = ctx.getImageData(0, 0, w, h);  // reads RGBA pixels
  return {
    width: w,
    height: h,
    channels: 4,
    bit_depth: 8,
    pixels: new Uint8Array(imageData.data.buffer),
    color_space: "srgb",
  };
}
```

### Step 6: Export a factory function

```typescript
// The factory function creates and configures a complete, ready-to-use VM.
export function createCanvasVM(): PaintVM<CanvasRenderingContext2D> {
  const vm = new PaintVM<CanvasRenderingContext2D>(clear);
  vm.register("rect",      handleRect);
  vm.register("ellipse",   handleEllipse);
  vm.register("path",      handlePath);
  vm.register("glyph_run", handleGlyphRun);
  vm.register("group",     handleGroup);
  vm.register("layer",     handleLayer);
  vm.register("line",      handleLine);
  vm.register("clip",      handleClip);
  vm.register("gradient",  handleGradient);
  vm.register("image",     handleImage);
  return vm;
}
```

Consumers of the Canvas backend call `createCanvasVM()` and get a fully configured
VM. They do not interact with individual handlers.

---

## Relationship to Animation

For animation, the producer calls `execute(scene)` every frame with a freshly
constructed scene. The VM clears and redraws the entire scene. This is correct
and simple.

```
Frame 1: execute(scene_t0, ctx)  // time = 0.000s
Frame 2: execute(scene_t1, ctx)  // time = 0.016s  (60fps)
Frame 3: execute(scene_t2, ctx)  // time = 0.033s
...
```

Each scene is a complete, self-contained snapshot. The producer is responsible
for computing updated values (easing, physics, tweening). PaintVM has no concept
of time, keyframes, or interpolation — it only executes what it's given.

For complex retained scenes with many stable elements, the producer uses
`patch(old, new)` to send only changes. This requires the producer to retain a
reference to the previous scene:

```
let prev_scene = initial_scene;
on_data_update(new_data):
  next_scene = build_scene(new_data)       // reconstruct from scratch
  vm.patch(prev_scene, next_scene, ctx)    // diff and repaint only changes
  prev_scene = next_scene                   // advance the retained state
```

The scene graph layer (P2D06) wraps this pattern and manages the retained tree
automatically. Producers that use the scene graph call `update_node(id, changes)`
instead of reconstructing the full scene, and the scene graph calls `patch()`
internally.

---

## Relationship to the GenericVM in the Monorepo

The `virtual-machine` package implements a `GenericVM<Opcode, TState>` that maps
numeric opcodes to handler functions. PaintVM applies the same pattern to string
instruction kinds.

Key differences:

| Aspect            | GenericVM (bytecode)       | PaintVM (paint)              |
|-------------------|----------------------------|------------------------------|
| Dispatch key      | Numeric opcode (u8/u16)    | String kind ("rect", "path") |
| Instruction shape | Fixed-width binary         | Structured record (TypeScript interface) |
| Children          | Linear instruction stream  | Nested tree (group, clip)    |
| Context           | VM registers + memory      | Backend rendering context    |
| Error on unknown  | Segfault / InvalidOpcode   | UnknownInstructionError      |

The fundamental pattern — "register a handler per key, dispatch at runtime,
crash on unknown key" — is the same. A developer who understands GenericVM
understands PaintVM.

---

## Error Conditions

| Error                       | When                                                    | How to fix                                |
|-----------------------------|---------------------------------------------------------|-------------------------------------------|
| `UnknownInstructionError`   | `dispatch()` is called with an unregistered kind        | Register a handler for that kind          |
| `DuplicateHandlerError`     | `register()` is called twice for the same kind         | Remove the duplicate registration         |
| `NullContextError`          | `execute()` or `patch()` is called with a null context | Pass a valid context object               |
| `MalformedSceneError`       | `instructions` is null or not an array                 | Validate scene with PaintScene type guard |
| `ExportNotSupportedError`   | `export()` called on a backend that cannot read pixels | Use a backend that supports pixel readback (Canvas, Metal, Cairo) |

These are all programming errors. They indicate a bug in the backend or producer
code, not a runtime data error. They should never be caught and ignored.

---

## Testing a PaintVM Backend

A backend is correct if, for every valid PaintScene, `execute(scene, ctx)` produces
output that is visually equivalent to what a reference backend (e.g., the Canvas
backend) would produce.

### Recommended test strategy

**1. Unit tests per handler** — test each handler in isolation with a mock context.
   Assert that the handler calls the right context methods with the right arguments.

**2. Snapshot tests** — for SVG and terminal backends, execute a set of reference
   scenes and compare the output string to a stored snapshot. Snapshot diffs are
   easy to review in PRs.

**3. Round-trip tests for patch()** — execute a scene, then patch it to a new scene,
   then compare the result to executing the new scene from scratch. The results
   should be identical.

**4. Crash tests for unknown kinds** — assert that executing a scene with an
   unregistered kind throws `UnknownInstructionError` with the correct `kind` field.

**5. Register collision tests** — assert that calling `register()` twice for the
   same kind throws `DuplicateHandlerError`.

Test coverage must exceed 90% for the VM base class. Handler code targets 85%+.
