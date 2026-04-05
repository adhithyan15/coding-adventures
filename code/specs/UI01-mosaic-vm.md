# UI01 — Mosaic VM and Compiler Backends

## Overview

This document specifies the **forward compiler pipeline**: from a validated `MosaicIR`
(produced by the analyzer described in UI00) to target platform code. The reverse
direction (ingesting existing code into Mosaic) is out of scope for this document.

The central architectural decision is a **generic VM** — a traversal engine that all
backends share. The VM drives the tree walk, normalizes property values, tracks slot
scope, and calls into a backend-provided renderer at each structural event. A backend
is nothing more than an implementation of the `MosaicRenderer` interface.

Two backends are specified here:
- **`mosaic-emit-react`** — emits a TypeScript React functional component (`.tsx`)
- **`mosaic-emit-webcomponent`** — emits a TypeScript Custom Element class (`.ts`)

Additional backends (SwiftUI, Compose, paint-vm) follow the same `MosaicRenderer`
contract and are out of scope for this document.

```
MosaicIR
    |
MosaicVM (mosaic-vm)
    |  drives tree walk, normalizes values, tracks scope
    |
    +------ MosaicReactRenderer --------> ComponentName.tsx
    |       (mosaic-emit-react)
    |
    +------ MosaicWebComponentRenderer -> ComponentName.ts
            (mosaic-emit-webcomponent)
```


## Part 1: `mosaic-vm` — The Generic Traversal Engine

**Package:** `code/packages/typescript/mosaic-vm`

The VM contains no backend-specific logic. It knows how to walk a `MosaicIR` tree,
normalize raw `MosaicValue` objects into typed `ResolvedValue` objects, track which
slots and loop variables are in scope at each point in the tree, and fire events on
a `MosaicRenderer` in depth-first order.

A backend only needs to implement `MosaicRenderer`. The VM handles everything else.


### 1.1 — ResolvedValue: Normalized Property Values

`MosaicValue` (from the IR) contains raw parsed forms — hex strings, dimension strings
with embedded units, etc. The VM parses these before passing them to renderers so no
backend ever needs to implement a hex parser or a unit splitter.

```typescript
// The normalized, fully-typed value the VM passes to renderer methods.
// Backends receive ResolvedValue, never raw MosaicValue.

export type ResolvedValue =
  | { kind: "string";    value: string }
  | { kind: "number";    value: number }
  | { kind: "bool";      value: boolean }
  | { kind: "dimension"; value: number; unit: "dp" | "sp" | "%" }
  | { kind: "color";     r: number; g: number; b: number; a: number }
  | { kind: "enum";      namespace: string; member: string }
  | { kind: "slot_ref";  slotName: string; slotType: MosaicType; isLoopVar: boolean };
```

**Normalization rules:**
- `MosaicValue { kind: "color_hex"; value: "#2563eb" }`
  → `ResolvedValue { kind: "color"; r: 37, g: 99, b: 235, a: 255 }`
  (three-digit hex `#rgb` expands to `#rrggbb`, eight-digit `#rrggbbaa` populates `a`)
- `MosaicValue { kind: "dimension"; value: "16dp" }`
  → `ResolvedValue { kind: "dimension"; value: 16, unit: "dp" }`
- `MosaicValue { kind: "bool"; value: true }`
  → `ResolvedValue { kind: "bool"; value: true }` (unchanged)
- `MosaicValue { kind: "slot_ref"; slotName: "title" }` — the VM looks up the slot
  in the current `SlotContext` to attach the full `slotType` and set `isLoopVar`.
- `MosaicValue { kind: "enum"; namespace: "elevation"; member: "medium" }`
  → `ResolvedValue { kind: "enum"; namespace: "elevation"; member: "medium" }` (unchanged)


### 1.2 — ResolvedProperty

A property after normalization:

```typescript
export interface ResolvedProperty {
  name: string;         // the Mosaic property name, e.g. "corner-radius"
  value: ResolvedValue;
}
```

The VM passes a `ResolvedProperty[]` to the renderer for each node.


### 1.3 — SlotContext: Scope Tracking

The VM maintains a `SlotContext` at every point in the tree walk. It tracks which
slots are in scope so that slot references can be fully resolved.

```typescript
export interface SlotContext {
  // All slots declared on the component being compiled.
  // Key: slot name (e.g. "title"), Value: MosaicSlot definition.
  componentSlots: ReadonlyMap<string, MosaicSlot>;

  // The stack of active `each` loop variables, innermost last.
  // When the VM enters `each @items as item { ... }`, it pushes
  // { itemName: "item", elementType: ... } onto this stack.
  loopScopes: ReadonlyArray<{ itemName: string; elementType: MosaicType }>;
}
```

The VM uses `SlotContext` to resolve every `slot_ref`:
1. If `slotName` matches a `loopScopes` entry (innermost first) → `isLoopVar: true`
2. Otherwise look up in `componentSlots` → `isLoopVar: false`
3. If neither → the analyzer should have caught this; VM throws `MosaicVMError`


### 1.4 — MosaicRenderer: The Backend Protocol

Every backend implements this interface. The VM calls these methods in strict
depth-first, open-before-close order.

