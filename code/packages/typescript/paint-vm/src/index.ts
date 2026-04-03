/**
 * @coding-adventures/paint-vm
 *
 * Dispatch-table virtual machine for PaintInstructions (P2D01).
 *
 * The PaintVM routes each instruction to a registered handler function at
 * runtime. This is the same pattern as a bytecode VM that maps numeric opcodes
 * to handler functions — except here the dispatch key is the string `kind`
 * field on each PaintInstruction.
 *
 * ## The pattern
 *
 *   opcode → handler   (bytecode VM in this repo's virtual-machine package)
 *   kind   → handler   (PaintVM, this package)
 *
 * The VM is a skeleton — it provides the routing machinery, but it has no
 * rendering logic of its own. Backends fill in the handlers.
 *
 * ## Usage
 *
 * ```typescript
 * import { PaintVM } from "@coding-adventures/paint-vm";
 * import type { CanvasRenderingContext2D } from "...";
 *
 * const vm = new PaintVM<CanvasRenderingContext2D>();
 *
 * vm.register("rect", (instr, ctx, vm) => {
 *   if (instr.kind !== "rect") return;
 *   ctx.fillStyle = instr.fill ?? "transparent";
 *   ctx.fillRect(instr.x, instr.y, instr.width, instr.height);
 * });
 *
 * vm.execute(scene, ctx);
 * ```
 *
 * ## Three operations
 *
 * execute(scene, ctx)       — immediate mode: clear + redraw everything
 * patch(old, new, ctx)      — retained mode: diff by id, repaint only changes
 * export(scene, options?)   — offscreen render: returns PixelContainer
 *
 * Backends that cannot read back pixels throw ExportNotSupportedError from export().
 */
export const VERSION = "0.1.0";

import type {
  PaintInstruction,
  PaintScene,
  PixelContainer,
} from "@coding-adventures/paint-instructions";

// ============================================================================
// Error types
// ============================================================================

/**
 * Thrown when execute() or patch() encounters an instruction kind with no
 * registered handler.
 *
 * This is always a programming error — it indicates the backend is incomplete
 * or the producer emitted an instruction kind the backend doesn't support.
 *
 * Why throw instead of silently skip?
 *
 * A Metal backend that encounters an svg:marker instruction is in an invalid
 * state. Silently skipping it would produce wrong output that could take hours
 * to debug. Throwing immediately pinpoints the exact instruction kind that has
 * no handler, making the bug trivially obvious.
 *
 * If you want opt-in graceful degradation, register a wildcard catch-all:
 *   vm.register("*", (instr, ctx, vm) => {
 *     console.warn(`Skipping unknown instruction: ${instr.kind}`);
 *   });
 */
export class UnknownInstructionError extends Error {
  readonly kind: string;

  constructor(kind: string) {
    super(`No handler registered for instruction kind: '${kind}'`);
    this.name = "UnknownInstructionError";
    this.kind = kind;
  }
}

/**
 * Thrown when register() is called twice for the same instruction kind.
 *
 * Double-registration is always a programming error (typo, accidental copy/paste,
 * or two backends registered on the same VM instance). Catching it at registration
 * time — not at execution time — makes the bug immediately obvious.
 */
export class DuplicateHandlerError extends Error {
  readonly kind: string;

  constructor(kind: string) {
    super(`Handler already registered for instruction kind: '${kind}'`);
    this.name = "DuplicateHandlerError";
    this.kind = kind;
  }
}

/**
 * Thrown when export() is called on a backend that cannot read back pixels.
 *
 * Backends like SVG (which produces a string) or terminal (which produces
 * character buffers) cannot return raw RGBA pixels. They should throw this
 * error from their export() implementation.
 *
 * Backends that CAN export (Canvas via getImageData, Metal via MTLTexture.getBytes)
 * implement export() and return a PixelContainer.
 */
export class ExportNotSupportedError extends Error {
  constructor(backendName: string) {
    super(
      `export() is not supported by the ${backendName} backend. ` +
        `Use a backend that supports pixel readback (Canvas, Metal, Cairo).`,
    );
    this.name = "ExportNotSupportedError";
  }
}

/**
 * Thrown when execute() or patch() is called with a null/undefined context.
 */
