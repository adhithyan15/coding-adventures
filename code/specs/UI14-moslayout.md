# moslayout — Component Layout Language

## Overview

`moslayout` is a strictly compiled language for declaring the **structural
arrangement** of a UI component. A `.moslayout` file answers exactly one
question: *how are a component's primitives arranged in space, and how do they
connect to the component's interface?*

It does this by wiring `mosmodel` slot and emit names to a fixed vocabulary of
layout primitives (Box, Row, Column, Text, Image, Spacer, Scroll, Grid). The
compiler knows exactly which primitives exist and what structural properties
each accepts. Anything outside that vocabulary is a compile error.

Three things are explicitly forbidden in `.moslayout` files:

1. **Style properties** — no color, font, border, shadow, opacity, or any other
   visual property. These belong in `.mosstyle`.
2. **Arbitrary logic** — no `if` statements, no loops, no expressions beyond
   slot references. Conditional structure is handled by platform-specific layout
   variants.
3. **Slot mutation** — the layout renders slot values; it cannot change them.

This boundary means a UI engineer can evolve the layout — rearrange primitives,
change nesting, produce a completely different mobile variant — without touching
the mosmodel interface or the mosstyle file.

---

## Position in the Stack

```
mosmodel (.mosmodel)          ← UI13-mosmodel.md
     │  exports: slot names, emit names
     ▼
moslayout (.moslayout)        ← THIS SPEC
     │  exports: part names
     ▼
mosstyle (.mosstyle)          ← UI15-mosstyle.md
     │  references: part names, token names
     ▼
backend emitter
```

The `moslayout` compiler imports the interface descriptor from `mosmodel` and
validates every slot and emit reference against it. It exports a **part map** —
named layout nodes that `mosstyle` can style independently.

---

## §1 Platform Variants

A component may have multiple layout files, one per target platform:

```
Button.mosmodel              ← universal interface
Button.moslayout             ← default layout (used if no better match)
Button.desktop.moslayout     ← desktop override
Button.mobile.moslayout      ← mobile override
Button.watch.moslayout       ← watch override
```

The compiler selects the most specific layout file for the target backend. If
`Button.mobile.moslayout` exists and the target is iOS, it is used. If not, it
falls back to `Button.moslayout`. All variants are validated against the same
`Button.mosmodel` interface — they all reference the same slot and emit names.

Platform tokens used in file names:

| Token | Targets |
|---|---|
| `desktop` | macOS, Windows, Linux |
| `mobile` | iOS, Android |
| `tablet` | iPadOS, Android tablet |
| `watch` | watchOS, Wear OS |
| `tv` | tvOS, Android TV |

---

## §2 Primitives

The primitive vocabulary is closed. The compiler knows every primitive. Any
identifier used in layout position that is not in this table is a compile error.

### Box

A rectangular container. The foundational primitive. Everything else composes
from Box.

```
Box { … }
Box [ part-name ] { … }
```

Structural properties (all optional):

| Property | Type | Description |
|---|---|---|
| `direction` | `row` \| `column` | Flex direction. Default: `column`. |
| `align` | `start` \| `center` \| `end` \| `stretch` | Cross-axis alignment. Default: `stretch`. |
| `justify` | `start` \| `center` \| `end` \| `space-between` \| `space-around` | Main-axis distribution. Default: `start`. |
| `wrap` | `bool` | Allow children to wrap. Default: `false`. |
| `grow` | `number` | Flex grow factor. Default: `0`. |
| `shrink` | `number` | Flex shrink factor. Default: `1`. |
| `clip` | `bool` | Clip overflowing children. Default: `false`. |
| `focusable` | `bool` | Whether Box can receive keyboard focus. Default: `false`. |
| `connects` | `emit-name` | Wires a mosmodel emit to this Box's native click event. |

### Row

Shorthand for `Box(direction: row)`. Accepts the same structural properties as
Box except `direction`.

```
Row { … }
Row [ part-name ] { … }
```

### Column

