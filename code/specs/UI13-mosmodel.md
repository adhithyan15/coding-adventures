# mosmodel — Component Interface Language

## Overview

`mosmodel` is a small, strictly compiled language for declaring the **external
interface** of a UI component. A `.mosmodel` file answers exactly one question:
*what does the outside world need to know to use this component?*

It answers that question with two constructs:

- **slots** — named, typed values the host pushes *into* the component
- **emits** — named, typed events the component fires *out* to the host

Nothing else belongs in a `.mosmodel` file. No layout. No style. No behavior.
No conditional logic. The compiler rejects everything that is not a slot or emit
declaration.

This strict boundary is the point. A VisiCalc developer who wants to embed a
`Grid` component reads one file, sees slots and emits, and knows everything they
need. They never open the layout or style files. The contract is complete and
self-contained.

---

## Position in the Stack

```
mosmodel (.mosmodel)         ← THIS SPEC
     │  declares interface
     ▼
moslayout (.moslayout)       ← UI14-moslayout.md
     │  references slot/emit names, arranges primitives
     ▼
mosstyle (.mosstyle)         ← UI15-mosstyle.md
     │  references part names, declares visual appearance
     ▼
backend emitter
     │  collapses all three into one output file
     ▼
Rust struct / Swift class / JSX component / Qt QObject / …
```

The three compilers run in dependency order. `moslayout` imports the slot and
emit names that `mosmodel` exports. `mosstyle` imports the part names that
`moslayout` exports. The backend emitter combines all three resolved artefacts
into a single target-language file. **No three-file split exists at runtime.**
The separation is authoring-time only.

---

## Design Principles

### Principle 1 — The interface is the only public API

Every other file in a component's source tree is an implementation detail.
`moslayout` can change completely — mobile and desktop layouts may be entirely
different — without the interface changing. `mosstyle` can be swapped for a
different theme. Only `.mosmodel` is stable and public.

### Principle 2 — One direction per construct

Slots carry data *inward*. Emits carry signals *outward*. There is no two-way
binding, no observable, no reactive stream at the interface level. The host
pushes new slot values whenever state changes. The component fires emits when
something happens. The host handles emits and decides what to do next. This is
the MVVM View boundary enforced by grammar.

### Principle 3 — The compiler is the enforcer

Cultural conventions erode. Code review misses things. A grammar that cannot
express style properties in a slot declaration is an enforcer that never sleeps
and cannot be overridden by a deadline.

### Principle 4 — Backend-agnostic by construction

A `.mosmodel` file has no knowledge of any backend. The same file drives code
generation for Rust structs, Swift classes, Qt QObjects, React props, and Web
Component attributes. Adding a new backend never requires touching `.mosmodel`
source files.

---

## §1 Slot Declarations

A slot is a named, typed input channel. The host sets it. The component reads
it. The component never writes to its own slots.

### Syntax

```
slot <name> : <type> ;
slot <name> : <type> = <default> ;
```

Names follow `kebab-case`. The optional `= <default>` provides a value used
when the host does not set the slot. Slots without defaults are **required** —
the compiler rejects a component instantiation that omits a required slot.

### Slot types

| Type | Description | Example value |
|---|---|---|
| `text` | UTF-8 string | `"Hello"` |
| `number` | 64-bit floating point | `42`, `3.14` |
| `bool` | boolean | `true`, `false` |
| `image` | opaque image reference | backend-specific handle |
| `color` | RGBA color value | `#4a90d9`, `rgba(74,144,217,0.5)` |
| `list<T>` | homogeneous ordered list | `list<text>`, `list<number>` |
| `node` | an arbitrary composed component | another component instance |
| `<ComponentName>` | a specific component type | `CellData`, `ColumnDef` |

`list<T>` is the only parameterized type. `T` may be any scalar type or a named
component type. Nested lists (`list<list<text>>`) are valid for table data.

### Named component slot types

A slot may declare its type as another mosmodel component name:

```
slot row-data : CellData ;
```

The compiler resolves `CellData` by looking for `CellData.mosmodel` in the same
component library. This enforces that the host passes structurally correct data,
not an arbitrary object.