export class NullContextError extends Error {
  constructor() {
    super("execute() and patch() require a non-null context");
    this.name = "NullContextError";
  }
}

// ============================================================================
// Handler type
// ============================================================================

/**
 * A handler for a single PaintInstruction kind.
 *
 * The handler receives:
 *   instruction — the PaintInstruction to render
 *   context     — the backend's rendering context (TContext)
 *   vm          — the PaintVM instance, so container handlers can recurse
 *
 * Container instructions (group, layer, clip) MUST call vm.dispatch() on each
 * child. The VM does not recurse automatically — each handler is responsible
 * for recursing into its children.
 *
 * This design allows backends to:
 *   - Skip children based on visibility (culling off-screen groups)
 *   - Inject pre/post steps around children (push/pop transform state)
 *   - Handle containers differently (SVG wraps in <g>, terminal just recurses)
 */
export type PaintHandler<TContext> = (
  instruction: PaintInstruction,
  context: TContext,
  vm: PaintVM<TContext>,
) => void;

/**
 * Options for the export() method.
 */
export interface ExportOptions {
  scale?: number; // pixel density multiplier; default 1.0
  channels?: 3 | 4; // 3=RGB, 4=RGBA; default 4
  bit_depth?: 8 | 16; // bits per channel; default 8
  color_space?: "srgb" | "display-p3" | "linear-srgb"; // default "srgb"
}

// ============================================================================
// PaintVM
// ============================================================================

/**
 * The dispatch-table virtual machine for PaintInstructions.
 *
 * PaintVM is generic over TContext — the backend's rendering context type.
 * The VM itself does not know what TContext is. It only passes it to handlers.
 *
 * Examples of TContext:
 *   SVG backend:      TContext = SVGElement (the root <svg> node)
 *   Canvas backend:   TContext = CanvasRenderingContext2D
 *   Metal backend:    TContext = MTLCommandEncoder (via Rust FFI)
 *   Terminal backend: TContext = { buffer: string[][]; width: number; height: number }
 *
 * The TContext generic makes handler type signatures self-documenting: reading
 * `PaintHandler<CanvasRenderingContext2D>` tells you immediately which backend
 * the handler is for, with no casting required inside the handler body.
 *
 * ## Creating a backend
 *
 * 1. Decide on TContext.
 * 2. Create a PaintVM<TContext> instance.
 * 3. Implement clear() — called before execute() to blank the surface.
 * 4. Register a handler for each supported instruction kind.
 * 5. Optionally implement export() for pixel readback.
 * 6. Export a factory function (e.g. createCanvasVM()) that returns a fully
 *    configured VM instance.
 */
export class PaintVM<TContext> {
  // The dispatch table: kind string → handler function.
  // Populated by register(). Queried by dispatch().
  private readonly table: Map<string, PaintHandler<TContext>> = new Map();

  // The backend's clear function: wipe the surface to the background colour.
  // Called by execute() before dispatching any instructions.
  private readonly clearFn: (
    context: TContext,
    background: string,
    width: number,
    height: number,
  ) => void;

  // Optional export function: renders an offscreen copy and returns pixels.
  private readonly exportFn:
    | ((
        scene: PaintScene,
        vm: PaintVM<TContext>,
        options: Required<ExportOptions>,
      ) => PixelContainer)
    | undefined;

  /**
   * Create a PaintVM.
   *
   * clearFn is required — it is called by execute() before any instructions.
   * exportFn is optional — provide it if the backend supports pixel readback.
   *
   * Example:
   *   const vm = new PaintVM<CanvasRenderingContext2D>(
   *     (ctx, bg, w, h) => {
   *       ctx.clearRect(0, 0, w, h);
   *       ctx.fillStyle = bg;
   *       ctx.fillRect(0, 0, w, h);
   *     }
   *   );
   */
  constructor(
    clearFn: (
      context: TContext,
      background: string,
      width: number,
      height: number,
    ) => void,
    exportFn?: (
      scene: PaintScene,
      vm: PaintVM<TContext>,
      options: Required<ExportOptions>,
    ) => PixelContainer,
  ) {
    this.clearFn = clearFn;
    this.exportFn = exportFn;
  }