Shorthand for `Box(direction: column)`. Accepts the same structural properties
as Box except `direction`.

```
Column { … }
Column [ part-name ] { … }
```

### Text

Renders a text value from a slot.

```
Text ( slot: <slot-name> )
Text [ part-name ] ( slot: <slot-name> )
```

The slot must be of type `text`. The visual properties of the text (font,
color, size, weight) are declared in `.mosstyle`, not here.

### Image

Renders an image value from a slot.

```
Image ( slot: <slot-name> )
Image [ part-name ] ( slot: <slot-name> )
```

The slot must be of type `image`.

### Spacer

A flexible empty space that expands to fill available room.

```
Spacer
Spacer ( grow: <number> )
```

`grow` defaults to `1`. Multiple Spacers in a flex container divide available
space proportionally by their grow values.

### Scroll

A scrollable container. Children that overflow the scroll axis are clipped; the
scroll direction is navigable.

```
Scroll { … }
Scroll [ part-name ] { … }
Scroll ( direction: horizontal ) { … }
```

| Property | Type | Description |
|---|---|---|
| `direction` | `vertical` \| `horizontal` \| `both` | Scroll axis. Default: `vertical`. |
| `connects-scroll` | `emit-name` | Wires a mosmodel emit to the scroll position change event. |

### Grid

A high-performance virtual-scrolling grid primitive. The host provides only the
visible viewport slice; Grid manages scrolling internally and fires the `onScroll`
emit when the viewport must change.

```
Grid [ part-name ] (
  headers:        slot: <slot-name> ,
  widths:         slot: <slot-name> ,
  rows:           slot: <slot-name> ,
  selected-row:   slot: <slot-name> ,
  selected-col:   slot: <slot-name> ,
  edit-row:       slot: <slot-name> ,
  edit-col:       slot: <slot-name> ,
  edit-content:   slot: <slot-name> ,
  on-navigate:    emit: <emit-name> ,
  on-edit-start:  emit: <emit-name> ,
  on-edit-commit: emit: <emit-name> ,
  on-edit-cancel: emit: <emit-name> ,
  on-scroll:      emit: <emit-name>
)
```

All bindings are optional; unbound slots use zero values, unbound emits are
silently ignored. The Grid primitive is implemented natively per backend — it is
not composed from smaller primitives.

---

## §3 Part Names

A **part** is a named layout node. Parts are the hook that `mosstyle` uses to
apply visual properties to specific elements. Without a part name, a node is
anonymous and cannot be individually styled.

Part names are declared inline with square brackets:

```
Box [ container ] {
  Image [ icon ] ( slot: icon )
  Text  [ label ] ( slot: label )
}
```

This exports three parts: `container`, `icon`, `label`. The mosstyle compiler
validates that every part name it references exists in this export list.

Part names follow `kebab-case`. They must be unique within a component layout.
The compiler rejects duplicate part names.

---

## §4 Emit Wiring

Emits declared in `mosmodel` are wired to primitive events using the `connects`
property. The compiler validates that the emit name exists in the interface
descriptor.

```
Box [ button-root ] ( focusable: true, connects: onClick ) {
  Text [ label ] ( slot: label )
}
```

Here, the native click event on `button-root` is wired to the `onClick` emit.
Each backend translates this wiring to its native event mechanism:

- **Metal / paint-vm** — pointer-up inside bounds calls the `on_click` closure
- **AppKit** — `NSTrackingArea` triggers the event
- **Web Component** — `addEventListener('click', …)` dispatches `CustomEvent('onClick')`
- **React** — becomes the `onClick` prop

---

## §5 Complete Component Examples

### Button

```moslayout
layout Button {
  Box [ root ] ( direction: row, align: center,
                 focusable: true, connects: onClick ) {
    Image [ icon ]  ( slot: icon )
    Text  [ label ] ( slot: label )
  }
}
```

Exports parts: `root`, `icon`, `label`.

### Button — mobile variant (`Button.mobile.moslayout`)