```typescript
export interface MosaicRenderer {
  // -----------------------------------------------------------------------
  // Component lifecycle
  // -----------------------------------------------------------------------

  // Called once before any tree traversal.
  // The renderer should initialize its output buffer and generate any
  // file header, import statements, or class preamble.
  beginComponent(name: string, slots: MosaicSlot[]): void;

  // Called once after all tree traversal is complete.
  // The renderer flushes its output buffer and returns the complete file
  // content as a string (or multiple files — see emit() below).
  endComponent(): void;

  // Called by MosaicVM.run() at the very end; returns all generated files.
  emit(): MosaicEmitResult;

  // -----------------------------------------------------------------------
  // Node lifecycle (depth-first, open before close)
  // -----------------------------------------------------------------------

  // Called when entering a node element (primitive or imported component).
  // properties: all ResolvedProperty entries for this node, in source order.
  // context: the SlotContext at this point in the tree.
  beginNode(
    tag: string,
    isPrimitive: boolean,
    properties: ResolvedProperty[],
    context: SlotContext,
  ): void;

  // Called when leaving a node element (after all children processed).
  endNode(tag: string): void;

  // -----------------------------------------------------------------------
  // Children
  // -----------------------------------------------------------------------

  // Called when a slot reference appears as a child of a node:
  //   Column { @action; }
  // (Not called for slot refs used as property values — those appear in
  //  the ResolvedProperty passed to beginNode.)
  renderSlotChild(slotName: string, slotType: MosaicType, context: SlotContext): void;

  // -----------------------------------------------------------------------
  // Conditional rendering
  // -----------------------------------------------------------------------

  // Called when entering a `when @flag { ... }` block.
  // context contains the bool slot `flag`.
  beginWhen(slotName: string, context: SlotContext): void;

  // Called when leaving a `when` block (after all its children).
  endWhen(): void;

  // -----------------------------------------------------------------------
  // Iteration
  // -----------------------------------------------------------------------

  // Called when entering `each @items as item { ... }`.
  // elementType: the T in list<T> — what each `item` loop variable is.
  // After this call, the VM pushes { itemName, elementType } onto SlotContext.loopScopes
  // so that references to `@item` inside the block resolve as loop variables.
  beginEach(
    slotName: string,
    itemName: string,
    elementType: MosaicType,
    context: SlotContext,
  ): void;

  // Called when leaving an `each` block (after all its children).
  // The VM pops the loop scope before calling this.
  endEach(): void;
}

// The result type every backend must return from emit().
export interface MosaicEmitResult {
  files: Array<{
    filename: string; // relative — e.g. "Footer.tsx" or "mosaic-footer.ts"
    content: string;
  }>;
}
```


### 1.5 — MosaicVM: The Driver

```typescript
export class MosaicVM {
  constructor(private ir: MosaicIR) {}

  // Traverse the IR tree, calling renderer methods in order.
  // Returns the MosaicEmitResult from renderer.emit().
  run(renderer: MosaicRenderer): MosaicEmitResult {
    const context: SlotContext = {
      componentSlots: new Map(
        this.ir.component.slots.map((s) => [s.name, s])
      ),
      loopScopes: [],
    };

    renderer.beginComponent(this.ir.component.name, this.ir.component.slots);
    this._walkNode(this.ir.component.tree, context, renderer);
    renderer.endComponent();
    return renderer.emit();
  }

  private _walkNode(node: MosaicNode, ctx: SlotContext, r: MosaicRenderer): void {
    const resolved = node.properties.map((p) => ({
      name: p.name,
      value: this._resolveValue(p.value, ctx),
    }));
    r.beginNode(node.tag, node.isPrimitive, resolved, ctx);
    for (const child of node.children) {
      this._walkChild(child, ctx, r);
    }
    r.endNode(node.tag);
  }

  private _walkChild(child: MosaicChild, ctx: SlotContext, r: MosaicRenderer): void {
    switch (child.kind) {
      case "node":
        this._walkNode(child.node, ctx, r);
        break;

      case "slot_ref":
        const slot = this._resolveSlot(child.slotName, ctx);
        r.renderSlotChild(child.slotName, slot.type, ctx);
        break;

      case "when":
        r.beginWhen(child.slotName, ctx);
        for (const c of child.children) this._walkChild(c, ctx, r);
        r.endWhen();
        break;

      case "each": {
        const listSlot = ctx.componentSlots.get(child.slotName)!;
        const elementType = (listSlot.type as { kind: "list"; elementType: MosaicType })
          .elementType;
        r.beginEach(child.slotName, child.itemName, elementType, ctx);
        const innerCtx: SlotContext = {
          componentSlots: ctx.componentSlots,
          loopScopes: [...ctx.loopScopes, { itemName: child.itemName, elementType }],
        };
        for (const c of child.children) this._walkChild(c, innerCtx, r);
        r.endEach();
        break;
      }
    }
  }

  private _resolveValue(v: MosaicValue, ctx: SlotContext): ResolvedValue {
    switch (v.kind) {
      case "string":    return { kind: "string", value: v.value };
      case "number":    return { kind: "number", value: v.value };
      case "bool":      return { kind: "bool", value: v.value };
      case "ident":     return { kind: "string", value: v.value };
      case "dimension": return this._parseDimension(v.value);
      case "color_hex": return this._parseColor(v.value);
      case "enum":      return { kind: "enum", namespace: v.namespace, member: v.member };
      case "slot_ref":  return this._resolveSlotRef(v.slotName, ctx);
    }
  }

  private _parseDimension(raw: string): ResolvedValue {
    // raw is e.g. "16dp", "1.5sp", "50%"
    const match = raw.match(/^(-?[0-9]*\.?[0-9]+)(dp|sp|%)$/);
    if (!match) throw new MosaicVMError(`Invalid dimension: ${raw}`);
    return { kind: "dimension", value: parseFloat(match[1]), unit: match[2] as "dp" | "sp" | "%" };
  }

  private _parseColor(hex: string): ResolvedValue {
    // hex is e.g. "#2563eb", "#fff", "#2563ebff"
    const h = hex.slice(1); // strip '#'
    let r: number, g: number, b: number, a = 255;
    if (h.length === 3) {
      r = parseInt(h[0] + h[0], 16);
      g = parseInt(h[1] + h[1], 16);
      b = parseInt(h[2] + h[2], 16);
    } else if (h.length === 6) {
      r = parseInt(h.slice(0, 2), 16);
      g = parseInt(h.slice(2, 4), 16);
      b = parseInt(h.slice(4, 6), 16);
    } else if (h.length === 8) {
      r = parseInt(h.slice(0, 2), 16);
      g = parseInt(h.slice(2, 4), 16);
      b = parseInt(h.slice(4, 6), 16);
      a = parseInt(h.slice(6, 8), 16);
    } else {
      throw new MosaicVMError(`Invalid color hex: ${hex}`);
    }
    return { kind: "color", r, g, b, a };
  }

  private _resolveSlotRef(slotName: string, ctx: SlotContext): ResolvedValue {
    // Check loop scopes innermost-first
    for (let i = ctx.loopScopes.length - 1; i >= 0; i--) {
      if (ctx.loopScopes[i].itemName === slotName) {
        return {
          kind: "slot_ref",
          slotName,
          slotType: ctx.loopScopes[i].elementType,
          isLoopVar: true,
        };
      }
    }
    // Fall back to component slots
    const slot = ctx.componentSlots.get(slotName);
    if (!slot) throw new MosaicVMError(`Unresolved slot reference: @${slotName}`);
    return { kind: "slot_ref", slotName, slotType: slot.type, isLoopVar: false };
  }

  private _resolveSlot(slotName: string, ctx: SlotContext): MosaicSlot {
    const slot = ctx.componentSlots.get(slotName);
    if (!slot) throw new MosaicVMError(`Unknown slot: @${slotName}`);
    return slot;
  }
}

export class MosaicVMError extends Error {}
```


