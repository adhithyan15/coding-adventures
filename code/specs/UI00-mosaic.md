# UI00 — Mosaic: A Component Description Language

## Overview

Mosaic is a purpose-built language for declaring UI component structure. A `.mosaic`
file describes **what** a component looks like — its visual tree, named typed slots,
abstract layout and styling properties, and accessibility annotations. Mosaic files
are **never shipped or interpreted at runtime**. A compiler reads the `.mosaic` file
and emits native code for a target platform: Web Components, React, SwiftUI, Jetpack
Compose, or Rust structs targeting paint-vm.

Mosaic follows the same proven pattern as `.tokens` and `.grammar` files in this repo:
a source-of-truth file that a compiler transforms into checked-in, readable, debuggable
code. Engineers write `.mosaic` files, run the compiler, and then write app logic against
the generated typed interfaces — just as they write `.tokens` files and consume the
generated `_grammar.ts` in their lexer packages.

Lattice is for **styling** (a CSS superset). Mosaic is for **structure** (component
trees). They belong to the same family and are composable: Mosaic components reference
abstract style properties, and a backend can map those to Lattice-generated CSS, SwiftUI
modifiers, Compose Modifier chains, or paint-vm instructions.

```
Layer position in the coding-adventures stack:

  .mosaic source file
      |
  Mosaic Lexer (grammar-tools, mosaic.tokens)
      |
  Mosaic Parser (grammar-tools, mosaic.grammar)
      |
  Mosaic AST (ASTNode tree — generic, from @coding-adventures/parser)
      |
  Mosaic Analyzer (type-check slots, resolve components, validate properties)
      |
  MosaicIR (typed, validated, platform-neutral intermediate representation)
      |
  +------------------+------------------+------------------+------------------+
  | Web Component    | React            | SwiftUI          | paint-vm         |
  | Backend          | Backend          | Backend          | Backend          |
  +------------------+------------------+------------------+------------------+
  | HTML + JS class  | TSX function     | Swift struct     | Rust struct +    |
  | (zero deps)      | component        | conforming View  | PaintInstructions|
  +------------------+------------------+------------------+------------------+
```

### When to use Mosaic

- Declaring reusable UI components that must work across platforms
- Defining the structural contract (slots) between visual components and app logic
- Progressive adoption: write one `.mosaic` component, import the generated code into
  an existing hand-written app, everything else stays as-is

### When NOT to use Mosaic

- One-off layouts that only exist on one platform — write native code directly
- Performance-critical inner loops (game rendering) — use paint-vm directly
- Server-side logic, data processing — Mosaic is only for visual structure


## Design Principles

### 1. No magic data binding

XAML uses `{Binding Path=Name, Mode=TwoWay}` — stringly-typed, fails silently at
runtime. QML embeds JavaScript expressions inline. Slint uses `<=>` reactive bindings.

Mosaic has **none of this**. Slots are typed holes. The host language fills them with
real function calls. If the wrong data shows up, you set a breakpoint on the line where
you passed the value. There is no binding engine, no dependency tracking, no digest
cycle, no hidden update mechanism.

### 2. Structural control flow tied to slots

Mosaic supports `when` (conditional) and `each` (iteration) blocks. These are NOT
a binding language. They are driven by slot values that the host language provides:

```
when @show-header {
  Text { content: "My Tasks"; }
}

each @items as item {
  Row { Text { content: @item; } }
}
```

`show-header` is a `bool` slot the engineer sets via a typed setter in their code.
`items` is a `list<text>` slot the engineer fills with an array. The compiler generates
the platform-native equivalent (`{showHeader && ...}` in React, `if showHeader { ... }`
in SwiftUI, `.map()` in Web Components, `ForEach` in SwiftUI, etc.).

### 3. Compile-time only

`.mosaic` files are source artifacts, like `.tokens` and `.grammar` files. The compiler
transforms them into native code that is **checked into the repository**. The generated
code is readable, debuggable, and editable as a last resort. The `.mosaic` file never
reaches the user's device.

### 4. Incrementally adoptable

You do not rewrite an entire app in Mosaic. You:

1. Write `Card.mosaic`
2. Run the compiler — get `Card.ts` (or `.swift`, `.kt`, `.rs`)
3. Import the generated code into your existing hand-written app
4. Everything else stays as-is

Each `.mosaic` component is independently adoptable. The generated code is a regular
component in the target language — it does not know or care whether the rest of the
app was hand-written or compiled from `.mosaic`.

### 5. Strongly typed with progressive strictness

Every slot is typed. The type system supports **progressive typing** — start with
flexible `node` slots during prototyping, then lock down to specific component types
as the design solidifies:

```
// Day 1: prototyping
slot actions: node;

// Day 30: we know it's always a Button
slot actions: Button;
```

The moment you change `node` to `Button`, the compiler starts enforcing it at every
call site. Type safety increases incrementally, matching the language's incremental
adoption model.

### 6. No security escape hatches

- No eval, no entity expansion, no processing instructions
- The grammar defines exactly what is legal — everything else is a parse error
- Slot values are passed through typed function parameters, never `innerHTML`
- No string interpolation that could become an injection vector
- The `.mosaic` file is never loaded, parsed, or interpreted at runtime


## Language Grammar

### File Extension

`.mosaic` — verified free of collisions with existing file formats.

### Token Definitions (`mosaic.tokens`)

```
# Token definitions for Mosaic — Component Description Language
# @version 1
#
# Mosaic declares UI component structure with named typed slots.
# It compiles to native code per target platform (Web Components,
# React, SwiftUI, Compose, Rust/paint-vm).

escapes: standard

# ============================================================================
# Skip Patterns
# ============================================================================

skip:
  LINE_COMMENT = /\/\/[^\n]*/
  BLOCK_COMMENT = /\/\*[\s\S]*?\*\//
  WHITESPACE = /[ \t\r\n]+/

# ============================================================================
# String Literals
# ============================================================================

STRING = /"([^"\\\n]|\\.)*"/

# ============================================================================
# Numeric Literals with Units
# ============================================================================
# ORDER MATTERS: DIMENSION before NUMBER (same as Lattice/CSS).
# A DIMENSION is a number immediately followed by a unit suffix.

DIMENSION = /-?[0-9]*\.?[0-9]+[a-zA-Z%]+/
NUMBER    = /-?[0-9]*\.?[0-9]+/

# ============================================================================
# Color Literals
# ============================================================================
# Hex colors: #rgb, #rrggbb, #rrggbbaa

COLOR_HEX = /#[0-9a-fA-F]{3,8}/

# ============================================================================
# Keywords
# ============================================================================
# These tokens take priority over IDENT when the text matches exactly.

keywords:
  component
  slot
  import
  from
  as
  text
  number
  bool
  image
  color
  node
  list
  true
  false
  when
  each

# ============================================================================
# Identifiers
# ============================================================================
# Component names, slot names, property names.
# Allows hyphens for CSS-like property names (e.g., corner-radius, a11y-label).

IDENT = /[a-zA-Z_][a-zA-Z0-9_-]*/

# ============================================================================
# Delimiters and Operators
# ============================================================================

LBRACE    = "{"
RBRACE    = "}"
LANGLE    = "<"
RANGLE    = ">"
COLON     = ":"
SEMICOLON = ";"
COMMA     = ","
DOT       = "."
EQUALS    = "="
AT        = "@"
```

### Parser Grammar (`mosaic.grammar`)