### Slot examples

```mosmodel
// A label the host provides as text
slot label : text ;

// An optional subtitle with a default
slot subtitle : text = "" ;

// A count used for display
slot total-rows : number ;

// Whether the component is interactive
slot disabled : bool = false ;

// A list of column header strings
slot column-headers : list<text> ;

// A two-dimensional list — viewport rows for a grid
slot viewport-rows : list<list<text>> ;

// A specific typed data record
slot active-cell : CellAddress ;
```

---

## §2 Emit Declarations

An emit is a named, typed output channel. The component fires it. The host
handles it. The component never receives the result of an emit. There is no
return value. There is no self-mutation triggered by an emit.

### Syntax

```
emit <name> ;
emit <name> ( <param> : <type> , … ) ;
```

Void emits carry no payload. Typed emits carry a structured payload that the
host receives when handling the event.

### Emit payload types

Payload parameters use the same type vocabulary as slots: `text`, `number`,
`bool`, `color`, and named component types. `image` and `node` are not valid
in emit payloads — events carry data, not rendered subtrees.

### Emit examples

```mosmodel
// A simple click with no payload
emit onClick ;

// Navigation carries the destination cell address
emit onNavigate ( row : number , col : number ) ;

// Edit commit carries the new cell value
emit onEditCommit ( value : text ) ;

// Edit cancel carries no payload
emit onEditCancel ;

// Scroll carries the new viewport offset
emit onScroll ( offset : number ) ;

// Selection change carries the selected range
emit onSelect ( start-row : number , start-col : number ,
                end-row   : number , end-col   : number ) ;
```

---

## §3 Complete Component Examples

### Button

```mosmodel
component Button {
  slot label   : text ;
  slot icon    : image ;
  slot disabled : bool = false ;

  emit onClick ;
  emit onLongPress ;
}
```

### Grid

```mosmodel
component Grid {
  // What to show
  slot column-headers  : list<text> ;
  slot column-widths   : list<number> ;
  slot total-rows      : number ;

  // What the host has scrolled to
  slot viewport-offset : number = 0 ;
  slot viewport-rows   : list<list<text>> ;

  // Selection and edit state — host owns, pushes in
  slot selected-row    : number = 0 ;
  slot selected-col    : number = 0 ;
  slot edit-row        : number = -1 ;   // -1 means not editing
  slot edit-col        : number = -1 ;
  slot edit-content    : text   = "" ;

  // Navigation
  emit onNavigate ( row : number , col : number ) ;

  // Edit lifecycle
  emit onEditStart  ( row : number , col : number ) ;
  emit onEditCommit ( value : text ) ;
  emit onEditCancel ;

  // Scroll — host must update viewport-offset and viewport-rows in response
  emit onScroll ( offset : number ) ;

  // Selection change
  emit onSelect ( start-row : number , start-col : number ,
                  end-row   : number , end-col   : number ) ;
}
```

### FormulaBar

```mosmodel
component FormulaBar {
  slot cell-address : text ;    // e.g. "A1", "B12"
  slot formula      : text ;    // raw formula string shown in bar
  slot read-only    : bool = false ;

  emit onFormulaChange ( formula : text ) ;
  emit onCommit ;
  emit onCancel ;
}
```

---

## §4 Grammar

The mosmodel grammar is intentionally tiny. Any construct that cannot be
expressed with this grammar does not belong in a `.mosmodel` file.

### Token file (`mosmodel.tokens`)

```
# Identifiers and keywords
IDENT         = /[a-zA-Z][a-zA-Z0-9]*/
KEBAB_IDENT   = /[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*/
NUMBER        = /[0-9]+(\.[0-9]+)?/
STRING        = /"([^"\\]|\\.)*"/
BOOL_LIT      = "true" | "false"

# Punctuation
LBRACE        = "{"
RBRACE        = "}"
LPAREN        = "("
RPAREN        = ")"
COLON         = ":"
SEMICOLON     = ";"
COMMA         = ","
EQUALS        = "="
LANGLE        = "<"
RANGLE        = ">"

# Keywords (must come before IDENT in token ordering)
KW_COMPONENT  = "component"
KW_SLOT       = "slot"
KW_EMIT       = "emit"
KW_LIST       = "list"

# Whitespace and comments
WHITESPACE    = /\s+/               skip
LINE_COMMENT  = /\/\/[^\n]*/       skip
BLOCK_COMMENT = /\/\*.*?\*\//      skip
```