Same interface, different structure. The mobile variant stacks vertically and
uses a larger touch target.

```moslayout
layout Button {
  Box [ root ] ( direction: column, align: center,
                 focusable: true, connects: onClick ) {
    Image [ icon ]  ( slot: icon )
    Text  [ label ] ( slot: label )
  }
}
```

### FormulaBar

```moslayout
layout FormulaBar {
  Row [ root ] {
    Text  [ address ] ( slot: cell-address )
    Box   [ divider ] {}
    Text  [ formula ] ( slot: formula )
  }
}
```

Exports parts: `root`, `address`, `divider`, `formula`.

### SpreadsheetWorkbook

```moslayout
layout SpreadsheetWorkbook {
  Column [ root ] {
    Row [ toolbar ] {
      // toolbar content provided by nested components
    }

    FormulaBar [ formula-bar ] (
      slot cell-address: slot: active-cell-address ,
      slot formula:      slot: active-cell-formula
    )

    Grid [ cell-grid ] (
      headers:        slot: column-headers ,
      widths:         slot: column-widths  ,
      rows:           slot: viewport-rows  ,
      selected-row:   slot: selected-row   ,
      selected-col:   slot: selected-col   ,
      edit-row:       slot: edit-row       ,
      edit-col:       slot: edit-col       ,
      edit-content:   slot: edit-content   ,
      on-navigate:    emit: onNavigate     ,
      on-edit-start:  emit: onEditStart    ,
      on-edit-commit: emit: onEditCommit   ,
      on-edit-cancel: emit: onEditCancel   ,
      on-scroll:      emit: onScroll
    )

    Row [ status-bar ] {}
  }
}
```

---

## §6 Grammar

### Token file (`moslayout.tokens`)

```
# Keywords (must precede IDENT)
KW_LAYOUT     = "layout"
KW_SLOT       = "slot"
KW_EMIT       = "emit"
KW_BOX        = "Box"
KW_ROW        = "Row"
KW_COLUMN     = "Column"
KW_TEXT       = "Text"
KW_IMAGE      = "Image"
KW_SPACER     = "Spacer"
KW_SCROLL     = "Scroll"
KW_GRID       = "Grid"

KW_DIRECTION  = "direction"
KW_ALIGN      = "align"
KW_JUSTIFY    = "justify"
KW_WRAP       = "wrap"
KW_GROW       = "grow"
KW_SHRINK     = "shrink"
KW_CLIP       = "clip"
KW_FOCUSABLE  = "focusable"
KW_CONNECTS   = "connects"

# Value keywords
KW_ROW_DIR    = "row"
KW_COLUMN_DIR = "column"
KW_START      = "start"
KW_CENTER     = "center"
KW_END        = "end"
KW_STRETCH    = "stretch"
KW_SPACE_BTW  = "space-between"
KW_SPACE_ARD  = "space-around"
KW_VERTICAL   = "vertical"
KW_HORIZONTAL = "horizontal"
KW_BOTH       = "both"
KW_TRUE       = "true"
KW_FALSE      = "false"

# Identifiers and numbers
IDENT         = /[a-zA-Z][a-zA-Z0-9]*/
KEBAB_IDENT   = /[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*/
NUMBER        = /[0-9]+(\.[0-9]+)?/

# Punctuation
LBRACE        = "{"
RBRACE        = "}"
LPAREN        = "("
RPAREN        = ")"
LBRACKET      = "["
RBRACKET      = "]"
COLON         = ":"
COMMA         = ","

# Whitespace and comments — skipped
WHITESPACE    = /\s+/           skip
LINE_COMMENT  = /\/\/[^\n]*/   skip
BLOCK_COMMENT = /\/\*.*?\*\//  skip
```

### Grammar file (`moslayout.grammar`)