```
# Parser grammar for Mosaic — Component Description Language
# @version 1
#
# A .mosaic file declares one component with typed slots and a visual tree.
# No logic, no binding expressions — only structure and abstract properties.
#
# Notation:
#   |     alternation (choice)
#   { }   repetition (zero or more)
#   [ ]   optional (zero or one)
#   ( )   grouping
#   "x"   literal match
# UPPERCASE = token reference, lowercase = rule reference

# ============================================================================
# Top-Level Structure
# ============================================================================
# A Mosaic file contains optional imports followed by exactly one component.

file = { import_decl } component_decl ;

# ============================================================================
# Imports
# ============================================================================
# Import another Mosaic component for use as a slot type or composite node.
#
#   import Button from "./button.mosaic";
#   import Card as InfoCard from "./cards/info.mosaic";

import_decl = KEYWORD IDENT [ KEYWORD IDENT ] KEYWORD STRING SEMICOLON ;

# ============================================================================
# Component Declaration
# ============================================================================
# A component has a name, slot declarations, and a node tree.
#
#   component ProfileCard {
#     slot avatar-url: image;
#     slot display-name: text;
#     slot actions: Button;
#
#     Column {
#       Text { content: @display-name; }
#       @actions;
#     }
#   }

component_decl = KEYWORD IDENT LBRACE { slot_decl } node_tree RBRACE ;

# ============================================================================
# Slot Declarations
# ============================================================================
# Each slot has a name, a type, and an optional default value.
#
# Primitive types:     text, number, bool, image, color
# Flexible type:       node (any component — for prototyping or polymorphism)
# Component types:     Button, Badge, Card (imported or self-referencing)
# Collection types:    list<text>, list<Button>, list<node>
#
#   slot title: text;
#   slot count: number = 0;
#   slot visible: bool = true;
#   slot items: list<text>;
#   slot action: Button;
#   slot replies: list<CommentThread>;

slot_decl = KEYWORD IDENT COLON slot_type [ EQUALS default_value ] SEMICOLON ;

slot_type = KEYWORD
          | IDENT
          | list_type ;

list_type = KEYWORD LANGLE slot_type RANGLE ;

default_value = STRING
              | NUMBER
              | DIMENSION
              | COLOR_HEX
              | KEYWORD ;

# ============================================================================
# Node Tree
# ============================================================================
# The visual tree of the component. The root must be exactly one element.

node_tree = node_element ;

node_element = IDENT LBRACE { node_content } RBRACE ;

node_content = property_assignment
             | child_node
             | slot_reference
             | when_block
             | each_block ;

# ============================================================================
# Property Assignments
# ============================================================================
# Properties are name: value pairs. Values can be literals or slot references.
#
#   padding: 16dp;
#   background: #2563eb;
#   content: @title;
#   align: center;
#   style: heading.small;

property_assignment = IDENT COLON property_value SEMICOLON ;

property_value = slot_ref
              | STRING
              | NUMBER
              | DIMENSION
              | COLOR_HEX
              | KEYWORD
              | IDENT
              | enum_value ;

slot_ref = AT IDENT ;

enum_value = IDENT DOT IDENT ;

# ============================================================================
# Child Nodes
# ============================================================================
# Nodes can contain other nodes as children.

child_node = node_element ;

# ============================================================================
# Slot References (as children)
# ============================================================================
# A slot of type node or a component type can appear as a child element.
#
#   Column {
#     @header;
#     Text { content: @body; }
#     @footer;
#   }

slot_reference = AT IDENT SEMICOLON ;

# ============================================================================
# Conditional Rendering
# ============================================================================
# Show a subtree only when a bool slot is true.
# No binding magic — the host language sets the bool via a typed setter.
#
#   when @show-header {
#     Text { content: "Tasks"; style: heading.medium; }
#   }

when_block = KEYWORD slot_ref LBRACE { node_content } RBRACE ;

# ============================================================================
# Iteration
# ============================================================================
# Repeat a subtree for each item in a list slot.
# The host language provides the list via a typed setter.
#
#   each @items as item {
#     Row {
#       Text { content: @item; }
#     }
#   }

each_block = KEYWORD slot_ref KEYWORD IDENT LBRACE { node_content } RBRACE ;
```


## Slot System

### Declaration

Slots are declared at the component level before the node tree:

```
slot title: text;                       // required, no default
slot count: number = 0;                 // optional with default
slot visible: bool = true;              // bool with default
slot avatar: image;                     // image source
slot accent: color = #2563eb;           // color with default
slot action: Button;                    // must be a specific component
slot sidebar: node;                     // any component (flexible)
slot items: list<text>;                 // list of strings
slot cards: list<Card>;                 // list of specific components
slot replies: list<CommentThread>;      // recursive self-reference
slot tags: list<node>;                  // list of any components
```

### Type System

The slot type system supports **progressive typing**:

| Type | Description | Strictness |
|---|---|---|
| `text` | A string value | Primitive |
| `number` | A numeric value | Primitive |
| `bool` | A boolean value | Primitive |
| `image` | An image source (URL, asset path, PixelContainer) | Primitive |
| `color` | A color value (hex, named, or platform-specific) | Primitive |
| `node` | Any component — the flexible/untyped option | Flexible |
| `Button` | Must be a Button — compiler-enforced | Strict |
| `list<T>` | A collection where T is any of the above | Collection |

The upgrade path: start with `node` during prototyping, change to a specific component
type when the design stabilizes. The compiler immediately enforces the new constraint
at every call site.

### Reference

Slots are referenced in the node tree with the `@` prefix:

```
// As a property value (text, number, bool, image, color slots)
Text { content: @title; }
Image { source: @avatar; }

// As a child element (component or node slots)
@action;
@sidebar;

// In each blocks (list slots)
each @items as item {
  Text { content: @item; }
}

// In when blocks (bool slots)
when @visible {
  Text { content: @title; }
}
```

### Generated APIs

For a component with these slots:

```
import Button from "./button.mosaic";

component Card {
  slot title: text;
  slot count: number = 0;
  slot action: Button;
  slot items: list<text>;
  slot expanded: bool = false;
}
```

The compiler generates:

**Web Component (TypeScript):**
```typescript
// AUTO-GENERATED from Card.mosaic — do not edit
export class CardElement extends HTMLElement {
  set title(value: string) { /* update shadow DOM */ }
  set count(value: number) { /* update shadow DOM, default: 0 */ }
  set action(value: ButtonElement) { /* project into slot */ }
  set items(value: string[]) { /* re-render list */ }
  set expanded(value: boolean) { /* toggle conditional, default: false */ }
}
customElements.define("mosaic-card", CardElement);
```

