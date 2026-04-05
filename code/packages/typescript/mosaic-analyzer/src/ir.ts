/**
 * MosaicIR — Typed Intermediate Representation for Mosaic Components.
 *
 * The MosaicIR is the output of the **analyzer stage** in the Mosaic compiler
 * pipeline. It is a validated, platform-neutral data structure that the VM
 * and all backends consume. Once produced, no further parsing or type-checking
 * is needed — all slot types are resolved, all defaults are normalized, and the
 * node tree is fully structured.
 *
 * What "Intermediate Representation" Means Here
 * ----------------------------------------------
 *
 * The term IR (Intermediate Representation) comes from compiler design. A
 * compiler typically works in stages:
 *
 *   Source code  →  AST  →  IR  →  Target code
 *
 * The AST is a direct reflection of the source syntax — every token and
 * grammar rule has a corresponding tree node. The IR is a cleaned-up, typed
 * version of the same information where:
 *
 *   - Syntax noise (keywords, semicolons, braces) is stripped away
 *   - Every name is resolved — no bare strings, just typed values
 *   - Defaults are normalized — "number = 0" becomes `{ kind: "number", value: 0 }`
 *   - Errors are caught — undefined slots, invalid types, unknown properties
 *
 * This split is why compilers are robust: the analyzer is the single place
 * where all semantic checks live. The backends receive an IR that is already
 * known-good and can focus entirely on code generation.
 *
 * MosaicIR Type Hierarchy
 * -----------------------
 *
 *   MosaicIR
 *   ├── component: MosaicComponent
 *   │   ├── name: string
 *   │   ├── slots: MosaicSlot[]
 *   │   │   ├── name: string
 *   │   │   ├── type: MosaicType
 *   │   │   ├── defaultValue?: MosaicValue
 *   │   │   └── required: boolean
 *   │   └── tree: MosaicNode
 *   │       ├── tag: string
 *   │       ├── isPrimitive: boolean
 *   │       ├── properties: MosaicProperty[]
 *   │       │   ├── name: string
 *   │       │   └── value: MosaicValue
 *   │       └── children: MosaicChild[]
 *   │           ├── { kind: "node", node: MosaicNode }
 *   │           ├── { kind: "slot_ref", slotName: string }
 *   │           ├── { kind: "when", slotName: string, children: MosaicChild[] }
 *   │           └── { kind: "each", slotName, itemName, children: MosaicChild[] }
 *   └── imports: MosaicImport[]
 *       ├── componentName: string
 *       ├── alias?: string
 *       └── path: string
 */

// ============================================================================
// Top-Level Structure
// ============================================================================

/**
 * The root of the intermediate representation.
 *
 * A `MosaicIR` contains one component and its import declarations.
 * (A `.mosaic` file always declares exactly one component.)
 */
export interface MosaicIR {
  /** The single component declared in this `.mosaic` file. */
  component: MosaicComponent;

  /** All `import X from "..."` declarations at the top of the file. */
  imports: MosaicImport[];
}

// ============================================================================
// Component
// ============================================================================

/**
 * A Mosaic component — the unit of UI composition.
 *
 * A component has:
 *   - A **name** (PascalCase by convention, e.g., `ProfileCard`)
 *   - **Slots** — typed data inputs (like props in React, attributes in HTML)
 *   - A **tree** — the root node of the visual hierarchy
 */
export interface MosaicComponent {
  /** PascalCase component name, e.g., `ProfileCard`. */
  name: string;

  /** Ordered list of slot declarations. */
  slots: MosaicSlot[];

  /** Root node of the visual tree. */
  tree: MosaicNode;
}

// ============================================================================
// Imports
// ============================================================================

