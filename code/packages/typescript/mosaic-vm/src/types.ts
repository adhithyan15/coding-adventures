/**
 * MosaicVM Types — ResolvedValue, ResolvedProperty, SlotContext, MosaicRenderer.
 *
 * These types define the **protocol** between the MosaicVM driver and the
 * backend renderers that generate platform-specific code. Backends implement
 * `MosaicRenderer`; the VM calls its methods in depth-first traversal order.
 *
 * Value Normalization: MosaicValue → ResolvedValue
 * -------------------------------------------------
 *
 * `MosaicValue` (from the IR) is what the Mosaic source text directly encodes —
 * a hex string like `"#2563eb"`, a dimension string like `"16dp"`, etc. This is
 * convenient for the analyzer but burdensome for backends. Every backend would
 * need to re-implement hex parsing and unit extraction.
 *
 * `ResolvedValue` is the VM's normalized form:
 *   - Hex colors are parsed into RGBA integers (0–255 each).
 *   - Dimensions are split into a numeric `value` and a `unit` string.
 *   - Bare identifiers (`{ kind: "ident" }`) are folded into `{ kind: "string" }`.
 *   - Slot refs gain the full `slotType` and an `isLoopVar` flag.
 *
 * Backends never receive raw `MosaicValue`. They always receive `ResolvedValue`.
 *
 * Slot Context
 * ------------
 *
 * `SlotContext` tracks which slots are in scope at any point in the tree walk:
 *
 *   - `componentSlots` — all slots declared on the component.
 *   - `loopScopes` — the stack of active `each` loop variables. When the VM
 *     enters `each @items as item { ... }`, it pushes `{ itemName: "item",
 *     elementType: T }` onto this stack. References to `@item` inside the
 *     block resolve as loop variables with `isLoopVar: true`.
 *
 * Renderer Interface
 * ------------------
 *
 * The `MosaicRenderer` interface has 11 methods, called in strict depth-first,
 * open-before-close order:
 *
 *   1. `beginComponent(name, slots)`         — once, before tree traversal
 *   2. `beginNode(tag, isPrimitive, props, ctx)` — on entering each node
 *   3. `renderSlotChild(slotName, …)`        — for `@slot;` children
 *   4. `beginWhen(slotName, ctx)`            — on entering `when @flag { ... }`
 *   5.   [children of when block]
 *   6. `endWhen()`                           — on leaving `when` block
 *   7. `beginEach(slotName, itemName, …)`    — on entering `each @list as item { ... }`
 *   8.   [children of each block]
 *   9. `endEach()`                           — on leaving `each` block
 *  10. `endNode(tag)`                        — on leaving each node
 *  11. `endComponent()`                      — once, after tree traversal
 *  12. `emit()`                              — called by `MosaicVM.run()` at the end
 */

import type { MosaicSlot, MosaicType } from "@coding-adventures/mosaic-analyzer";

// ============================================================================
// ResolvedValue
// ============================================================================

/**
 * A normalized, fully-typed property value.
 *
 * The VM normalizes `MosaicValue` into `ResolvedValue` before passing values
 * to the renderer. Backends always receive `ResolvedValue`, never raw `MosaicValue`.
 *
 * Normalization summary:
 *
 *   | MosaicValue kind | → ResolvedValue kind  | What changes                         |
 *   |------------------|-----------------------|--------------------------------------|
 *   | color_hex        | color                 | Parsed into r,g,b,a integers         |
 *   | dimension        | dimension             | Split into numeric value + unit       |
 *   | ident            | string                | Folded — no semantic change           |
 *   | slot_ref         | slot_ref              | Gains slotType and isLoopVar          |
 *   | string/number/bool/enum | unchanged       | Passed through as-is                |
 */
export type ResolvedValue =
  | { kind: "string";    value: string }
  | { kind: "number";    value: number }
  | { kind: "bool";      value: boolean }
  | { kind: "dimension"; value: number; unit: "dp" | "sp" | "%" }
  | { kind: "color";     r: number; g: number; b: number; a: number }
  | { kind: "enum";      namespace: string; member: string }
  | {
      kind: "slot_ref";
      slotName: string;
      /** The declared type of the referenced slot. */
      slotType: MosaicType;
      /**
       * `true` when this slot ref refers to a loop variable from an enclosing
       * `each` block, not a component slot.
       */
      isLoopVar: boolean;
    };

// ============================================================================
// ResolvedProperty
// ============================================================================

/**
 * A property after value normalization.
 *
 * Passed to `beginNode()` for each property on a node element.
 */
export interface ResolvedProperty {
  /**
   * The property name in Mosaic source — always kebab-case, e.g. `"corner-radius"`,
   * `"font-size"`, `"padding-top"`.
   */
  name: string;

  /** The fully resolved value. */
  value: ResolvedValue;
}

// ============================================================================
// SlotContext
// ============================================================================

/**
 * Scope tracking for slot resolution during tree traversal.
 *
 * The VM creates a root `SlotContext` from the component's slot list and
 * maintains it throughout the walk. When entering an `each` block, the VM
 * creates a new inner context with the loop variable pushed onto `loopScopes`.
 *
 * Resolution priority (innermost-first):
 *   1. `loopScopes` — innermost loop variable wins
 *   2. `componentSlots` — component-level slots
 *   3. Neither — `MosaicVMError` (should have been caught by analyzer)
 */
export interface SlotContext {
  /**
   * All slots declared on the component being compiled.
   * Key: slot name (e.g. `"title"`), Value: full `MosaicSlot` definition.
   */
  componentSlots: ReadonlyMap<string, MosaicSlot>;