### 1.6 — Top-Level Entry Point

```typescript
// mosaic-vm/src/index.ts — public API

export { MosaicVM, MosaicVMError } from "./vm.js";
export type {
  MosaicRenderer,
  MosaicEmitResult,
  ResolvedValue,
  ResolvedProperty,
  SlotContext,
} from "./types.js";
// Re-export MosaicIR types so backends only need to depend on mosaic-vm,
// not mosaic-analyzer directly.
export type {
  MosaicIR,
  MosaicComponent,
  MosaicSlot,
  MosaicType,
  MosaicNode,
  MosaicChild,
  MosaicProperty,
  MosaicValue,
  MosaicImport,
} from "./ir.js";
```

Backends depend on `mosaic-vm` only. They never import from `mosaic-analyzer` directly.


## Part 2: `mosaic-emit-react` — React Backend

**Package:** `code/packages/typescript/mosaic-emit-react`

**Input:** `MosaicIR` (via `MosaicVM.run(new MosaicReactRenderer())`)

**Output:** one `.tsx` file per component


### 2.1 — Internal Architecture

`MosaicReactRenderer` maintains a **string stack** — a stack of string buffers, one
per open node. When `beginNode` is called, a new buffer is pushed. When `endNode` is
called, the buffer is popped, its accumulated children content is inserted into the
closing tag, and the complete element string is appended to the parent buffer.

The stack enables arbitrarily deep nesting with correct close-before-pop ordering
without any lookahead.

```
Stack state after beginComponent + beginNode("Column") + beginNode("Text"):
  [0] component body buffer: ""
  [1] Column buffer: ""          <- open, will hold Column's complete JSX
  [2] Text buffer: ""            <- open, waiting for Text's children
```

When `endNode("Text")` is called:
```
  Pop Text buffer → "<span style={{...}}>{props.title}</span>"
  Append to Column buffer:
  [0] component body buffer: ""
  [1] Column buffer: "<span style={{...}}>{props.title}</span>"
```

When `endNode("Column")` is called:
```
  Pop Column buffer → "<div style={{...}}><span>...</span></div>"
  Append to component body buffer:
  [0] component body buffer: "<div style={{...}}>...</div>"
```

`endComponent()` wraps this in the function declaration.
`emit()` finalizes imports and returns the complete file.


### 2.2 — Slot Types → TypeScript Props Interface

| Mosaic slot type | TypeScript type in props |
|---|---|
| `text` | `string` |
| `number` | `number` |
| `bool` | `boolean` |
| `image` | `string` |
| `color` | `string` |
| `node` | `React.ReactNode` |
| `Button` (component) | `React.ReactElement<ButtonProps>` |
| `T` (any component) | `React.ReactElement<TProps>` |
| `list<text>` | `string[]` |
| `list<number>` | `number[]` |
| `list<bool>` | `boolean[]` |
| `list<node>` | `React.ReactNode[]` |
| `list<T>` (component) | `Array<React.ReactElement<TProps>>` |

