# mosaic-vm

The generic tree-walker that drives all Mosaic compiler backends. `MosaicVM`
traverses a `MosaicComponent` IR, normalizes values (colors → rgba, dimensions
→ px), and calls a `MosaicRenderer` implementation at each event in the walk.

## Where it fits in the pipeline

```
.mosaic source
    └─► mosaic-lexer → mosaic-parser → mosaic-analyzer  (produces MosaicIR)
    └─► mosaic-vm  ← YOU ARE HERE
              └─► MosaicRenderer (React, WebComponent, etc.)
                        └─► EmitResult  { files: OutputFile[] }
```

## Usage

```typescript
import { MosaicVM, MosaicRenderer, EmitResult } from "@coding-adventures/mosaic-vm";
import { analyzeMosaic } from "@coding-adventures/mosaic-analyzer";

// 1. Build a renderer
class MyRenderer implements MosaicRenderer {
  beginComponent(name: string, slots: MosaicSlot[]) { /* ... */ }
  endComponent(): EmitResult { /* return files */ }
  beginNode(nodeType: string, props: ResolvedProperty[]) { /* ... */ }
  endNode() { /* ... */ }
  beginWhen(slotName: string) { /* ... */ }
  endWhen() { /* ... */ }
  beginEach(slotName: string, itemName: string) { /* ... */ }
  endEach() { /* ... */ }
  renderSlotChild(slotName: string, slotType: MosaicType) { /* ... */ }
}

// 2. Run the VM
const ir = analyzeMosaic(source);
const vm = new MosaicVM(ir);
const result = vm.run(new MyRenderer());
// result.files[0].filename  → "MyComponent.tsx"
// result.files[0].content   → generated code
```

## Visitor protocol

`MosaicVM.run(renderer)` walks the IR depth-first and fires these callbacks:

| Callback | When |
|---|---|
| `beginComponent(name, slots)` | Once, before the root node |
| `beginNode(nodeType, props)` | Entering each element node |
| `endNode()` | After all children of a node |
| `beginWhen(slotName)` | Entering a `when @x { ... }` block |
| `endWhen()` | Leaving the when block |
| `beginEach(slotName, itemName)` | Entering an `each @xs as item { ... }` block |
| `endEach()` | Leaving the each block |
| `renderSlotChild(slotName, type)` | For a `@slotRef` used as a child node |
| `endComponent()` | After the root node; must return `EmitResult` |

## Value normalization

Before calling `beginNode`, the VM resolves all property values:

| Raw `MosaicValue` | `ResolvedValue` |
|---|---|
| `{ kind: "literal", value: "hello" }` | `{ kind: "literal", value: "hello" }` (pass-through) |
| `{ kind: "number", value: 16, unit: "dp" }` | `{ kind: "dimension", value: 16, unit: "dp" }` |
| `{ kind: "slot_ref", name: "foo" }` | `{ kind: "slot_ref", name: "foo" }` (pass-through) |
| `{ kind: "color", r, g, b, a }` | `{ kind: "color", r, g, b, a }` (pass-through) |

The `ResolvedProperty` passed to `beginNode` pairs each property name with its
`ResolvedValue`.

## Types exported

```typescript
interface ResolvedValue {
  kind: "literal" | "number" | "dimension" | "slot_ref" | "color";
  value?: string | number;
  unit?: string | null;
  name?: string;
  r?: number; g?: number; b?: number; a?: number;
}

interface ResolvedProperty {
  name: string;
  value: ResolvedValue;
}

interface MosaicRenderer {
  beginComponent(name: string, slots: MosaicSlot[]): void;
  endComponent(): EmitResult;
  beginNode(nodeType: string, props: ResolvedProperty[]): void;
  endNode(): void;
  beginWhen(slotName: string): void;
  endWhen(): void;
  beginEach(slotName: string, itemName: string): void;
  endEach(): void;
  renderSlotChild(slotName: string, slotType: MosaicType): void;
}

interface OutputFile {
  filename: string;
  content: string;
}

interface EmitResult {
  files: OutputFile[];
}
```

## Development

```bash
# Run tests with coverage
bash BUILD
```

## Dependencies

- `@coding-adventures/mosaic-analyzer` — `MosaicComponent`, `MosaicSlot`, `MosaicType`, etc.
- `@coding-adventures/mosaic-parser` — `parseMosaic`
- `@coding-adventures/mosaic-lexer` — `tokenizeMosaic`
- `@coding-adventures/grammar-tools` — grammar engine
- `@coding-adventures/lexer` — base lexer
- `@coding-adventures/directed-graph` — dependency graph
- `@coding-adventures/parser` — base parser
- `@coding-adventures/state-machine` — NFA/DFA state machine