/**
 * An `import X from "..."` declaration.
 *
 * Imports bring other `.mosaic` components into scope so they can be used as
 * slot types or as composite nodes.
 *
 * Examples:
 *   - `import Button from "./button.mosaic"` → `{ componentName: "Button", path: "./button.mosaic" }`
 *   - `import Card as InfoCard from "./card.mosaic"` → `{ componentName: "Card", alias: "InfoCard", path: "./card.mosaic" }`
 */
export interface MosaicImport {
  /** The exported name from the source file (the `X` in `import X from …`). */
  componentName: string;

  /** Optional local alias (`Y` in `import X as Y from …`). */
  alias?: string;

  /** Relative or absolute path to the `.mosaic` source file. */
  path: string;
}

// ============================================================================
// Slots
// ============================================================================

/**
 * A typed data slot — the "props API" of a Mosaic component.
 *
 * Slots are the only way data enters a Mosaic component. There are no global
 * variables, no context, no implicit state. The host language fills slots via
 * generated typed setters before the component renders.
 *
 * Examples:
 *   - `slot title: text;`   → `{ name: "title", type: { kind: "text" }, required: true }`
 *   - `slot count: number = 0;` → `{ name: "count", type: { kind: "number" }, defaultValue: { kind: "number", value: 0 }, required: false }`
 */
export interface MosaicSlot {
  /** Slot name, kebab-case by convention (e.g., `avatar-url`, `display-name`). */
  name: string;

  /** The type of data this slot accepts. */
  type: MosaicType;

  /**
   * Default value when no data is provided by the host.
   * Present only when the slot declaration includes `= value`.
   */
  defaultValue?: MosaicValue;

  /**
   * Whether this slot must be set by the host.
   * A slot is required if and only if it has no `defaultValue`.
   */
  required: boolean;
}

// ============================================================================
// Types
// ============================================================================

/**
 * The type system for Mosaic slots.
 *
 * Mosaic has six primitive types (text, number, bool, image, color, node),
 * one flexible type (node), component types (named component references),
 * and one parameterized type (list).
 *
 * Type evolution:
 *   - During prototyping, use `node` — accepts any component.
 *   - As design stabilizes, tighten to `Button`, `Badge`, etc. for type safety.
 *
 * Discriminated union: every variant has a `kind` field for narrowing.
 *
 *   ```typescript
 *   function describeType(t: MosaicType): string {
 *     switch (t.kind) {
 *       case "text":      return "plain text";
 *       case "number":    return "numeric value";
 *       case "bool":      return "true/false flag";
 *       case "image":     return "image source";
 *       case "color":     return "color value";
 *       case "node":      return "any component (flexible)";
 *       case "component": return `${t.name} component`;
 *       case "list":      return `list of ${describeType(t.elementType)}`;
 *     }
 *   }
 *   ```
 */
export type MosaicType =
  | { kind: "text" }
  | { kind: "number" }
  | { kind: "bool" }
  | { kind: "image" }
  | { kind: "color" }
  /** Flexible type — accepts any component. Use during prototyping. */
  | { kind: "node" }
  /** Named component type from an import or self-reference. */
  | { kind: "component"; name: string }
  /** Parameterized list type. `list<text>` → `{ kind: "list", elementType: { kind: "text" } }` */
  | { kind: "list"; elementType: MosaicType };

// ============================================================================
// Node Tree
// ============================================================================

/**
 * A visual node in the component tree.
 *
 * Nodes correspond to platform-native elements. "Primitive" nodes are layout
 * containers and display elements defined in the Mosaic standard library
 * (Row, Column, Text, Image, Box, Stack). Non-primitive nodes are imported
 * component types.
 *
 * Example: `Text { content: @title; font-size: 14sp; }`
 *
 *   ```typescript
 *   {
 *     tag: "Text",
 *     isPrimitive: true,
 *     properties: [{ name: "content", value: { kind: "slot_ref", slotName: "title" } },
 *                  { name: "font-size", value: { kind: "dimension", value: 14, unit: "sp" } }],
 *     children: []
 *   }
 *   ```
 */