  /**
   * Register a handler for a specific instruction kind.
   *
   * Calling register() twice for the same kind is a programming error —
   * it throws DuplicateHandlerError immediately (at registration time,
   * not at execution time). This catches typos and accidental double-
   * registration during development.
   *
   * The wildcard kind "*" registers a catch-all handler that handles any
   * instruction kind that has no specific handler registered. This is
   * opt-in: no backend registers a catch-all by default.
   */
  register(kind: string, handler: PaintHandler<TContext>): void {
    if (this.table.has(kind)) {
      throw new DuplicateHandlerError(kind);
    }
    this.table.set(kind, handler);
  }

  /**
   * Dispatch a single instruction to its registered handler.
   *
   * If no handler is registered for the instruction's kind, checks for a
   * wildcard "*" handler. If that is also absent, throws UnknownInstructionError.
   *
   * Container handlers (group, layer, clip) call this method recursively on
   * their children. The VM does not recurse automatically.
   */
  dispatch(instruction: PaintInstruction, context: TContext): void {
    const handler =
      this.table.get(instruction.kind) ?? this.table.get("*");
    if (!handler) {
      throw new UnknownInstructionError(instruction.kind);
    }
    handler(instruction, context, this);
  }

  /**
   * Immediate mode: clear the context and execute all instructions in scene.
   *
   * Steps:
   *   1. Clear the entire context to scene.background.
   *   2. Walk scene.instructions in order, dispatch each one back-to-front
   *      (painter's algorithm — earlier instructions are drawn first).
   *
   * Use execute() when:
   *   - The scene changes completely between frames (game, animation)
   *   - Simplicity matters more than performance
   *   - You do not want to maintain old/new scene state
   */
  execute(scene: PaintScene, context: TContext): void {
    if (context == null) throw new NullContextError();
    this.clearFn(context, scene.background, scene.width, scene.height);
    for (const instruction of scene.instructions) {
      this.dispatch(instruction, context);
    }
  }

  /**
   * Retained mode: diff old and new scenes, execute only the changes.
   *
   * Use patch() when:
   *   - Most of the scene is stable across frames
   *   - Performance matters (60fps charts with live data)
   *   - The producer has assigned stable ids to dynamic instructions
   *
   * patch() implements a structural diff between two PaintScene versions.
   * It executes only the minimal set of backend operations needed to transform
   * the rendered output of `old` into the rendered output of `new`.
   *
   * ## Algorithm
   *
   * Step 1 — Build identity maps (id → instruction) for old and new scenes.
   *           Instructions without id are not in the identity map.
   *
   * Step 2 — Deletions: ids in old but not in new → call onDelete per id.
   *
   * Step 3 — Insertions and updates: walk new scene's instruction list.
   *   - If instruction has an id that exists in old:
   *       if identical → skip (no work)
   *       if changed   → call onUpdate(old, new)
   *   - Otherwise (no id, or new id):
   *       positional diff: compare new[i] with old[i]
   *       if identical → skip; if changed → call onUpdate; if new → call onInsert
   *
   * ## Default implementation
   *
   * The default patch() re-executes the entire new scene (falls back to execute).
   * Backends override this by implementing onDelete, onUpdate, onInsert.
   *
   * For SVG backends, onDelete removes DOM nodes, onUpdate mutates attributes,
   * onInsert adds new nodes. For Canvas backends, onUpdate = erase + redraw.
   *
   * This default is correct but not optimal. Override patch() or the three
   * operation callbacks for performance.
   */
  patch(
    old: PaintScene,
    next: PaintScene,
    context: TContext,
    callbacks?: {
      onDelete?: (instruction: PaintInstruction) => void;
      onInsert?: (instruction: PaintInstruction, position: number) => void;
      onUpdate?: (
        oldInstruction: PaintInstruction,
        newInstruction: PaintInstruction,
      ) => void;
    },
  ): void {
    if (context == null) throw new NullContextError();

    if (!callbacks) {
      // Default: re-execute the entire new scene.
      this.execute(next, context);
      return;
    }

    const { onDelete, onInsert, onUpdate } = callbacks;

    // Step 1: build identity maps
    const oldById = new Map<string, PaintInstruction>();
    const newById = new Map<string, PaintInstruction>();
    for (const instr of old.instructions) {
      if (instr.id) oldById.set(instr.id, instr);
    }
    for (const instr of next.instructions) {
      if (instr.id) newById.set(instr.id, instr);
    }

    // Step 2: deletions — ids present in old but absent in new
    for (const [id, instr] of oldById) {
      if (!newById.has(id)) {
        onDelete?.(instr);
      }
    }

    // Step 3: insertions and updates — walk the new instruction list
    for (let i = 0; i < next.instructions.length; i++) {
      const newInstr = next.instructions[i];

      if (newInstr.id && oldById.has(newInstr.id)) {
        // Stable identity node — same id in both scenes
        const oldInstr = oldById.get(newInstr.id)!;
        if (!deepEqual(newInstr, oldInstr)) {
          onUpdate?.(oldInstr, newInstr);
        }
        // else: identical — skip
      } else {
        // Positional diff
        if (i < old.instructions.length) {
          const oldInstr = old.instructions[i];
          if (!deepEqual(newInstr, oldInstr)) {
            onUpdate?.(oldInstr, newInstr);
          }
        } else {
          // Position i doesn't exist in old — new instruction
          onInsert?.(newInstr, i);
        }
      }
    }
  }