**React (TypeScript):**
```typescript
// AUTO-GENERATED from Card.mosaic — do not edit
import type { ButtonProps } from "./Button.js";

interface CardProps {
  title: string;
  count?: number;
  action: React.ReactElement<ButtonProps>;
  items: string[];
  expanded?: boolean;
}
export function Card(props: CardProps): JSX.Element { /* ... */ }
```

**SwiftUI (Swift):**
```swift
// AUTO-GENERATED from Card.mosaic — do not edit
struct Card: View {
    let title: String
    var count: Int = 0
    let action: Button           // the Mosaic-generated Button type
    let items: [String]
    var expanded: Bool = false

    var body: some View { /* ... */ }
}
```

**Rust / paint-vm:**
```rust
// AUTO-GENERATED from Card.mosaic — do not edit
pub struct Card {
    pub title: String,
    pub count: f64,
    pub action: Button,          // the Mosaic-generated Button type
    pub items: Vec<String>,
    pub expanded: bool,
}

impl Card {
    pub fn render(&self, x: f64, y: f64, width: f64) -> Vec<PaintInstruction> {
        /* layout solver + paint instruction emission */
    }
}
```


## Primitive Node Types

These are the built-in layout and rendering primitives that every backend must support.
Primitive names start with an uppercase letter and are recognized by the analyzer
without an import declaration.

| Node | Purpose | Web (HTML) | SwiftUI | Compose | paint-vm |
|---|---|---|---|---|---|
| `Box` | Generic container, z-axis stacking | `<div>` (position: relative) | `ZStack` | `Box` | `PaintGroup` |
| `Column` | Vertical stack | `<div>` (flex column) | `VStack` | `Column` | layout solver (vertical) |
| `Row` | Horizontal stack | `<div>` (flex row) | `HStack` | `Row` | layout solver (horizontal) |
| `Text` | Text display | `<span>` | `Text` | `Text` | `PaintGlyphRun` |
| `Image` | Image display | `<img>` | `AsyncImage` | `AsyncImage` | `PaintImage` |
| `Spacer` | Flexible empty space | `<div>` (flex: 1) | `Spacer` | `Spacer` | layout solver flex gap |
| `Scroll` | Scrollable container | `<div>` (overflow: auto) | `ScrollView` | `LazyColumn`/`LazyRow` | deferred to host |
| `Divider` | Visual separator line | `<hr>` | `Divider` | `HorizontalDivider` | `PaintLine` |


## Abstract Property System

Mosaic uses its own vocabulary for visual properties. Each property maps to a
platform-specific equivalent through the compiler backend. This is **not CSS** — it is
a curated, unambiguous subset designed to have deterministic behavior across all targets.

### Layout Properties

| Property | Type | Description |
|---|---|---|
| `padding` | dimension | Internal spacing on all sides |
| `padding-left` | dimension | Internal spacing on left side |
| `padding-right` | dimension | Internal spacing on right side |
| `padding-top` | dimension | Internal spacing on top |
| `padding-bottom` | dimension | Internal spacing on bottom |
| `gap` | dimension | Space between children in Column/Row |
| `align` | enum | Child alignment: `start`, `center`, `end`, `stretch`, `center-vertical`, `center-horizontal` |
| `width` | dimension/keyword | Element width. Keywords: `fill` (expand to parent), `wrap` (shrink to content) |
| `height` | dimension/keyword | Element height. Same keywords as width |
| `min-width` | dimension | Minimum width constraint |
| `max-width` | dimension | Maximum width constraint |
| `min-height` | dimension | Minimum height constraint |
| `max-height` | dimension | Maximum height constraint |
| `overflow` | enum | `visible`, `hidden`, `scroll` |

### Visual Properties

| Property | Type | Description |
|---|---|---|
| `background` | color/slot ref | Background fill color |
| `corner-radius` | dimension | Rounded corners on all sides |
| `border-width` | dimension | Border stroke width |
| `border-color` | color | Border stroke color |
| `shadow` | enum | Shadow elevation: `elevation.none`, `elevation.low`, `elevation.medium`, `elevation.high` |
| `opacity` | number (0-1) | Element transparency |
| `visible` | bool/slot ref | Whether element is rendered |

### Text Properties

| Property | Type | Description |
|---|---|---|
| `content` | text/slot ref | The text string to display |
| `style` | enum | Typography style: `heading.large`, `heading.medium`, `heading.small`, `body.large`, `body.medium`, `body.small`, `label`, `caption` |
| `color` | color/slot ref | Text foreground color |
| `max-lines` | number | Maximum number of visible lines (truncate with ellipsis) |
| `text-align` | enum | `start`, `center`, `end` |
| `font-weight` | enum | `normal`, `bold` |