Slots with a `defaultValue` → optional (`?`) with a default in the destructured params.
Slots without → required (no `?`).

**Example — given:**
```
slot title: text;
slot count: number = 0;
slot action: Button;
slot expanded: bool = false;
slot tags: list<text>;
```

**Emits:**
```typescript
import type { ButtonProps } from "./Button.js";

interface CardProps {
  title: string;
  count?: number;
  action: React.ReactElement<ButtonProps>;
  expanded?: boolean;
  tags: string[];
}

export function Card({
  title,
  count = 0,
  action,
  expanded = false,
  tags,
}: CardProps): JSX.Element {
```


### 2.3 — Primitive Nodes → JSX Elements

| Mosaic tag | JSX element | Required base style |
|---|---|---|
| `Box` | `<div>` | `position: 'relative'` |
| `Column` | `<div>` | `display: 'flex', flexDirection: 'column'` |
| `Row` | `<div>` | `display: 'flex', flexDirection: 'row'` |
| `Text` | `<span>` | _(none)_ |
| `Image` | `<img>` | _(none)_ |
| `Spacer` | `<div>` | `flex: 1` |
| `Scroll` | `<div>` | `overflow: 'auto'` |
| `Divider` | `<hr>` | `border: 'none', borderTop: '1px solid currentColor'` |

When `isPrimitive` is `false`, the tag is an imported component name. The renderer
emits `<ComponentName ...props />` and ensures the component's import appears at the
top of the file.


### 2.4 — Abstract Properties → Inline Style

The renderer accumulates inline style keys from each `ResolvedProperty`. All styles
are emitted as a single `style={{...}}` on the JSX element.

**Layout:**

| Mosaic | React inline style |
|---|---|
| `padding: {dim}` | `padding: '{dim}px'` _(dp → px)_ |
| `padding-left/right/top/bottom` | `paddingLeft/Right/Top/Bottom` |
| `gap: {dim}` | `gap: '{dim}px'` |
| `width: fill` | `width: '100%'` |
| `width: wrap` | `width: 'fit-content'` |
| `width: {dim}` | `width: '{dim}px'` |
| `height: fill` | `height: '100%'` |
| `height: wrap` | `height: 'fit-content'` |
| `height: {dim}` | `height: '{dim}px'` |
| `min-width/max-width/min-height/max-height: {dim}` | `minWidth/maxWidth/minHeight/maxHeight` |
| `overflow: visible` | `overflow: 'visible'` |
| `overflow: hidden` | `overflow: 'hidden'` |
| `overflow: scroll` | `overflow: 'auto'` |

`dp` and `sp` both map to `px` for the React/web backend.
`%` passes through unchanged.

**Alignment:**

The `align` property interacts with the node's `flexDirection`. The renderer looks
up the current node's tag to determine which flex axis is the main axis.

| `align` value | On `Column` | On `Row` |
|---|---|---|
| `start` | `alignItems: 'flex-start'` | `alignItems: 'flex-start'` |
| `center` | `alignItems: 'center'` | `alignItems:'center', justifyContent:'center'` |
| `end` | `alignItems: 'flex-end'` | `alignItems:'flex-end', justifyContent:'flex-end'` |
| `stretch` | `alignItems: 'stretch'` | `alignItems: 'stretch'` |
| `center-horizontal` | `alignItems: 'center'` | `justifyContent: 'center'` |
| `center-vertical` | `justifyContent: 'center'` | `alignItems: 'center'` |

On `Box`, `align` sets `alignItems` and additionally sets `display: 'flex'`.

**Visual:**

| Mosaic | React inline style |
|---|---|
| `background: {color}` | `backgroundColor: 'rgba(r,g,b,a/255)'` |
| `corner-radius: {dim}` | `borderRadius: '{dim}px'` |
| `border-width: {dim}` | `borderWidth: '{dim}px', borderStyle: 'solid'` |
| `border-color: {color}` | `borderColor: 'rgba(...)'` |
| `opacity: {number}` | `opacity: N` |
| `shadow: elevation.none` | `boxShadow: 'none'` |
| `shadow: elevation.low` | `boxShadow: '0 1px 3px rgba(0,0,0,0.12)'` |
| `shadow: elevation.medium` | `boxShadow: '0 4px 12px rgba(0,0,0,0.15)'` |
| `shadow: elevation.high` | `boxShadow: '0 8px 24px rgba(0,0,0,0.20)'` |
| `visible: false` | `display: 'none'` |
| `visible: @slot` | conditional — see §2.6 |

Color output format: `rgba(37, 99, 235, 1)` when `a === 255` → simplifies to
`rgba(37, 99, 235, 1)`. Always use `rgba()` (never hex strings) in generated code so
the output is consistent regardless of how the source was written.

**Text (`Text` primitive only):**