  /**
   * The stack of active `each` loop variables, innermost last.
   *
   * When the VM enters `each @items as item { ... }`:
   *   - `itemName` is the loop variable name (`"item"`)
   *   - `elementType` is the T in `list<T>` — the type of each element
   */
  loopScopes: ReadonlyArray<{ itemName: string; elementType: MosaicType }>;
}

// ============================================================================
// MosaicRenderer
// ============================================================================

/**
 * The backend protocol — every code generator implements this interface.
 *
 * The VM calls these methods in strict depth-first, open-before-close order.
 * A backend typically accumulates generated source code in a string buffer
 * and flushes it in `emit()`.
 *
 * Order guarantee:
 *   beginComponent
 *     beginNode (root)
 *       beginNode (child-1) ... endNode (child-1)
 *       beginWhen
 *         beginNode (when-child) ... endNode (when-child)
 *       endWhen
 *       beginEach
 *         beginNode (each-child) ... endNode (each-child)
 *       endEach
 *       renderSlotChild (for @slotName; children)
 *     endNode (root)
 *   endComponent
 *   emit  ← called by MosaicVM.run()
 */
export interface MosaicRenderer {
  // --------------------------------------------------------------------------
  // Component lifecycle
  // --------------------------------------------------------------------------

  /**
   * Called once before any tree traversal.
   *
   * Use this to initialize output buffers and emit file headers, import
   * statements, class/function preambles, and prop type declarations.
   *
   * @param name - The component name (PascalCase), e.g. `"ProfileCard"`.
   * @param slots - All declared slots, in source order.
   */
  beginComponent(name: string, slots: MosaicSlot[]): void;

  /**
   * Called once after all tree traversal is complete.
   *
   * Use this to close any open structures (function bodies, class bodies)
   * and finalize output buffers before `emit()` is called.
   */
  endComponent(): void;

  /**
   * Called by `MosaicVM.run()` at the very end to collect generated files.
   *
   * @returns An array of files, each with a relative `filename` and a `content` string.
   */
  emit(): MosaicEmitResult;

  // --------------------------------------------------------------------------
  // Node lifecycle
  // --------------------------------------------------------------------------

  /**
   * Called when entering a node element.
   *
   * @param tag - Element type name, e.g. `"Row"`, `"Text"`, `"Button"`.
   * @param isPrimitive - `true` for built-in elements (Row, Column, Text, etc.),
   *   `false` for imported component types.
   * @param properties - All resolved property assignments, in source order.
   * @param context - The `SlotContext` at this point in the tree.
   */
  beginNode(
    tag: string,
    isPrimitive: boolean,
    properties: ResolvedProperty[],
    context: SlotContext,
  ): void;

  /**
   * Called when leaving a node element (after all its children are processed).
   *
   * @param tag - The same tag passed to `beginNode`.
   */
  endNode(tag: string): void;

  // --------------------------------------------------------------------------
  // Children
  // --------------------------------------------------------------------------

  /**
   * Called when a slot reference appears as a **child** of a node.
   *
   * This is for the `@slotName;` form inside a node body, not for slot refs
   * used as property values (those appear in `ResolvedProperty`).
   *
   * Example: `Column { @action; }` → `renderSlotChild("action", actionSlotType, ctx)`
   *
   * @param slotName - The slot being referenced, e.g. `"action"`.
   * @param slotType - The declared `MosaicType` of that slot.
   * @param context - The current `SlotContext`.
   */
  renderSlotChild(slotName: string, slotType: MosaicType, context: SlotContext): void;

  // --------------------------------------------------------------------------
  // Conditional rendering
  // --------------------------------------------------------------------------

  /**
   * Called when entering a `when @flag { ... }` block.
   *
   * The bool slot `flag` is in `context.componentSlots`.
   *
   * @param slotName - The bool slot that gates this block.
   * @param context - The current `SlotContext`.
   */
  beginWhen(slotName: string, context: SlotContext): void;

  /** Called when leaving a `when` block (after all its children). */
  endWhen(): void;

  // --------------------------------------------------------------------------
  // Iteration
  // --------------------------------------------------------------------------

  /**
   * Called when entering an `each @items as item { ... }` block.
   *
   * After this call, the VM pushes `{ itemName, elementType }` onto
   * `context.loopScopes` so references to `@item` inside the block resolve
   * as loop variables.
   *
   * @param slotName - The list slot being iterated, e.g. `"items"`.
   * @param itemName - The loop variable name, e.g. `"item"`.
   * @param elementType - The `T` in `list<T>` — the type of each element.
   * @param context - The current `SlotContext` (loop scope not yet pushed).
   */
  beginEach(
    slotName: string,
    itemName: string,
    elementType: MosaicType,
    context: SlotContext,
  ): void;

  /**
   * Called when leaving an `each` block (after all its children).
   *
   * The VM pops the loop scope from `loopScopes` before calling this.
   */
  endEach(): void;
}

// ============================================================================
// MosaicEmitResult
// ============================================================================

/**
 * The output of a backend — one or more generated files.
 *
 * Each entry has a relative `filename` (no leading `/`) and the complete
 * file `content` as a string. Most backends emit one file per component.
 * Some backends (e.g., one that generates both `.ts` and `.css`) emit two.
 */
export interface MosaicEmitResult {
  files: Array<{
    /** Relative output filename, e.g. `"ProfileCard.tsx"` or `"mosaic-profile-card.ts"`. */
    filename: string;
    /** Complete file content as a UTF-8 string. */
    content: string;
  }>;
}