export interface MosaicNode {
  /** Element type name, e.g., `Row`, `Column`, `Text`, `Button`. */
  tag: string;

  /**
   * Whether this is a Mosaic primitive node.
   *
   * Primitives: Row, Column, Box, Stack, Text, Image, Icon, Spacer, Divider, Scroll.
   * Imported components have `isPrimitive: false`.
   */
  isPrimitive: boolean;

  /** Property assignments on this node (`name: value` pairs). */
  properties: MosaicProperty[];

  /** Direct children of this node: child nodes, slot refs, when/each blocks. */
  children: MosaicChild[];
}

// ============================================================================
// Children
// ============================================================================

/**
 * A child of a node — one of four forms.
 *
 * Discriminated union; switch on `kind`:
 *
 *   ```
 *   switch (child.kind) {
 *     case "node":     // render child.node recursively
 *     case "slot_ref": // render the slot named child.slotName
 *     case "when":     // render child.children only when bool slot is true
 *     case "each":     // render child.children for each item in list slot
 *   }
 *   ```
 */
export type MosaicChild =
  /** A nested node element (child_node in the grammar). */
  | { kind: "node"; node: MosaicNode }
  /** A slot reference used as a child: `@header;` */
  | { kind: "slot_ref"; slotName: string }
  /** Conditional subtree: `when @show { ... }` */
  | { kind: "when"; slotName: string; children: MosaicChild[] }
  /** Iterating subtree: `each @items as item { ... }` */
  | { kind: "each"; slotName: string; itemName: string; children: MosaicChild[] };

// ============================================================================
// Properties
// ============================================================================

/**
 * A single property assignment on a node.
 *
 * Properties set abstract layout/visual traits. Backends map them to
 * platform-native equivalents (CSS properties, SwiftUI modifiers, etc.).
 *
 * Examples:
 *   - `padding: 16dp;` → `{ name: "padding", value: { kind: "dimension", value: 16, unit: "dp" } }`
 *   - `background: #2563eb;` → `{ name: "background", value: { kind: "color_hex", value: "#2563eb" } }`
 *   - `align: center;` → `{ name: "align", value: { kind: "ident", value: "center" } }`
 */
export interface MosaicProperty {
  /** Property name, kebab-case (e.g., `corner-radius`, `font-size`). */
  name: string;

  /** The property's value. */
  value: MosaicValue;
}

// ============================================================================
// Values
// ============================================================================

/**
 * A property value or slot default value.
 *
 * Values come from the source in multiple forms. The analyzer normalizes them
 * all into typed discriminated union members so that backends never need to
 * parse strings.
 *
 * | Source text    | MosaicValue                                       |
 * |----------------|---------------------------------------------------|
 * | `@title`       | `{ kind: "slot_ref", slotName: "title" }`         |
 * | `"hello"`      | `{ kind: "string", value: "hello" }`              |
 * | `42`, `-3.14`  | `{ kind: "number", value: 42 }`                   |
 * | `16dp`         | `{ kind: "dimension", value: 16, unit: "dp" }`    |
 * | `#2563eb`      | `{ kind: "color_hex", value: "#2563eb" }`         |
 * | `true`/`false` | `{ kind: "bool", value: true }`                   |
 * | `center`       | `{ kind: "ident", value: "center" }`              |
 * | `align.center` | `{ kind: "enum", namespace: "align", member: "center" }` |
 */
export type MosaicValue =
  | { kind: "slot_ref"; slotName: string }
  | { kind: "string"; value: string }
  | { kind: "number"; value: number }
  | { kind: "dimension"; value: number; unit: string }
  | { kind: "color_hex"; value: string }
  | { kind: "bool"; value: boolean }
  /** Bare identifier used as a property value (e.g., `align: center;`). */
  | { kind: "ident"; value: string }
  /** Dotted namespace reference (e.g., `style: heading.small;`). */
  | { kind: "enum"; namespace: string; member: string };