### Grammar file (`mosmodel.grammar`)

```
mosmodel_file = component_def ;

component_def = KW_COMPONENT IDENT LBRACE { member } RBRACE ;

member = slot_decl | emit_decl ;

slot_decl = KW_SLOT KEBAB_IDENT COLON slot_type
            [ EQUALS slot_default ] SEMICOLON ;

slot_type = scalar_type
          | list_type
          | IDENT ;               # named component type

scalar_type = "text" | "number" | "bool" | "image" | "color" | "node" ;

list_type = KW_LIST LANGLE ( scalar_type | IDENT ) RANGLE ;

slot_default = STRING | NUMBER | BOOL_LIT ;

emit_decl = KW_EMIT KEBAB_IDENT
            [ LPAREN emit_param { COMMA emit_param } RPAREN ]
            SEMICOLON ;

emit_param = KEBAB_IDENT COLON emit_payload_type ;

emit_payload_type = "text" | "number" | "bool" | "color" | IDENT ;
```

The grammar is context-free and LL(1). Every construct begins with an
unambiguous keyword (`component`, `slot`, `emit`). No lookahead beyond one
token is required at any decision point.

---

## §5 Compiler Behaviour

### Input

A single `.mosmodel` file.

### Validation

1. **Unique names** — no two slots share a name; no two emits share a name; a
   slot and emit may not share a name.
2. **Valid types** — all slot types and emit payload types resolve to known
   scalar types, `list<T>` with a valid inner type, or a named component found
   in the component library.
3. **Valid defaults** — slot defaults must be type-compatible. A `text` slot
   may default to a string literal. A `number` slot may default to a number
   literal. A `bool` slot may default to `true` or `false`. `image`, `color`,
   `list<T>`, and component types may not have inline defaults (the host always
   supplies them explicitly).
4. **No unknown constructs** — anything that is not a `slot` or `emit`
   declaration inside the component body is a compile error.

### Output

The compiler produces two artefacts:

**1. Interface descriptor (internal, JSON)**

```json
{
  "component": "Grid",
  "slots": [
    { "name": "column-headers", "type": "list<text>", "required": true },
    { "name": "total-rows",     "type": "number",     "required": true },
    { "name": "viewport-offset","type": "number",     "default": 0     },
    { "name": "selected-row",   "type": "number",     "default": 0     }
  ],
  "emits": [
    { "name": "onNavigate",    "params": [{"name":"row","type":"number"},{"name":"col","type":"number"}] },
    { "name": "onEditCommit",  "params": [{"name":"value","type":"text"}] },
    { "name": "onEditCancel",  "params": [] }
  ]
}
```

This descriptor is consumed by the `moslayout` and `mosstyle` compilers and by
each backend emitter.

**2. Target-language binding**

The backend emitter generates one file per target. Examples:

*Rust (paint-vm / Metal backend):*
```rust
pub struct Grid {
    pub column_headers:   Vec<String>,
    pub total_rows:       f64,
    pub viewport_offset:  f64,              // default: 0
    pub selected_row:     f64,              // default: 0
    pub on_navigate:      Option<Box<dyn Fn(f64, f64)>>,
    pub on_edit_commit:   Option<Box<dyn Fn(String)>>,
    pub on_edit_cancel:   Option<Box<dyn Fn()>>,
}

impl Grid {
    pub fn new() -> Self { … }
    pub fn column_headers(mut self, v: Vec<String>) -> Self { … }
    pub fn on_navigate(mut self, f: impl Fn(f64, f64) + 'static) -> Self { … }
    // … builder methods for every slot and emit
}
```