| Mosaic | React |
|---|---|
| `content: "literal"` | children: `"literal"` |
| `content: @slot` | children: `{props.slotName}` (or `{item}` for loop var) |
| `color: {color}` | `color: 'rgba(...)'` in style |
| `text-align: start/center/end` | `textAlign: 'left'/'center'/'right'` |
| `font-weight: normal/bold` | `fontWeight: 'normal'/'bold'` |
| `max-lines: N` | `WebkitLineClamp:N, overflow:'hidden', display:'-webkit-box', WebkitBoxOrient:'vertical'` |
| `style: heading.large` | `className="mosaic-heading-large"` |
| `style: heading.medium` | `className="mosaic-heading-medium"` |
| `style: heading.small` | `className="mosaic-heading-small"` |
| `style: body.large/medium/small` | `className="mosaic-body-large/medium/small"` |
| `style: label` | `className="mosaic-label"` |
| `style: caption` | `className="mosaic-caption"` |

Typography `style:` is the only property that emits `className` (not inline style).
A single `mosaic-type-scale.css` companion file is emitted once per project and
imported in any component that uses `style:`. This avoids hardcoding font metrics
in every generated component.

**Image (`Image` primitive only):**

| Mosaic | React |
|---|---|
| `source: @slot` | `src={props.slotName}` |
| `source: "literal"` | `src="literal"` |
| `size: {dim}` | `width: '{dim}px', height: '{dim}px'` |
| `shape: circle` | `borderRadius: '50%'` |
| `shape: rounded` | `borderRadius: '8px'` |
| `fit: cover/contain/fill/none` | `objectFit: 'cover'/'contain'/'fill'/'none'` |

**Accessibility:**

| Mosaic | React JSX attribute |
|---|---|
| `a11y-label: "literal"` | `aria-label="literal"` |
| `a11y-label: @slot` | `aria-label={props.slotName}` |
| `a11y-role: button` | `role="button"` |
| `a11y-role: heading` | element becomes `<h2>` instead of `<span>` for `Text` |
| `a11y-role: image` | `role="img"` |
| `a11y-role: list` | `role="list"` |
| `a11y-role: listitem` | `role="listitem"` |
| `a11y-role: none` | `aria-hidden="true"` |
| `a11y-hidden: true` | `aria-hidden="true"` |


### 2.5 — Renderer Calls → JSX: Worked Traces

**Slot child (`renderSlotChild`):**

When the VM calls `renderSlotChild("action", {kind:"component", name:"Button"}, ctx)`:
- Append `{props.action}` to the current buffer (no wrapping element needed)
- For `node` type: also append `{props.action}` — `React.ReactNode` renders directly

**When block (`beginWhen` / `endWhen`):**

When the VM calls `beginWhen("visible", ctx)`:
- Push a new "when buffer" onto the stack

When `endWhen()` is called:
- Pop the when buffer, wrap its content:
  - Single child: `{props.visible && (<child />)}`
  - Multiple children: `{props.visible && (<>{...children}</>)}`
- Append to parent buffer

**Each block (`beginEach` / `endEach`):**

When the VM calls `beginEach("items", "item", {kind:"text"}, ctx)`:
- Push a new "each buffer" and record `itemName = "item"`

When `endEach()` is called:
- Pop the each buffer, wrap its content:
  ```tsx
  {props.items.map((item, _index) => (
    <React.Fragment key={_index}>
      {/* ...body... */}
    </React.Fragment>
  ))}
  ```
- For `list<node>` / `list<ComponentType>`, the items are already React elements:
  ```tsx
  {props.items.map((item, _index) => (
    <React.Fragment key={_index}>{item}</React.Fragment>
  ))}
  ```

Inside the each body, references to `@item` (the loop variable, `isLoopVar: true`)
emit `item` (the local variable name) instead of `props.item`.


### 2.6 — Full Generated File Structure

```typescript
// AUTO-GENERATED from ComponentName.mosaic — do not edit
// Generated by mosaic-emit-react v1.0
// Source: ComponentName.mosaic
//
// To modify this component, edit ComponentName.mosaic and re-run the compiler.

import React from "react";
import "./mosaic-type-scale.css";      // present only if any Text uses style:

import type { ButtonProps } from "./Button.js";   // one line per component-type slot

interface ComponentNameProps {
  // required slots first, then optional (with default values noted)
  title: string;
  count?: number;           // default: 0
  expanded?: boolean;       // default: false
  action: React.ReactElement<ButtonProps>;
  tags: string[];
}

export function ComponentName({
  title,
  count = 0,
  expanded = false,
  action,
  tags,
}: ComponentNameProps): JSX.Element {
  return (
    <div style={{ display: "flex", flexDirection: "column", padding: "16px", gap: "12px" }}>
      <span>{title}</span>
      {expanded && (
        <span style={{ fontWeight: "bold" }}>{count}</span>
      )}
      {tags.map((item, _index) => (
        <React.Fragment key={_index}>
          <span>{item}</span>
        </React.Fragment>
      ))}
      {action}
    </div>
  );
}
```


## Part 3: `mosaic-emit-webcomponent` — Web Components Backend

**Package:** `code/packages/typescript/mosaic-emit-webcomponent`

**Input:** `MosaicIR` (via `MosaicVM.run(new MosaicWebComponentRenderer())`)

**Output:** one `.ts` file (the Custom Element class)


### 3.1 — Custom Element Design

Each Mosaic component compiles to a Custom Element:
- Class extends `HTMLElement`
- Uses Shadow DOM (`attachShadow({ mode: "open" })`) for style encapsulation
- Private fields for each slot value (prefixed `_`)
- A single `_render()` method that rebuilds the shadow DOM from current slot values
- `connectedCallback()` calls `_render()` on first attachment
- `disconnectedCallback()` cleans up any `node` slots projected into Light DOM (see §3.4)