```
moslayout_file = layout_def ;

layout_def = KW_LAYOUT IDENT LBRACE { node } RBRACE ;

node = box_node | row_node | column_node | text_node | image_node
     | spacer_node | scroll_node | grid_node | component_node ;

# Part name is optional on every node
part_name = LBRACKET KEBAB_IDENT RBRACKET ;

# Box
box_node = KW_BOX [ part_name ] [ LPAREN box_props RPAREN ]
           LBRACE { node } RBRACE ;

box_props = box_prop { COMMA box_prop } ;

box_prop = KW_DIRECTION COLON direction_val
         | KW_ALIGN     COLON align_val
         | KW_JUSTIFY   COLON justify_val
         | KW_WRAP      COLON bool_val
         | KW_GROW      COLON NUMBER
         | KW_SHRINK    COLON NUMBER
         | KW_CLIP      COLON bool_val
         | KW_FOCUSABLE COLON bool_val
         | KW_CONNECTS  COLON KEBAB_IDENT ;   # emit name

direction_val = KW_ROW_DIR | KW_COLUMN_DIR ;
align_val     = KW_START | KW_CENTER | KW_END | KW_STRETCH ;
justify_val   = KW_START | KW_CENTER | KW_END | KW_SPACE_BTW | KW_SPACE_ARD ;
bool_val      = KW_TRUE | KW_FALSE ;

# Row — shorthand for Box(direction: row)
row_node = KW_ROW [ part_name ] [ LPAREN box_props RPAREN ]
           LBRACE { node } RBRACE ;

# Column — shorthand for Box(direction: column)
column_node = KW_COLUMN [ part_name ] [ LPAREN box_props RPAREN ]
              LBRACE { node } RBRACE ;

# Text
text_node = KW_TEXT [ part_name ] LPAREN KW_SLOT COLON KEBAB_IDENT RPAREN ;

# Image
image_node = KW_IMAGE [ part_name ] LPAREN KW_SLOT COLON KEBAB_IDENT RPAREN ;

# Spacer
spacer_node = KW_SPACER [ LPAREN KW_GROW COLON NUMBER RPAREN ] ;

# Scroll
scroll_node = KW_SCROLL [ part_name ] [ LPAREN scroll_props RPAREN ]
              LBRACE { node } RBRACE ;

scroll_props = scroll_prop { COMMA scroll_prop } ;

scroll_prop = KW_DIRECTION COLON scroll_dir_val
            | "connects-scroll" COLON KEBAB_IDENT ;

scroll_dir_val = KW_VERTICAL | KW_HORIZONTAL | KW_BOTH ;

# Grid
grid_node = KW_GRID [ part_name ] LPAREN grid_bindings RPAREN ;

grid_bindings = grid_binding { COMMA grid_binding } ;

grid_binding = KEBAB_IDENT COLON ( KW_SLOT | KW_EMIT ) COLON KEBAB_IDENT ;

# Component reference — another mosmodel component used as a child
component_node = IDENT [ part_name ] LPAREN component_bindings RPAREN ;

component_bindings = component_binding { COMMA component_binding } ;

component_binding = KW_SLOT KEBAB_IDENT COLON KW_SLOT COLON KEBAB_IDENT
                  | KW_SLOT KEBAB_IDENT COLON KW_EMIT COLON KEBAB_IDENT ;
```

---

## §7 Compiler Behaviour

### Inputs

1. A `.moslayout` file (or platform-specific variant).
2. The interface descriptor JSON exported by the `mosmodel` compiler for the
   same component.

### Validation

1. **Slot references** — every `slot: <name>` reference must name a slot
   declared in the interface descriptor. Type compatibility is checked: a `Text`
   node's slot must be of type `text`; an `Image` node's slot must be of type
   `image`; a `Grid` binding for `rows` must be of type `list<list<text>>`.
2. **Emit references** — every `connects: <name>` and `emit: <name>` reference
   must name an emit declared in the interface descriptor.
3. **No style properties** — the grammar admits no color, font, border, opacity,
   or similar visual properties. This is enforced structurally — they are not in
   the grammar, so they cannot be written.