### Image Properties

| Property | Type | Description |
|---|---|---|
| `source` | image/slot ref | Image source (URL, asset reference) |
| `size` | dimension | Width and height (square) |
| `shape` | enum | Clipping shape: `rectangle`, `circle`, `rounded` |
| `fit` | enum | How image fills its bounds: `cover`, `contain`, `fill`, `none` |

### Accessibility Properties

| Property | Type | Description |
|---|---|---|
| `a11y-label` | text/slot ref | Screen reader label |
| `a11y-role` | enum | Semantic role: `button`, `heading`, `image`, `list`, `listitem`, `link`, `none` |
| `a11y-hidden` | bool | Hide from accessibility tree |

### Platform Mapping Examples

Each backend maps abstract properties to native equivalents:

| Mosaic | Web (CSS) | SwiftUI | Compose | paint-vm |
|---|---|---|---|---|
| `padding: 16dp` | `padding: 16px` | `.padding(16)` | `Modifier.padding(16.dp)` | layout solver inset |
| `gap: 8dp` | `gap: 8px` | `spacing: 8` param | `Arrangement.spacedBy(8.dp)` | layout solver gap |
| `corner-radius: 8dp` | `border-radius: 8px` | `.clipShape(RoundedRectangle(cornerRadius: 8))` | `Modifier.clip(RoundedCornerShape(8.dp))` | `PaintRect.corner_radius` |
| `a11y-label: @title` | `aria-label` attribute | `.accessibilityLabel(title)` | `Modifier.semantics { contentDescription = title }` | metadata field |
| `style: heading.small` | CSS class with font rules | `.font(.headline)` | `MaterialTheme.typography.headlineSmall` | font-parser metrics |

### Dimension Units

| Unit | Description | Web | iOS | Android | paint-vm |
|---|---|---|---|---|---|
| `dp` | Density-independent pixels | `px` (with DPI scale) | points | dp | user-space units |
| `sp` | Scale-independent pixels (text) | `px` (with font scale) | points (Dynamic Type) | sp | user-space units |
| `%` | Percentage of parent dimension | `%` | proportional frame | `fillMaxWidth(fraction)` | proportional constraint |

Unitless numbers in layout contexts are treated as `dp`. Unitless numbers in numeric
contexts (e.g., `opacity: 0.5`) are raw numbers.


## Compiler Architecture

The compiler pipeline has five stages with clean interfaces between each.

### Stage 1: Lex (source text → Token[])

- **Input**: `.mosaic` source string
- **Output**: `Token[]` (from `@coding-adventures/lexer`)
- **Engine**: `grammarTokenize(source, MOSAIC_TOKEN_GRAMMAR)`
- **Package**: `mosaic-lexer`

### Stage 2: Parse (Token[] → ASTNode)

- **Input**: `Token[]`
- **Output**: `ASTNode` tree (from `@coding-adventures/parser`)
- **Engine**: `new GrammarParser(tokens, MOSAIC_PARSER_GRAMMAR).parse()`
- **Package**: `mosaic-parser`

### Stage 3: Analyze (ASTNode → MosaicIR)

- **Input**: `ASTNode` tree
- **Output**: `MosaicIR` (typed intermediate representation)
- **Package**: `mosaic-analyzer`

This stage performs semantic analysis:

1. **Resolve imports** — find and parse referenced `.mosaic` files, build component
   registry
2. **Type-check slots** — verify every `@slot-ref` references a declared slot, and the
   slot type matches the context (e.g., `content:` expects `text`, not `number`)
3. **Validate properties** — ensure every property name is valid for the node type, and
   the value type is compatible
4. **Resolve component references** — verify node names and slot types are either
   primitives or imported components (or the component itself for recursion)
5. **Validate when/each** — `when` requires a `bool` slot, `each` requires a `list<T>` slot
6. **Detect errors** — undefined slots, type mismatches, unknown properties, unknown
   components, missing required slots, circular imports

The `MosaicIR` is a typed, validated, platform-neutral data structure:

```typescript
interface MosaicIR {
  component: MosaicComponent;
  imports: MosaicImport[];
}

interface MosaicComponent {
  name: string;
  slots: MosaicSlot[];
  tree: MosaicNode;
}

interface MosaicImport {
  componentName: string;
  alias?: string;
  path: string;
}

interface MosaicSlot {
  name: string;
  type: MosaicType;
  defaultValue?: MosaicValue;
  required: boolean;
}

type MosaicType =
  | { kind: "text" }
  | { kind: "number" }
  | { kind: "bool" }
  | { kind: "image" }
  | { kind: "color" }
  | { kind: "node" }
  | { kind: "component"; name: string }
  | { kind: "list"; elementType: MosaicType };

interface MosaicNode {
  tag: string;
  isPrimitive: boolean;
  properties: MosaicProperty[];
  children: MosaicChild[];
}

type MosaicChild =
  | { kind: "node"; node: MosaicNode }
  | { kind: "slot_ref"; slotName: string }
  | { kind: "when"; slotName: string; children: MosaicChild[] }
  | { kind: "each"; slotName: string; itemName: string; children: MosaicChild[] };

interface MosaicProperty {
  name: string;
  value: MosaicValue;
}

type MosaicValue =
  | { kind: "slot_ref"; slotName: string }
  | { kind: "string"; value: string }
  | { kind: "number"; value: number }
  | { kind: "dimension"; value: number; unit: string }
  | { kind: "color_hex"; value: string }
  | { kind: "bool"; value: boolean }
  | { kind: "ident"; value: string }
  | { kind: "enum"; namespace: string; member: string };
```

### Stage 4: Emit (MosaicIR → target code)

- **Input**: `MosaicIR`
- **Output**: target language source code
- **Package**: one per backend

Each backend implements the same interface:

```typescript
interface MosaicBackend {
  readonly name: string;
  readonly fileExtension: string;
  emit(ir: MosaicIR): MosaicEmitResult;
}

interface MosaicEmitResult {
  files: Array<{
    filename: string;
    content: string;
  }>;
}
```

Some backends emit a single file (React: one `.tsx`). Others may emit multiple files
(Web Component: `.ts` for the element class, optional `.css` for shadow DOM styles).

Backend packages:
- `mosaic-emit-webcomponent` — Web Component (browser-native, zero deps)
- `mosaic-emit-react` — React functional component
- `mosaic-emit-swiftui` — SwiftUI view struct
- `mosaic-emit-compose` — Jetpack Compose function
- `mosaic-emit-paintvm` — Rust struct targeting paint-vm

### Stage 5: Write (MosaicEmitResult → disk)

- **Input**: `MosaicEmitResult`
- **Output**: files written to the output directory, checked into git
- **Package**: `mosaic-compiler` (the top-level pipeline orchestrator)

```typescript
function compileMosaic(
  sourcePath: string,
  backend: MosaicBackend,
  outputDir: string,
): void {
  const source = readFileSync(sourcePath, "utf-8");
  const tokens = tokenizeMosaic(source);
  const ast = parseMosaic(tokens);
  const ir = analyzeMosaic(ast, componentRegistry);
  const result = backend.emit(ir);
  for (const file of result.files) {
    writeFileSync(join(outputDir, file.filename), file.content);
  }
}
```


## Example

A complete example showing a `ProfileCard.mosaic` file and the generated output for
the Web Component backend.

### Input: `ProfileCard.mosaic`

```
// ProfileCard.mosaic — A user profile card component

import Avatar from "./avatar.mosaic";
import Button from "./button.mosaic";
import Badge from "./badge.mosaic";

component ProfileCard {
  slot avatar-url: image;
  slot display-name: text;
  slot bio: text;
  slot verified: bool = false;
  slot action: Button;
  slot badges: list<Badge>;

  Column {
    padding: 16dp;
    gap: 12dp;
    background: #ffffff;
    corner-radius: 8dp;
    shadow: elevation.medium;

    Row {
      gap: 12dp;
      align: center-vertical;

      Avatar {
        source: @avatar-url;
        size: 48dp;
        shape: circle;
      }

      Column {
        gap: 4dp;

        Text {
          content: @display-name;
          style: heading.small;
          font-weight: bold;
        }

        Text {
          content: @bio;
          style: body.medium;
          color: #6b7280;
          max-lines: 2;
        }
      }
    }

    when @verified {
      Row {
        gap: 4dp;
        align: center-vertical;

        Text {
          content: "Verified";
          style: label;
          color: #059669;
        }
      }
    }

    each @badges as badge {
      @badge;
    }

    @action;
  }
}
```

### Output: Web Component (`profile-card.ts`)

