/**
 * MosaicVM — Generic tree-walking driver for Mosaic compiler backends.
 *
 * The MosaicVM is the fourth stage of the Mosaic compiler pipeline:
 *
 *   Source text → Lexer → Parser → Analyzer → MosaicIR → **VM** → Backend → Target code
 *
 * The VM's responsibilities:
 *   1. Traverse the `MosaicIR` tree depth-first.
 *   2. Normalize every `MosaicValue` into a `ResolvedValue` (hex → RGBA, dimension → {value, unit}).
 *   3. Track the `SlotContext` (component slots + active each-loop scopes).
 *   4. Call `MosaicRenderer` methods in strict open-before-close order.
 *
 * What the VM Does NOT Do
 * -----------------------
 *
 * The VM is agnostic about output format. It has no knowledge of React, Web
 * Components, SwiftUI, or any other platform. Backends own the output — the VM
 * only drives the traversal and normalizes values.
 *
 * This separation mirrors the design of the JVM or CLR: the VM handles
 * execution mechanics; the platform code handles platform-specific behavior.
 *
 * Traversal Order
 * ---------------
 *
 * The VM visits nodes depth-first, calling `beginNode` before children and
 * `endNode` after. The exact call sequence for a component is:
 *
 *   beginComponent(name, slots)
 *     beginNode(root, isPrimitive, resolvedProps, ctx)
 *       [for each child of root in source order:]
 *         beginNode(child, ...) ... endNode(child)   ← child nodes
 *         renderSlotChild(...)                        ← @slotName; children
 *         beginWhen(...)                              ← when blocks
 *           [when children]
 *         endWhen()
 *         beginEach(...)                              ← each blocks
 *           [each children — with loop scope pushed]
 *         endEach()
 *     endNode(root)
 *   endComponent()
 *   emit() → MosaicEmitResult
 *
 * Color Parsing
 * -------------
 *
 * Hex colors use these expansion rules:
 *
 *   | Source   | r  | g  | b  | a   |
 *   |----------|----|----|----|----|
 *   | #rgb     | rr | gg | bb | 255 |
 *   | #rrggbb  | rr | gg | bb | 255 |
 *   | #rrggbbaa| rr | gg | bb | aa  |
 *
 * For three-digit hex (`#fff`), each digit is doubled: `#fff` → `r=255, g=255, b=255`.
 *
 * Dimension Parsing
 * -----------------
 *
 * The VM splits dimension strings into numeric value and unit:
 *   "16dp" → { kind: "dimension", value: 16, unit: "dp" }
 *   "1.5sp" → { kind: "dimension", value: 1.5, unit: "sp" }
 *   "100%" → { kind: "dimension", value: 100, unit: "%" }
 *
 * Slot Ref Resolution
 * -------------------
 *
 * When the VM encounters a slot ref value (`@title`, `@item`), it looks up the
 * slot in the current `SlotContext`:
 *   1. Check `loopScopes` innermost-first (for each-block loop variables).
 *   2. Fall back to `componentSlots`.
 *   3. If neither matches → throw `MosaicVMError` (the analyzer missed something).
 */

import type {
  MosaicIR,
  MosaicNode,
  MosaicChild,
  MosaicValue,
  MosaicSlot,
  MosaicType,
} from "@coding-adventures/mosaic-analyzer";

import type {
  MosaicRenderer,
  MosaicEmitResult,
  ResolvedValue,
  ResolvedProperty,
  SlotContext,
} from "./types.js";

// ============================================================================
// MosaicVMError
// ============================================================================

/**
 * Thrown when the VM encounters a runtime invariant violation.
 *
 * These are "should not happen" errors — they indicate that the analyzer
 * failed to catch an undefined slot reference or an invalid type annotation.
 * Syntactic and semantic errors should be caught earlier in the pipeline.
 */
export class MosaicVMError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MosaicVMError";
  }
}

// ============================================================================
// MosaicVM
// ============================================================================

