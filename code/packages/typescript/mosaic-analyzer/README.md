# mosaic-analyzer

Semantic analysis pass for the Mosaic compiler pipeline. Walks the raw
`ASTNode` tree produced by `mosaic-parser` and emits a validated, typed
`MosaicIR` — the intermediate representation consumed by `mosaic-vm` and
all code-generation backends.

## Where it fits in the pipeline

```
.mosaic source
    └─► mosaic-lexer  (tokens)
    └─► mosaic-parser  (ASTNode tree)
    └─► mosaic-analyzer  ← YOU ARE HERE
              └─► MosaicIR (typed IR)
    └─► mosaic-vm  (drives traversal, calls renderer)
    └─► mosaic-emit-react / mosaic-emit-webcomponent
```

## Usage

```typescript
import { analyzeMosaic } from "@coding-adventures/mosaic-analyzer";

const ir = analyzeMosaic(`
  component Button {
    slot label: text
    slot disabled: bool

    Box {
      padding: 8dp;
      Text {
        content: @label;
      }
    }
  }
`);

ir.name;              // "Button"
ir.slots[0].name;     // "label"
ir.slots[0].type;     // { kind: "primitive", name: "text" }
ir.root.nodeType;     // "Box"
ir.root.children[0].nodeType;  // "Text"
```

## MosaicIR types

```typescript
// The top-level IR for one component
interface MosaicComponent {
  name: string;           // "ProfileCard"
  slots: MosaicSlot[];    // ordered slot declarations
  imports: MosaicImport[]; // other components referenced in body
  root: MosaicNode;       // the single root node
}

// A slot declaration  e.g.  slot foo: text
interface MosaicSlot {
  name: string;           // "foo"
  type: MosaicType;       // the resolved type
}

// slot foo: text  →  { kind: "primitive", name: "text" }
// slot xs: list<text>  →  { kind: "list", element: { kind: "primitive", name: "text" } }
// slot btn: Button  →  { kind: "component", name: "Button" }
type MosaicType =
  | { kind: "primitive"; name: "text" | "number" | "bool" | "image" | "node" | "color" }
  | { kind: "list"; element: MosaicType }
  | { kind: "component"; name: string };

// An element node in the tree  e.g.  Box { padding: 16dp; ... }
interface MosaicNode {
  nodeType: string;                 // "Box", "Text", "Image", "Button", etc.
  properties: MosaicProperty[];
  children: MosaicChild[];
}

// A property assignment  e.g.  padding: 16dp;
interface MosaicProperty {
  name: string;   // "padding", "color", "content", "src", "a11y-role"
  value: MosaicValue;
}

// Property values
type MosaicValue =
  | { kind: "literal"; value: string }      // "hello", "#fff"
  | { kind: "number"; value: number; unit: string | null }  // 16, "dp"
  | { kind: "slot_ref"; name: string }      // @slotName
  | { kind: "color"; r: number; g: number; b: number; a: number };  // #rrggbbaa

// Children: nested nodes, when blocks, each blocks
type MosaicChild =
  | { kind: "node"; node: MosaicNode }
  | { kind: "when"; slot: string; body: MosaicNode[] }
  | { kind: "each"; slot: string; itemName: string; body: MosaicNode[] };

// A referenced component import  e.g.  Avatar { ... }  inside body
interface MosaicImport {
  name: string;   // "Avatar"
}
```

## Error handling

`analyzeMosaic` throws an `Error` with a descriptive message if:

- No `component` declaration found in the source
- A slot references an unknown type
- A `@slotName` reference in a property points to an undeclared slot
- `when @x` references an undeclared slot
- `each @xs as item` references an undeclared slot

## Development

```bash
# Run tests with coverage
bash BUILD
```

## Dependencies

- `@coding-adventures/mosaic-parser` — `parseMosaic`, `ASTNode`
- `@coding-adventures/mosaic-lexer` — `tokenizeMosaic`
- `@coding-adventures/grammar-tools` — grammar engine
- `@coding-adventures/lexer` — base lexer
- `@coding-adventures/directed-graph` — dependency graph
- `@coding-adventures/parser` — base parser
- `@coding-adventures/state-machine` — NFA/DFA state machine