```typescript
// AUTO-GENERATED from ProfileCard.mosaic — do not edit
// Regenerate with: mosaic-compiler ProfileCard.mosaic --backend webcomponent

import type { AvatarElement } from "./avatar.js";
import type { ButtonElement } from "./button.js";
import type { BadgeElement } from "./badge.js";

const _template = document.createElement("template");
_template.innerHTML = `
<style>
  :host { display: block; }
  .column-root {
    display: flex; flex-direction: column; padding: 16px; gap: 12px;
    background: #ffffff; border-radius: 8px;
    box-shadow: 0 4px 6px -1px rgba(0,0,0,0.1);
  }
  .row-header { display: flex; flex-direction: row; gap: 12px; align-items: center; }
  .column-name { display: flex; flex-direction: column; gap: 4px; }
  .text-name { font-size: 1rem; font-weight: bold; }
  .text-bio { font-size: 0.875rem; color: #6b7280;
    display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .row-verified { display: flex; flex-direction: row; gap: 4px; align-items: center; }
  .text-verified { font-size: 0.75rem; color: #059669; }
</style>
<div class="column-root">
  <div class="row-header">
    <slot name="avatar-url"></slot>
    <div class="column-name">
      <span class="text-name" id="_display-name"></span>
      <span class="text-bio" id="_bio"></span>
    </div>
  </div>
  <div class="row-verified" id="_verified-block" hidden></div>
  <div id="_badges-container"></div>
  <slot name="action"></slot>
</div>
`;

export class ProfileCardElement extends HTMLElement {
  private _shadow: ShadowRoot;

  constructor() {
    super();
    this._shadow = this.attachShadow({ mode: "open" });
    this._shadow.appendChild(_template.content.cloneNode(true));
  }

  // ── Text slots ──────────────────────────────────────────────

  set displayName(value: string) {
    this._shadow.getElementById("_display-name")!.textContent = value;
  }

  set bio(value: string) {
    this._shadow.getElementById("_bio")!.textContent = value;
  }

  // ── Bool slots ──────────────────────────────────────────────

  set verified(value: boolean) {
    const el = this._shadow.getElementById("_verified-block")!;
    el.hidden = !value;
  }

  // ── Component slots ─────────────────────────────────────────

  set action(value: ButtonElement) {
    value.slot = "action";
    // Clear previous, append new
    for (const child of [...this.children]) {
      if (child.slot === "action") child.remove();
    }
    this.appendChild(value);
  }

  // ── List slots ──────────────────────────────────────────────

  set badges(value: BadgeElement[]) {
    const container = this._shadow.getElementById("_badges-container")!;
    container.innerHTML = "";
    for (const badge of value) {
      container.appendChild(badge);
    }
  }
}

customElements.define("mosaic-profile-card", ProfileCardElement);
```


## Error Reporting

All errors carry file path, line number, and column from the originating token.

| Error | Stage | Example Message |
|---|---|---|
| Unknown character | Lex | `ProfileCard.mosaic:5:12 — Unexpected character '~'` |
| Syntax error | Parse | `ProfileCard.mosaic:8:30 — Expected SEMICOLON after slot declaration` |
| Undefined slot | Analyze | `ProfileCard.mosaic:15:20 — Slot '@username' is not declared in component ProfileCard` |
| Type mismatch | Analyze | `ProfileCard.mosaic:20:14 — Property 'content' expects text slot, got number slot '@count'` |
| Unknown property | Analyze | `ProfileCard.mosaic:22:5 — Unknown property 'foobar' on Text. Did you mean 'font-weight'?` |
| Unknown component | Analyze | `ProfileCard.mosaic:30:5 — Component 'Butto' not found. Did you mean 'Button'?` |
| Missing slot | Analyze | `App.ts:45:3 — Component Button requires slot 'label' but no value was provided` |
| Circular import | Analyze | `ProfileCard.mosaic:1:1 — Circular import: ProfileCard.mosaic -> Avatar.mosaic -> ProfileCard.mosaic` |
| Slot unused | Analyze (warning) | `ProfileCard.mosaic:6:3 — Slot 'subtitle' is declared but never referenced` |
| Invalid when type | Analyze | `ProfileCard.mosaic:25:8 — 'when' requires a bool slot, but '@title' is type text` |
| Invalid each type | Analyze | `ProfileCard.mosaic:30:8 — 'each' requires a list slot, but '@name' is type text` |


## Package Structure

```
code/grammars/
  mosaic.tokens                         # Token definitions
  mosaic.grammar                        # Parser grammar

code/packages/typescript/
  mosaic-lexer/                         # Stage 1: tokenization
    src/_grammar.ts                     # Compiled token grammar
    src/index.ts                        # tokenizeMosaic()

  mosaic-parser/                        # Stage 2: parsing
    src/_grammar.ts                     # Compiled parser grammar
    src/index.ts                        # parseMosaic()

  mosaic-analyzer/                      # Stage 3: semantic analysis
    src/types.ts                        # MosaicIR type definitions
    src/analyzer.ts                     # ASTNode -> MosaicIR
    src/primitives.ts                   # Primitive node registry
    src/properties.ts                   # Abstract property registry
    src/errors.ts                       # Error types with suggestions
    src/index.ts

  mosaic-emit-webcomponent/             # Stage 4: Web Component backend
    src/emitter.ts
    src/index.ts

  mosaic-emit-react/                    # Stage 4: React backend
    src/emitter.ts
    src/index.ts

  mosaic-emit-swiftui/                  # Stage 4: SwiftUI backend
    src/emitter.ts
    src/index.ts

  mosaic-emit-compose/                  # Stage 4: Compose backend
    src/emitter.ts
    src/index.ts

  mosaic-emit-paintvm/                  # Stage 4: Rust/paint-vm backend
    src/emitter.ts
    src/index.ts

  mosaic-compiler/                      # Stage 5: pipeline orchestrator
    src/compiler.ts
    src/index.ts
```

### Dependency Chain

```
mosaic-compiler
  +-- mosaic-analyzer
  |     +-- mosaic-parser
  |     |     +-- mosaic-lexer
  |     |     |     +-- lexer, grammar-tools
  |     |     +-- parser
  |     +-- (no other deps — pure analysis)
  +-- mosaic-emit-webcomponent  (or any backend)
        +-- mosaic-analyzer     (for MosaicIR types only)
```

No backend depends on any other backend. The analyzer is the shared contract.


## Security Model

Mosaic has minimal attack surface by design:

1. **No eval** — the compiler generates static code. No string is ever interpreted as
   code at runtime.

2. **No entity expansion** — unlike XML, there are no entities, no DTDs, no external
   references in the grammar. The parser recognizes exactly what the grammar defines.

3. **No injection** — slot values are passed through typed function parameters. Text
   content uses `textContent` (not `innerHTML`), preventing XSS. Image sources use
   `src` attributes, not embedded data URIs.

4. **Closed grammar** — the `.tokens` and `.grammar` files define exactly what is legal.
   There is no escape hatch, no extension mechanism, no plugin system in the language
   itself.

5. **No network access** — the compiler reads `.mosaic` files from disk. It never
   fetches URLs, downloads assets, or connects to services.

6. **No runtime parsing** — `.mosaic` files are never shipped. There is no runtime
   parser that could be fed malicious input.


## Testing Strategy

### Unit Tests (per package)

1. **mosaic-lexer**: Token stream correctness for all token types. Edge cases: nested
   comments, escaped strings, dimension units with decimals, color hex with alpha.
   Error positions for unknown characters.

2. **mosaic-parser**: AST structure for valid inputs. Parse error messages for invalid
   inputs (missing semicolons, unclosed braces, malformed slot types).

3. **mosaic-analyzer**: Slot type checking (all valid types, all invalid mismatches).
   Property validation (known/unknown, type compatibility). Import resolution (found,
   not found, circular). Component references (primitive, imported, self-referencing).
   `when` block requires `bool` slot. `each` block requires `list` slot. Error message
   quality (line numbers, suggestions).

4. **Each backend emitter**: Snapshot tests — given a fixed `MosaicIR`, the emitted
   code matches a checked-in expected output file. Expected outputs are hand-reviewed.

5. **mosaic-compiler**: End-to-end tests — `.mosaic` file in, target code out, compared
   against snapshots.

### Cross-Backend Consistency

A set of canonical `.mosaic` fixtures that every backend must compile. Not pixel-identical
(each platform renders differently), but structurally equivalent: same slot API, same
component name, same property mappings.

### Compilation Correctness

For the Web Component and React backends, the generated code is compiled by the TypeScript
compiler to verify it is syntactically and type-correct.


## Future Extensions

1. **Theming** — a `theme.mosaic` file defining design tokens (colors, spacing scales,
   typography). Components reference tokens by name. Each backend maps tokens to the
   platform's theming system.

2. **Animation declarations** — static transition descriptions:
   `transition: opacity 200ms ease-in`. Mapped to CSS transitions, SwiftUI `.animation()`,
   Compose `animate*AsState`.

3. **`else` block for `when`** — conditional alternative rendering:
   `when @expanded { ... } else { ... }`.

4. **`when` with enum matching** — beyond bool toggling, match on specific enum values.

5. **Dark mode** — color properties accept light/dark pairs:
   `background: #ffffff | #1a1a1a`. The compiler generates media queries (web),
   trait collections (iOS), or theme-aware resource references (Android).

6. **Bidirectional text** — `direction: auto | ltr | rtl` property. Mapped to `dir`
   attribute (web), `.environment(\.layoutDirection)` (SwiftUI),
   `LocalLayoutDirection` (Compose).

7. **Additional backends** — Flutter (Dart widgets), Qt (QML), terminal (box-drawing
   characters via paint-vm text backend).
