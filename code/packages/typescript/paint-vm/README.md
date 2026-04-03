# @coding-adventures/paint-vm

Dispatch-table virtual machine for PaintInstructions. Part of the P2D series.

## What it is

`paint-vm` is the execution engine for `paint-instructions` (P2D01). It routes each instruction to a registered handler function at runtime — the same pattern as a bytecode VM that maps opcodes to handlers, except the dispatch key is the string `kind` field.

```
opcode → handler   (bytecode VM)
kind   → handler   (PaintVM, this package)
```

The VM is a **framework**, not a renderer. It provides the routing machinery. Backends fill in the handlers.

## Three operations

| Method | Mode | Description |
|---|---|---|
| `execute(scene, ctx)` | Immediate | Clear + dispatch all instructions |
| `patch(old, next, ctx, callbacks?)` | Retained | Diff by id, call onDelete/onInsert/onUpdate |
| `export(scene, options?)` | Pixel output | Render offscreen → `PixelContainer` |

## Usage

```typescript
import { PaintVM } from "@coding-adventures/paint-vm";
import { paintScene, paintRect } from "@coding-adventures/paint-instructions";

// TContext = whatever the backend's context type is
const vm = new PaintVM<CanvasRenderingContext2D>(
  // clearFn — called by execute() before dispatching
  (ctx, bg, w, h) => {
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, w, h);
  }
);

vm.register("rect", (instr, ctx) => {
  if (instr.kind !== "rect") return;
  if (instr.fill) { ctx.fillStyle = instr.fill; ctx.fillRect(instr.x, instr.y, instr.width, instr.height); }
  if (instr.stroke) { ctx.strokeStyle = instr.stroke; ctx.strokeRect(instr.x, instr.y, instr.width, instr.height); }
});

// ... register handlers for other kinds ...

const scene = paintScene(800, 600, "#fff", [
  paintRect(20, 20, 200, 100, { fill: "#3b82f6" }),
]);

vm.execute(scene, canvas.getContext("2d")!);
```

## Error handling

| Error | When |
|---|---|
| `UnknownInstructionError` | `dispatch()` encounters a kind with no handler |
| `DuplicateHandlerError` | `register()` called twice for the same kind |
| `ExportNotSupportedError` | `export()` called on a backend with no pixel readback |
| `NullContextError` | `execute()` or `patch()` called with null context |

All errors indicate programming bugs, not runtime data errors. Do not catch and ignore them.

## Opt-in graceful degradation

By default, encountering an unregistered kind throws immediately. To opt into silent fallback:

```typescript
vm.register("*", (instr, ctx, vm) => {
  console.warn(`Skipping unknown instruction kind: ${instr.kind}`);
});
```

## Creating a backend

1. Pick `TContext` (e.g. `CanvasRenderingContext2D`, `SVGElement`, `StringBuffer`)
2. Implement `clearFn` — wipe the surface to the background color
3. Register a handler per supported instruction kind
4. Optionally implement `exportFn` for `export()` support
5. Export a factory: `export function createMyVM(): PaintVM<MyContext>`

Use `createCanvasVM()` from `paint-vm-canvas` or `createSvgVM()` from `paint-vm-svg` as reference implementations.