4. **Unique part names** — no two nodes in the same layout share a part name.
5. **Valid nesting** — `Text`, `Image`, and `Spacer` are leaf nodes; they may
   not contain children. `Box`, `Row`, `Column`, and `Scroll` are containers.
   `Grid` is a self-contained leaf.
6. **Known primitives** — only the primitives listed in §2 are valid at node
   positions. An unknown identifier at a node position is a compile error.

### Outputs

**1. Part map (internal, JSON)**

```json
{
  "component": "Button",
  "platform": "default",
  "parts": [
    { "name": "root",  "primitive": "Box",   "connects": "onClick" },
    { "name": "icon",  "primitive": "Image"  },
    { "name": "label", "primitive": "Text"   }
  ]
}
```

This is consumed by the `mosstyle` compiler to validate part name references.

**2. Layout IR**

A resolved layout tree (LayoutNode format per `UI02-layout-ir.md`) with slot
values represented as named references rather than concrete values. The backend
emitter resolves named references to actual slot values at render time.

---

## §8 Backend Mapping

Each backend translates the layout IR to its native layout primitives:

| Primitive | Metal / paint-vm | AppKit | Web Component | React | Qt |
|---|---|---|---|---|---|
| Box / Row / Column | LayoutNode (flexbox) | NSStackView | `<div>` with flex CSS | `<div>` with inline style | QBoxLayout |
| Text | PaintGlyphRun | NSTextField (non-editable) | `<span>` | `<span>` | QLabel |
| Image | PaintImage | NSImageView | `<img>` | `<img>` | QLabel (pixmap) |
| Spacer | flex-grow LayoutNode | NSLayoutGuide | `<div style="flex:1">` | `<div style="flex:1"}>` | QSpacerItem |
| Scroll | LayoutNode + clip | NSScrollView | `<div style="overflow:auto">` | `<div style="overflow:auto">` | QScrollArea |
| Grid | Grid primitive (native per backend) | NSTableView | `<canvas>` + ARIA grid | virtualized div grid | QTableView |

---

## §9 Error Messages

| Error | Condition | Example message |
|---|---|---|
| `UnknownSlot` | `slot: foo` where `foo` not in interface | `Unknown slot 'foo' at line 8 — Grid declares no slot named 'foo'` |
| `UnknownEmit` | `connects: bar` where `bar` not in interface | `Unknown emit 'bar' at line 12 — Button declares no emit named 'bar'` |
| `SlotTypeMismatch` | Slot type incompatible with primitive | `Text node at line 6 requires a text slot, but 'total-rows' is number` |
| `DuplicatePart` | Two nodes share a part name | `Duplicate part name 'label' at line 15` |
| `LeafHasChildren` | Children inside Text, Image, or Spacer | `Text is a leaf node and cannot have children at line 9` |
| `UnknownPrimitive` | Unknown identifier in node position | `Unknown primitive 'Divider' at line 20 — valid primitives are: Box, Row, Column, Text, Image, Spacer, Scroll, Grid` |
| `UnknownComponent` | Component reference not in library | `Component 'FormulaBar' not found in component library at line 25` |

---

## §10 Relationship to Other Specs

- **UI02-layout-ir.md** — the internal LayoutNode tree format that the moslayout
  compiler produces as its layout IR output.
- **UI03-layout-flexbox.md** — the flexbox algorithm that resolves the layout IR
  to positioned nodes for backends that use flexbox (paint-vm, web).
- **UI13-mosmodel.md** — the interface descriptor that this compiler imports for
  slot and emit validation.
- **UI15-mosstyle.md** — imports the part map that this compiler exports.

---

## §11 Out of Scope

- Style properties of any kind — see `mosstyle`
- Animations and transitions — see `mosstyle`
- Conditional rendering based on slot values — the host changes slot values;
  the layout always renders its full structure. Visibility as a style concern
  (opacity: 0, display: none) belongs in `mosstyle` state declarations.
- Computed layout values (e.g. `grow: total-rows / 10`) — all structural
  property values are static literals
- Arbitrary nested component composition beyond the one level shown in
  `component_node` — deep trees are assembled by the host application