/**
 * The generic tree-walking driver for Mosaic compiler backends.
 *
 * Construct a VM with a `MosaicIR`, then call `run(renderer)` with any
 * backend that implements `MosaicRenderer`. The VM returns the `MosaicEmitResult`
 * from the renderer.
 *
 * A single `MosaicVM` instance can be run against multiple renderers — one for
 * React, another for Web Components, etc. The VM is stateless between `run()` calls.
 *
 * @example
 *     const ir = analyzeMosaic(source);
 *     const vm = new MosaicVM(ir);
 *
 *     const reactResult = vm.run(new ReactRenderer());
 *     const webResult   = vm.run(new WebComponentRenderer());
 */
export class MosaicVM {
  constructor(private readonly ir: MosaicIR) {}

  /**
   * Traverse the IR tree, calling renderer methods in depth-first order.
   *
   * @param renderer - A backend that implements `MosaicRenderer`.
   * @returns The `MosaicEmitResult` produced by `renderer.emit()`.
   */
  run(renderer: MosaicRenderer): MosaicEmitResult {
    /**
     * Build the root SlotContext from the component's slot declarations.
     *
     * `componentSlots` is a Map so slot lookups are O(1) during traversal.
     * `loopScopes` starts empty; it grows/shrinks as we enter/leave each blocks.
     */
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

  // --------------------------------------------------------------------------
  // Tree Traversal
  // --------------------------------------------------------------------------

  /**
   * Traverse a single node: resolve properties, call beginNode, walk children,
   * call endNode.
   */
  private _walkNode(node: MosaicNode, ctx: SlotContext, r: MosaicRenderer): void {
    /**
     * Resolve every property before calling beginNode so the renderer
     * receives fully normalized values without needing to parse hex strings
     * or split dimension units.
     */
    const resolved: ResolvedProperty[] = node.properties.map((p) => ({
      name: p.name,
      value: this._resolveValue(p.value, ctx),
    }));

    r.beginNode(node.tag, node.isPrimitive, resolved, ctx);

    for (const child of node.children) {
      this._walkChild(child, ctx, r);
    }

    r.endNode(node.tag);
  }

  /**
   * Dispatch a single child to the appropriate renderer method.
   *
   * Children have four forms, each handled differently:
   *   - `node`: recurse into _walkNode
   *   - `slot_ref`: call renderSlotChild (slot used as a child, not a value)
   *   - `when`: open/close conditional block
   *   - `each`: open/close iteration block with loop scope push/pop
   */
  private _walkChild(child: MosaicChild, ctx: SlotContext, r: MosaicRenderer): void {
    switch (child.kind) {
      case "node":
        this._walkNode(child.node, ctx, r);
        break;

      case "slot_ref": {
        /**
         * The slot used as a child: `Column { @header; }`.
         * We look up the slot type so the renderer knows what kind of content
         * to project (e.g., whether to use <slot> in Web Components or just
         * render the prop directly in React).
         */
        const slot = this._resolveSlot(child.slotName, ctx);
        r.renderSlotChild(child.slotName, slot.type, ctx);
        break;
      }

      case "when":
        /**
         * Conditional block: `when @show { ... }`.
         * The renderer generates the platform conditional (e.g., `{show && ...}` in React).
         */
        r.beginWhen(child.slotName, ctx);
        for (const c of child.children) {
          this._walkChild(c, ctx, r);
        }
        r.endWhen();
        break;

      case "each": {
        /**
         * Iteration block: `each @items as item { ... }`.
         *
         * 1. Resolve the element type (the T in list<T>).
         * 2. Call beginEach on the renderer.
         * 3. Push the loop scope so @item references inside the block resolve correctly.
         * 4. Walk the block's children with the updated context.
         * 5. Call endEach (the loop scope is conceptually popped — we discard innerCtx).
         */
        const listSlot = ctx.componentSlots.get(child.slotName);
        if (!listSlot) {
          throw new MosaicVMError(`Unknown list slot: @${child.slotName}`);
        }
        if (listSlot.type.kind !== "list") {
          throw new MosaicVMError(
            `each block references @${child.slotName} but it is not a list type`
          );
        }
        const elementType: MosaicType = listSlot.type.elementType;

        r.beginEach(child.slotName, child.itemName, elementType, ctx);

        const innerCtx: SlotContext = {
          componentSlots: ctx.componentSlots,
          loopScopes: [...ctx.loopScopes, { itemName: child.itemName, elementType }],
        };

        for (const c of child.children) {
          this._walkChild(c, innerCtx, r);
        }

        r.endEach();
        break;
      }
    }
  }

  // --------------------------------------------------------------------------
  // Value Resolution
  // --------------------------------------------------------------------------

  /**
   * Normalize a `MosaicValue` into a `ResolvedValue`.
   *
   * The main transformations are:
   *   - `color_hex` → parsed RGBA integers
   *   - `dimension` → extracted numeric value + unit string
   *   - `ident` → folded into `string` (bare identifiers used as property values)
   *   - `slot_ref` → enriched with slot type info and loop-variable flag
   *   - All others pass through unchanged
   */
  private _resolveValue(v: MosaicValue, ctx: SlotContext): ResolvedValue {
    switch (v.kind) {
      case "string":    return { kind: "string", value: v.value };
      case "number":    return { kind: "number", value: v.value };
      case "bool":      return { kind: "bool", value: v.value };
      case "ident":     return { kind: "string", value: v.value };
      case "dimension": return this._parseDimension(v.value, v.unit);
      case "color_hex": return this._parseColor(v.value);
      case "enum":      return { kind: "enum", namespace: v.namespace, member: v.member };
      case "slot_ref":  return this._resolveSlotRef(v.slotName, ctx);
    }
  }

  /**
   * Convert a parsed dimension (value + unit) into a typed ResolvedValue.
   *
   * The analyzer has already split the "16dp" token into `{ value: 16, unit: "dp" }`,
   * so this method only validates the unit string.
   */
  private _parseDimension(value: number, unit: string): ResolvedValue {
    if (unit !== "dp" && unit !== "sp" && unit !== "%") {
      // Unknown unit — pass as-is by treating it as dp (permissive mode).
      // Stricter validation can be added as an optional pass.
      return { kind: "dimension", value, unit: unit as "dp" | "sp" | "%" };
    }
    return { kind: "dimension", value, unit };
  }

  /**
   * Parse a hex color string into RGBA integer components.
   *
   * Three-digit hex: `#rgb` → doubles each digit → `#rrggbb`.
   * Six-digit hex: `#rrggbb` → alpha defaults to 255.
   * Eight-digit hex: `#rrggbbaa` → all four channels explicit.
   */
  private _parseColor(hex: string): ResolvedValue {
    const h = hex.slice(1); // strip leading '#'
    let r: number, g: number, b: number, a = 255;

    if (h.length === 3) {
      // Three-digit shorthand: #rgb → #rrggbb
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

  /**
   * Resolve a slot reference to a typed `ResolvedValue`.
   *
   * Checks loop scopes innermost-first, then component slots.
   * Throws `MosaicVMError` if the slot name is not found — this indicates
   * a bug in the analyzer (undefined slot should have been caught earlier).
   */
  private _resolveSlotRef(slotName: string, ctx: SlotContext): ResolvedValue {
    // 1. Check active loop scopes, innermost first.
    for (let i = ctx.loopScopes.length - 1; i >= 0; i--) {
      const scope = ctx.loopScopes[i];
      if (scope.itemName === slotName) {
        return {
          kind: "slot_ref",
          slotName,
          slotType: scope.elementType,
          isLoopVar: true,
        };
      }
    }

    // 2. Fall back to component slots.
    const slot = ctx.componentSlots.get(slotName);
    if (!slot) {
      throw new MosaicVMError(`Unresolved slot reference: @${slotName}`);
    }
    return { kind: "slot_ref", slotName, slotType: slot.type, isLoopVar: false };
  }

  /**
   * Look up a named slot for `renderSlotChild`.
   *
   * This is called when a slot reference appears as a child (`@header;`),
   * not as a property value. It must find a component slot (not a loop var).
   */
  private _resolveSlot(slotName: string, ctx: SlotContext): MosaicSlot {
    const slot = ctx.componentSlots.get(slotName);
    if (!slot) {
      throw new MosaicVMError(`Unknown slot: @${slotName}`);
    }
    return slot;
  }
}