*Web Component (JavaScript):*
```javascript
class GridElement extends HTMLElement {
  set columnHeaders(v) { this._columnHeaders = v; this._render(); }
  set totalRows(v)     { this._totalRows = v;     this._render(); }
  // …
  _fireOnNavigate(row, col) {
    this.dispatchEvent(new CustomEvent('on-navigate', { detail: { row, col } }));
  }
}
customElements.define('mos-grid', GridElement);
```

*React (TypeScript):*
```typescript
export interface GridProps {
  columnHeaders:  string[];
  totalRows:      number;
  viewportOffset?: number;
  selectedRow?:   number;
  onNavigate?:    (row: number, col: number) => void;
  onEditCommit?:  (value: string) => void;
  onEditCancel?:  () => void;
}
export function Grid(props: GridProps): JSX.Element { … }
```

*Swift / AppKit:*
```swift
public class GridView: NSView {
  public var columnHeaders:  [String] = [] { didSet { setNeedsDisplay(bounds) } }
  public var totalRows:      Double   = 0  { didSet { setNeedsDisplay(bounds) } }
  public var onNavigate:     ((Double, Double) -> Void)?
  public var onEditCommit:   ((String) -> Void)?
  public var onEditCancel:   (() -> Void)?
}
```

---

## §6 Error Messages

| Error | Condition | Example message |
|---|---|---|
| `DuplicateName` | Two slots or two emits share a name | `Duplicate slot name 'label' at line 4` |
| `NameConflict` | A slot and emit share a name | `'onClick' is declared as both a slot and an emit at line 6` |
| `UnknownType` | A type name does not resolve | `Unknown type 'CelAddress' — did you mean 'CellAddress'? at line 8` |
| `InvalidDefault` | Default value is type-incompatible | `Slot 'disabled' has type bool but default value "yes" is text at line 3` |
| `NoDefaultForType` | Default provided for non-defaultable type | `Slots of type image cannot have inline defaults at line 5` |
| `UnknownConstruct` | Anything other than slot/emit in component body | `Unexpected token 'Box' — only slot and emit declarations are allowed here at line 9` |
| `MissingComponent` | Named component type not found | `Component type 'CellAddress' not found in component library at line 12` |

All errors include file path, line number, and column number.

---

## §7 Relationship to Other Specs

- **UI14-moslayout.md** — the moslayout compiler imports the interface
  descriptor produced by this compiler. Every slot and emit reference in a
  `.moslayout` file is validated against the descriptor.
- **UI15-mosstyle.md** — the mosstyle compiler does not import the interface
  descriptor directly. It imports the part names that moslayout exports, which
  are derived from the layout's wiring of slot values to named primitives.
- **UI00-mosaic.md** — the original Mosaic spec conflated interface and layout
  in a single `.mosaic` file. `mosmodel` is the strict interface-only successor.
  New components use `.mosmodel` + `.moslayout` + `.mosstyle`. The original
  `.mosaic` format is supported by a compatibility shim in the mosaic compiler
  that splits the file into the three-file representation before compilation.

---

## §8 Out of Scope

The following are explicitly not part of mosmodel:

- Layout primitives (Box, Row, Column, Text, etc.) — see `moslayout`
- Style properties (color, font, border, etc.) — see `mosstyle`
- Animations and transitions — see `mosstyle`
- Platform-specific variants — see `moslayout`
- Behavior, event routing logic, conditional rendering — all belong in the host
  application that wires slots and handles emits
- Validation of slot values (e.g. "row must be ≥ 0") — belongs in the host
- Default slot values for `image`, `color`, `list`, and component types — these
  require the host to supply a concrete value

---

## §9 Future Extensions

- **Slot groups** — a named bundle of related slots (`group viewport { offset, rows, count }`)
  for components with many slots, giving the host a single object to pass.
- **Computed slots** — a slot whose value is derived from other slots at the
  interface level (e.g. `slot page-count = ceil(total-rows / page-size)`).
  Kept out of v1 to preserve the "interface only, no logic" invariant.
- **Slot validation attributes** — `slot row : number where row >= 0` — compile-time
  annotation that the emitter can use to generate runtime assertions at the
  host boundary.
- **Versioning** — `component Grid version 2` — enables breaking changes to the
  interface with explicit version negotiation.