**Tag name convention:** `mosaic-{kebab-case-component-name}`
- `ProfileCard` → `<mosaic-profile-card>`
- `Button` → `<mosaic-button>`
- `HowItWorks` → `<mosaic-how-it-works>`


### 3.2 — Slot Types → Class Fields and Setters

Each slot becomes a private backing field and a public setter/getter pair.

| Mosaic type | TypeScript field type | Setter accepts | Attribute? |
|---|---|---|---|
| `text` | `string` | `string` | Yes (kebab-case name) |
| `number` | `number` | `number` | Yes (string → parseFloat in getter) |
| `bool` | `boolean` | `boolean` | Yes (presence = true, absence = false) |
| `image` | `string` | `string` | Yes |
| `color` | `string` | `string` | Yes |
| `node` | `Node \| null` | `Element` | No |
| `T` (component) | `HTMLElement \| null` | `HTMLElement` | No |
| `list<text>` | `string[]` | `string[]` | No (arrays cannot be attributes) |
| `list<number>` | `number[]` | `number[]` | No |
| `list<node>` | `Element[]` | `Element[]` | No |
| `list<T>` | `HTMLElement[]` | `HTMLElement[]` | No |

**`observedAttributes`** includes only the primitive scalar slot names (those where
"Attribute?" is Yes above). When an attribute changes, `attributeChangedCallback`
updates the backing field and calls `_render()`.

**Default values:** Primitive slots with defaults initialize the backing field at
declaration: `private _count: number = 0;`

**Example for:**
```
slot title: text;
slot count: number = 0;
slot visible: bool = false;
slot action: Button;
slot items: list<text>;
```

```typescript
private _title: string = '';
private _count: number = 0;
private _visible: boolean = false;
private _action: HTMLElement | null = null;
private _items: string[] = [];

static get observedAttributes(): string[] {
  return ['title', 'count', 'visible'];
}

attributeChangedCallback(name: string, _old: string | null, value: string | null): void {
  switch (name) {
    case 'title':   this._title = value ?? '';          break;
    case 'count':   this._count = parseFloat(value ?? '0'); break;
    case 'visible': this._visible = value !== null;     break;
  }
  this._render();
}

set title(v: string)   { this._title = v;  this._render(); }
get title(): string    { return this._title; }

set count(v: number)   { this._count = v;  this._render(); }
get count(): number    { return this._count; }

set visible(v: boolean){ this._visible = v; this._render(); }
get visible(): boolean { return this._visible; }

set action(v: HTMLElement) { this._projectSlot('action', v); }
set items(v: string[])     { this._items = v; this._render(); }
```


### 3.3 — Node Slots: Light DOM Projection

`node`-typed slots and component-typed slots cannot be expressed as attributes —
they are full DOM subtrees. The Web Components backend uses **Light DOM slotting**:
the Shadow DOM contains a named `<slot>` element, and the setter inserts the provided
node into Light DOM with the matching `slot` attribute.

```typescript
private _projectSlot(name: string, node: Element): void {
  // Remove any previously projected element for this slot.
  const prev = this.querySelector(`[data-mosaic-slot="${name}"]`);
  if (prev) prev.remove();

  // Stamp the slot attribute and a tracking attribute onto the node.
  node.setAttribute('slot', name);
  node.setAttribute('data-mosaic-slot', name);
  this.appendChild(node);
  // No _render() needed — Shadow DOM slot projection updates automatically.
}
```

The Shadow DOM template for a `node` slot `@action` contains:
```html
<slot name="action"></slot>
```

`disconnectedCallback` removes all `data-mosaic-slot` children to prevent leaks:
```typescript
disconnectedCallback(): void {
  [...this.querySelectorAll('[data-mosaic-slot]')].forEach((el) => el.remove());
}
```


### 3.4 — List Slots: `list<text>` vs `list<node>`

**`list<text>` (and other primitive lists):**
The `_render()` method iterates `this._items` to build the innerHTML fragment for
the `each` block inline. No Light DOM projection is used.

```javascript
// Inside _render() for each @items as item { <span>{@item}</span> }
${this._items.map(item =>
  `<span>${this._escapeHtml(item)}</span>`
).join('')}
```

**`list<node>` (and component lists):**
The setter removes previous projected children and projects the new array:

```typescript
set items(nodes: Element[]): void {
  [...this.querySelectorAll('[data-mosaic-slot="items"]')].forEach(el => el.remove());
  nodes.forEach((node, i) => {
    node.setAttribute('slot', `items-${i}`);
    node.setAttribute('data-mosaic-slot', 'items');
    this.appendChild(node);
  });
  this._render();
}
```

The Shadow DOM template generates a `<slot name="items-N">` for each projected
child during `_render()`. Because the count of nodes is known only at setter time,
`_render()` emits as many `<slot>` elements as there are items:

```typescript
// Inside _render()
${this._itemNodes.map((_, i) =>
  `<slot name="items-${i}"></slot>`
).join('')}
```

This means `_render()` and the setter are coupled for `list<node>` slots: the setter
stores the count (`this._itemNodeCount`) so `_render()` can emit the right number of
`<slot>` elements.