  /**
   * Pixel export: render the scene to an internal offscreen buffer and return
   * the raw pixels as a PixelContainer.
   *
   * export() does NOT use or modify any live context. It allocates its own
   * internal offscreen surface via the exportFn provided at construction time.
   *
   * Backends that support pixel readback (Canvas via OffscreenCanvas + getImageData,
   * Metal via MTLTexture.getBytes) provide an exportFn. Backends that cannot
   * produce pixel data (SVG string output, terminal character buffer) throw
   * ExportNotSupportedError.
   *
   * The returned PixelContainer is owned by the caller — the VM does not retain it.
   * Pass it to an ImageCodec.encode() to compress it into a specific format.
   *
   * Example:
   *   const pixels = vm.export(scene, { scale: 2 });   // 2× Retina
   *   const bytes  = pngCodec.encode(pixels);
   *   fs.writeFileSync("chart.png", bytes);
   */
  export(scene: PaintScene, options?: ExportOptions): PixelContainer {
    if (!this.exportFn) {
      throw new ExportNotSupportedError("this");
    }
    const opts: Required<ExportOptions> = {
      scale: options?.scale ?? 1.0,
      channels: options?.channels ?? 4,
      bit_depth: options?.bit_depth ?? 8,
      color_space: options?.color_space ?? "srgb",
    };
    return this.exportFn(scene, this, opts);
  }

  /**
   * Returns the set of registered instruction kinds.
   * Useful for debugging and testing.
   */
  registeredKinds(): string[] {
    return [...this.table.keys()];
  }
}

// ============================================================================
// deepEqual — structural equality for patch() diffing
// ============================================================================

/**
 * Deep structural equality check.
 *
 * Used by patch() to determine whether an instruction has changed between
 * the old and new scenes. If two instructions are deeply equal, no rendering
 * work is needed for that instruction.
 *
 * This is a recursive JSON-style equality check. It handles:
 *   - Primitive values (number, string, boolean, null, undefined)
 *   - Arrays (element-by-element, order matters)
 *   - Plain objects (key-by-key, insertion order ignored)
 *
 * It does NOT handle:
 *   - Functions, Dates, RegExp, Map, Set, typed arrays (Uint8Array)
 *   - Circular references
 *
 * PixelContainer pixels (Uint8Array) in PaintImage.src are compared by
 * reference identity — two different Uint8Array objects with the same bytes
 * are treated as DIFFERENT. This is intentional: if you create a new
 * PixelContainer, it is considered a changed instruction.
 */
export function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a == null || b == null) return false;
  if (typeof a !== typeof b) return false;

  if (typeof a !== "object") return a === b;

  // Both are objects
  if (Array.isArray(a) !== Array.isArray(b)) return false;

  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (!deepEqual(a[i], b[i])) return false;
    }
    return true;
  }

  // Both are plain objects
  const aObj = a as Record<string, unknown>;
  const bObj = b as Record<string, unknown>;
  const aKeys = Object.keys(aObj);
  const bKeys = Object.keys(bObj);
  if (aKeys.length !== bKeys.length) return false;
  for (const key of aKeys) {
    if (!Object.prototype.hasOwnProperty.call(bObj, key)) return false;
    if (!deepEqual(aObj[key], bObj[key])) return false;
  }
  return true;
}