### 3.5 — The `_render()` Method

`_render()` rebuilds `this._shadow.innerHTML` entirely on each call. This is fast for
small components (< ~50 nodes) and keeps the implementation simple. Performance
optimization (diffing, keyed updates) is intentionally out of scope for v1.

**Security:** every string slot value passed to `innerHTML` must go through
`_escapeHtml()`. Color and image slots are sanitized separately (see §3.6).

```typescript
private _escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
```

**Color values** from color slots are validated before insertion into inline styles.
The VM already provides colors as `ResolvedValue { kind:"color"; r; g; b; a }`, so
the renderer emits them as `rgba(r, g, b, a/255)` — never from raw slot strings.
When a color slot value comes in via the setter (as a `string` CSS color), it is
sanitized by rejecting any value that does not match
`/^(#[0-9a-fA-F]{3,8}|rgba?\([^)]+\)|[a-zA-Z]+)$/` before storing.

**Image source values** from image slots are used in `src` attributes (not innerHTML
style). The renderer emits `<img src="${this._escapeHtml(this._avatarUrl)}" ...>`.
URL validation (no `javascript:` scheme) is enforced in the setter:
```typescript
set avatarUrl(v: string) {
  if (/^javascript:/i.test(v.trim())) return; // silently reject
  this._avatarUrl = v;
  this._render();
}
```


### 3.6 — Abstract Properties → Inline Style Strings

The Web Components backend emits properties as HTML inline style attribute strings
(e.g., `style="display:flex;flex-direction:column;padding:16px"`) rather than JS style
objects. The mapping rules are the same dimension conversions as the React backend
(dp → px, sp → px, % → %).

The renderer accumulates style string fragments for each `ResolvedProperty` and joins
them with `;`:

```typescript
function resolvedToStyleString(props: ResolvedProperty[]): string {
  return props.flatMap(p => propertyToCSS(p)).join(';');
}
```

**Layout properties → CSS:**

| Mosaic | CSS |
|---|---|
| `padding: 16dp` | `padding:16px` |
| `gap: 8dp` | `gap:8px` |
| `width: fill` | `width:100%` |
| `width: wrap` | `width:fit-content` |
| `corner-radius: 8dp` | `border-radius:8px` |
| `background: {color}` | `background-color:rgba(...)` |
| `shadow: elevation.medium` | `box-shadow:0 4px 12px rgba(0,0,0,0.15)` |
| `opacity: 0.5` | `opacity:0.5` |

The full table mirrors §2.4 with CSS property names (kebab-case) instead of React
style keys (camelCase). `rgba()` format is the same.

**Primitive node base styles** (emitted as the starting style for each tag):

| Mosaic tag | Base CSS |
|---|---|
| `Box` | `position:relative` |
| `Column` | `display:flex;flex-direction:column` |
| `Row` | `display:flex;flex-direction:row` |
| `Text` | _(none)_ |
| `Image` | _(none)_ |
| `Spacer` | `flex:1` |
| `Scroll` | `overflow:auto` |
| `Divider` | `border:none;border-top:1px solid currentColor` |

Typography `style:` emits a class name here too, same as the React backend. A
companion `mosaic-type-scale.css` is loaded via a `<link>` in the Shadow DOM template
(not an external import — injected as a `<style>` tag from the emitted `.ts` file as
a constant string).


### 3.7 — Conditional Rendering in `_render()`

`when @visible { ... }` becomes a ternary in the template string:

```typescript
beginWhen("visible", ctx)   // records: "if this._visible"
endWhen()
```

Emits:
```javascript
${this._visible ? `<...children...>` : ''}
```

For `bool` slots with `isLoopVar: false`, the backing field is `this._slotName`.


### 3.8 — Iteration in `_render()`

`each @items as item { ... }` emits a `.map()` call in the template string.

For `list<text>`:
```javascript
${this._items.map(item => `
  <div style="...">
    <span>${this._escapeHtml(item)}</span>
  </div>
`).join('')}
```

For `list<node>` — the each block body is expressed as a counter-indexed `<slot>`:
```javascript
${this._items.map((_, i) => `<slot name="items-${i}"></slot>`).join('')}
```

The `list<node>` setter (§3.4) keeps `this._items` as the `Element[]` and
`this._items.length` provides the count for slot generation.


### 3.9 — Full Generated File Structure

```typescript
// AUTO-GENERATED from ComponentName.mosaic — do not edit
// Generated by mosaic-emit-webcomponent v1.0
// Source: ComponentName.mosaic
//
// To modify this component, edit ComponentName.mosaic and re-run the compiler.

const MOSAIC_TYPE_SCALE_CSS = `
  .mosaic-heading-large { font-size: 2rem; font-weight: 700; line-height: 1.2; }
  /* ... full scale ... */
`;

export class MosaicComponentNameElement extends HTMLElement {
  private _shadow: ShadowRoot;

  // --- Backing fields ---
  private _title: string = '';
  private _count: number = 0;
  // ...

  constructor() {
    super();
    this._shadow = this.attachShadow({ mode: 'open' });
  }

  // --- Attribute observation (primitive slots only) ---
  static get observedAttributes(): string[] {
    return ['title', 'count'];
  }

  attributeChangedCallback(name: string, _old: string | null, value: string | null): void {
    // ...
    this._render();
  }

  // --- Property setters/getters ---
  set title(v: string) { this._title = v; this._render(); }
  get title(): string  { return this._title; }
  // ...

  // --- Node slot setters ---
  set action(v: HTMLElement) { this._projectSlot('action', v); }
  // ...

  // --- DOM helpers ---
  private _projectSlot(name: string, node: Element): void { /* ... */ }
  private _escapeHtml(s: string): string { /* ... */ }

  // --- Lifecycle ---
  connectedCallback(): void { this._render(); }
  disconnectedCallback(): void {
    [...this.querySelectorAll('[data-mosaic-slot]')].forEach((el) => el.remove());
  }

  // --- Render ---
  private _render(): void {
    this._shadow.innerHTML = `
      <style>${MOSAIC_TYPE_SCALE_CSS}</style>
      <div style="display:flex;flex-direction:column;padding:16px;gap:12px">
        <span>${this._escapeHtml(this._title)}</span>
        ${this._visible ? `<span>${this._escapeHtml(this._title)}</span>` : ''}
        ${this._items.map(item => `<span>${this._escapeHtml(item)}</span>`).join('')}
        <slot name="action"></slot>
      </div>
    `;
  }
}

customElements.define('mosaic-component-name', MosaicComponentNameElement);
```

The type scale CSS is embedded as a template literal constant inside the generated `.ts`
file. This keeps the output to a single file (no external `.css` dependency) while
still using named style classes inside Shadow DOM (where external CSS does not penetrate
anyway).


### 3.10 — Renderer Stack vs React Backend

The Web Components renderer does **not** use the same JSX-building string stack as the
React backend. Instead it builds **nested template literal fragments** — strings that
contain literal backtick template expressions. The nesting is achieved by a recursive
helper that the renderer builds as it receives VM events:

```typescript
// Pseudocode of the internal accumulator structure
type TemplateFragment =
  | { kind: "literal"; text: string }
  | { kind: "escape"; field: string }              // ${this._escapeHtml(this._foo)}
  | { kind: "conditional"; field: string; body: TemplateFragment[] }
  | { kind: "map_list"; field: string; body: TemplateFragment[] }
  | { kind: "slot_projection"; slotName: string }
  | { kind: "container"; tag: string; style: string; children: TemplateFragment[] };
```

The renderer walks this fragment tree to produce the final `_render()` body string.
This two-pass approach (accumulate fragments, then serialize) is cleaner than
building the nested template string inline during the VM walk, since template strings
require the `${...}` delimiters to be positioned correctly relative to nesting depth.


## Part 4: Packages Required

| Package | Depends on | Role |
|---|---|---|
| `mosaic-lexer` | `@coding-adventures/lexer` | Tokenize `.mosaic` source |
| `mosaic-parser` | `@coding-adventures/parser`, `mosaic-lexer` | Parse tokens → AST |
| `mosaic-analyzer` | `mosaic-parser` | Validate AST → MosaicIR |
| **`mosaic-vm`** | `mosaic-analyzer` | Drive tree walk, normalize values |
| **`mosaic-emit-react`** | `mosaic-vm` | React backend |
| **`mosaic-emit-webcomponent`** | `mosaic-vm` | Web Components backend |
| `mosaic-compiler` | `mosaic-vm`, all emitters | CLI + file I/O |

The three bold packages are new in this spec. `mosaic-compiler` is the CLI tool that
connects them: it reads a `.mosaic` file, runs the full pipeline, and writes output
files to disk.

```
mosaic-compiler compile --backend react --output-dir src/ src/Footer.mosaic
mosaic-compiler compile --backend webcomponent --output-dir dist/ src/Footer.mosaic
mosaic-compiler compile --backend react --backend webcomponent src/Footer.mosaic
```


## Part 5: Build File Notes

Following the `lessons.md` BUILD patterns:

- `mosaic-vm`: depends on `mosaic-analyzer` → `mosaic-parser` → `mosaic-lexer` →
  `@coding-adventures/parser` → `@coding-adventures/lexer`. List in leaf-to-root order
  in the BUILD file's deps array.

- `mosaic-emit-react`: depends only on `mosaic-vm` (re-exports all IR types). No direct
  dep on `mosaic-analyzer`.

- `mosaic-emit-webcomponent`: same as `mosaic-emit-react` — depends only on `mosaic-vm`.

- Both emit packages are pure TypeScript, no runtime deps beyond what's already in the
  generated output consumers. The `typescript` package is a dev-only dep (for type
  checking during build), not a runtime dep.

- Each package needs `BUILD`, `BUILD_windows`, `README.md`, `CHANGELOG.md`,
  `required_capabilities.json`.


## Deliberate Omissions

- **Ingestion (React → Mosaic):** out of scope for this spec and this compiler version.
- **SwiftUI / Compose / paint-vm backends:** same `MosaicRenderer` contract applies;
  specified in future documents.
- **Incremental compilation / caching:** not in v1. The VM re-runs from IR on every
  compile invocation.
- **Source maps:** not in v1. The `// AUTO-GENERATED from X.mosaic` header is the only
  provenance information.
- **Hot reload / watch mode:** a future `mosaic-compiler --watch` flag will use
  filesystem watchers to re-invoke the pipeline on `.mosaic` changes. Not specified here.
