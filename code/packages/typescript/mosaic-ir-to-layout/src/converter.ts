/**
 * Mosaic IR → LayoutNode Converter
 *
 * Takes a `MosaicComponent` (output of mosaic-analyzer) and a map of resolved
 * slot values (the component's props at render time), and produces a
 * `LayoutNode` tree with `ext["flex"]` and `ext["paint"]` populated, ready
 * for `layout-flexbox`.
 *
 * Pipeline position:
 *
 *   MosaicComponent IR + slot values + theme
 *       ↓  mosaic_ir_to_layout()
 *   LayoutNode tree (ext["flex"] + ext["paint"])
 *       ↓  layout-flexbox
 *   PositionedNode tree
 *       ↓  layout-to-paint
 *   PaintScene
 *
 * Design: Slot Value Resolution
 * -----------------------------
 *
 * Every `@slotName` reference in the Mosaic source is replaced by a concrete
 * runtime value from the `slots` map. The converter resolves references on
 * the fly as it walks the node tree.
 *
 * Resolution priority:
 *   1. `slots` map (caller-provided runtime values)
 *   2. Slot's `defaultValue` from the IR
 *   3. Type-appropriate zero value (empty string, 0, false, transparent)
 *
 * Design: Node → LayoutNode Mapping
 * ----------------------------------
 *
 * Each Mosaic primitive maps to a LayoutNode with specific ext fields:
 *
 *   Column   → flex container, direction: "column"
 *   Row      → flex container, direction: "row"
 *   Box      → flex container, direction: "column", wrap: "wrap"
 *   Text     → leaf with TextContent
 *   Image    → leaf with ImageContent
 *   Spacer   → flex-grow filler
 *   Divider  → thin horizontal rule
 *   Scroll   → flex container with overflow hint
 *
 * Non-primitive nodes (imported components) become placeholder containers that
 * the caller can fill with the referenced component's own LayoutNode tree.
 *
 * See: code/specs/UI05-mosaic-ir-to-layout.md
 */

import type {
  LayoutNode,
  SizeValue,
  Color,
  FontSpec,
} from "@coding-adventures/layout-ir";
import {
  size_fixed,
  size_fill,
  size_wrap,
  edges_all,
  edges_zero,
  rgb,
  rgba,
  font_spec,
} from "@coding-adventures/layout-ir";
import type { FlexContainerExt, FlexItemExt } from "@coding-adventures/layout-flexbox";
import type { PaintExt } from "@coding-adventures/layout-to-paint";
import type {
  MosaicComponent,
  MosaicNode,
  MosaicChild,
  MosaicValue,
  MosaicSlot,
} from "@coding-adventures/mosaic-analyzer";

// ── Slot value types ───────────────────────────────────────────────────────

/**
 * A runtime slot value — what the host provides when rendering a component.
 *
 * Unlike `MosaicValue` (which is a source-code representation), `SlotValue`
 * is a JavaScript runtime value that has already been evaluated.
 *
 * The six slot value kinds mirror the six Mosaic slot types:
 *   - `text` slot   → string
 *   - `number` slot → number
 *   - `bool` slot   → boolean
 *   - `image` slot  → string (URL or data URI)
 *   - `color` slot  → Color (from layout-ir)
 *   - `node` slot   → empty LayoutNode placeholder
 *   - `list` slot   → SlotValue[]
 */
export type SlotValue =
  | string
  | number
  | boolean
  | Color
  | LayoutNode
  | SlotValue[];

/** Map of slot names to their runtime values */
export type SlotMap = Map<string, SlotValue>;

// ── Theme ──────────────────────────────────────────────────────────────────

/**
 * Default visual style applied when a Mosaic property is not explicitly set.
 *
 * The theme is the only "global" configuration. All visual defaults come from
 * here — there is no CSS cascade or style inheritance between components.
 */
export interface MosaicLayoutTheme {
  /** Default font for Text nodes with no explicit font properties. */
  defaultFont: FontSpec;
  /** Default text color for Text nodes with no explicit color. */
  defaultTextColor: Color;
  /** Scale factor: 1 dp = this many logical pixels. Default: 1.0. */
  baseFontSize: number;
}

/**
 * Create the standard default theme.
 *
 * This theme matches web conventions: 16px body text in dark charcoal.
 */
export function mosaic_default_theme(): MosaicLayoutTheme {
  return {
    defaultFont: font_spec("system-ui", 16),
    defaultTextColor: rgb(17, 17, 17),
    baseFontSize: 1.0,
  };
}

// ── Colour parsing ─────────────────────────────────────────────────────────

/**
 * Parse a CSS hex color string (#rgb, #rrggbb, #rrggbbaa) into a Color.
 *
 * The hash `#` is optional. If parsing fails, returns transparent black.
 */
function parseHexColor(hex: string): Color {
  const s = hex.replace(/^#/, "");
  if (s.length === 3 || s.length === 4) {
    const r = parseInt(s[0] + s[0], 16);
    const g = parseInt(s[1] + s[1], 16);
    const b = parseInt(s[2] + s[2], 16);
    const a = s.length === 4 ? parseInt(s[3] + s[3], 16) : 255;
    return rgba(r, g, b, a);
  }
  if (s.length === 6 || s.length === 8) {
    const r = parseInt(s.slice(0, 2), 16);
    const g = parseInt(s.slice(2, 4), 16);
    const b = parseInt(s.slice(4, 6), 16);
    const a = s.length === 8 ? parseInt(s.slice(6, 8), 16) : 255;
    return rgba(r, g, b, a);
  }
  return rgba(0, 0, 0, 0); // transparent fallback
}

// ── Value resolution ───────────────────────────────────────────────────────

/**
 * Resolve a `MosaicValue` to a concrete JavaScript value.
 *
 * The converter reads `MosaicValue` from the IR and resolves all slot
 * references using the runtime `slots` map and the `loopContext`.
 *
 * @param value     The IR value to resolve
 * @param slots     Runtime slot values from the caller
 * @param component The component (for finding slot defaults)
 * @param loopContext  Optional loop variable bindings from `each` blocks
 */
function resolveValue(
  value: MosaicValue,
  slots: SlotMap,
  component: MosaicComponent,
  loopContext: Map<string, SlotValue>
): SlotValue {
  switch (value.kind) {
    case "string": return value.value;
    case "number": return value.value;
    case "bool":   return value.value;
    case "ident":  return value.value;       // bare identifiers become strings
    case "enum":   return value.member;      // enum → the member string

    case "dimension":
      // Convert to logical pixels. "dp" → multiply by baseFontSize at resolve time.
      // We return the raw number and let the consumer apply the unit if needed.
      return value.value;

    case "color_hex":
      return parseHexColor(value.value);

    case "slot_ref": {
      const { slotName } = value;

      // Check loop variable first (innermost scope)
      if (loopContext.has(slotName)) return loopContext.get(slotName)!;

      // Runtime slot value
      if (slots.has(slotName)) return slots.get(slotName)!;

      // Slot default from IR
      const slot = component.slots.find(s => s.name === slotName);
      if (slot?.defaultValue) return resolveValue(slot.defaultValue, slots, component, loopContext);

      // Zero value by type
      if (slot) return zeroForSlot(slot);

      return ""; // unknown slot → empty string
    }
  }
}

/** Zero value for a slot with no default and no runtime value. */
function zeroForSlot(slot: MosaicSlot): SlotValue {
  switch (slot.type.kind) {
    case "text":      return "";
    case "number":    return 0;
    case "bool":      return false;
    case "image":     return "";
    case "color":     return rgba(0, 0, 0, 0);
    case "node":      return emptyContainer();
    case "component": return emptyContainer();
    case "list":      return [];
  }
}

function emptyContainer(): LayoutNode {
  return {
    width: size_fill(),
    height: size_wrap(),
    content: null,
    children: [],
    ext: {},
    minWidth: undefined,
    maxWidth: undefined,
    minHeight: undefined,
    maxHeight: undefined,
    margin: edges_zero(),
    padding: edges_zero(),
  };
}

// ── Property reading helpers ───────────────────────────────────────────────

/**
 * Find a property value from a node's property list by name.
 * Returns the resolved JavaScript value, or `undefined` if not present.
 */
function prop(
  node: MosaicNode,
  name: string,
  slots: SlotMap,
  component: MosaicComponent,
  loopContext: Map<string, SlotValue>
): SlotValue | undefined {
  const p = node.properties.find(p => p.name === name);
  if (!p) return undefined;
  return resolveValue(p.value, slots, component, loopContext);
}

/** Read a string property. */
function strProp(
  node: MosaicNode,
  name: string,
  slots: SlotMap,
  component: MosaicComponent,
  loopContext: Map<string, SlotValue>,
  fallback = ""
): string {
  const v = prop(node, name, slots, component, loopContext);
  return typeof v === "string" ? v : fallback;
}

/** Read a numeric property (from a dimension or number value). */
function numProp(
  node: MosaicNode,
  name: string,
  slots: SlotMap,
  component: MosaicComponent,
  loopContext: Map<string, SlotValue>,
  fallback = 0
): number {
  const v = prop(node, name, slots, component, loopContext);
  return typeof v === "number" ? v : fallback;
}

/** Read a Color property. */
function colorProp(
  node: MosaicNode,
  name: string,
  slots: SlotMap,
  component: MosaicComponent,
  loopContext: Map<string, SlotValue>
): Color | undefined {
  const v = prop(node, name, slots, component, loopContext);
  if (v && typeof v === "object" && !Array.isArray(v) && "r" in v) {
    return v as Color;
  }
  return undefined;
}

// ── Size resolution ────────────────────────────────────────────────────────

/**
 * Resolve a size property to a `SizeValue`.
 *
 * Mosaic uses the `width` and `height` properties:
 *   - "fill"    → size_fill()
 *   - "wrap"    → size_wrap()
 *   - number    → size_fixed(n)
 *   - dimension → size_fixed(n)
 */
function resolveSize(
  node: MosaicNode,
  propName: string,
  fallback: SizeValue,
  slots: SlotMap,
  component: MosaicComponent,
  loopContext: Map<string, SlotValue>
): SizeValue {
  const v = prop(node, propName, slots, component, loopContext);
  if (v === "fill") return size_fill();
  if (v === "wrap") return size_wrap();
  if (typeof v === "number" && v > 0) return size_fixed(v);
  return fallback;
}

// ── Paint ext builder ──────────────────────────────────────────────────────

type Elevation = "none" | "low" | "medium" | "high";

const SHADOW_TABLE: Record<Elevation, Omit<PaintExt, "backgroundColor" | "borderWidth" | "borderColor" | "cornerRadius" | "opacity">> = {
  none:   { shadowColor: rgba(0, 0, 0, 0),  shadowOffsetX: 0, shadowOffsetY: 0, shadowBlur: 0 },
  low:    { shadowColor: rgba(0, 0, 0, 31), shadowOffsetX: 0, shadowOffsetY: 1, shadowBlur: 3 },
  medium: { shadowColor: rgba(0, 0, 0, 38), shadowOffsetX: 0, shadowOffsetY: 4, shadowBlur: 12 },
  high:   { shadowColor: rgba(0, 0, 0, 51), shadowOffsetX: 0, shadowOffsetY: 8, shadowBlur: 24 },
};

/**
 * Build a `PaintExt` from a node's visual decoration properties.
 *
 * Reads: background, border-width, border-color, corner-radius, opacity, shadow
 */
function paintExtFromProps(
  node: MosaicNode,
  slots: SlotMap,
  component: MosaicComponent,
  loopContext: Map<string, SlotValue>
): PaintExt {
  const ext: PaintExt = {};

  const bg = colorProp(node, "background", slots, component, loopContext);
  if (bg) ext.backgroundColor = bg;

  const borderW = numProp(node, "border-width", slots, component, loopContext, 0);
  if (borderW > 0) {
    ext.borderWidth = borderW;
    const borderC = colorProp(node, "border-color", slots, component, loopContext);
    if (borderC) ext.borderColor = borderC;
  }

  const cr = numProp(node, "corner-radius", slots, component, loopContext, 0);
  if (cr > 0) ext.cornerRadius = cr;

  const opacity = prop(node, "opacity", slots, component, loopContext);
  if (typeof opacity === "number" && opacity < 1.0) ext.opacity = opacity;

  // Shadow: read as "shadow: elevation.low" etc.
  const shadowProp = node.properties.find(p => p.name === "shadow");
  if (shadowProp) {
    const sv = shadowProp.value;
    let elevation: Elevation = "none";
    if (sv.kind === "enum" && sv.namespace === "elevation") {
      elevation = sv.member as Elevation;
    } else if (sv.kind === "ident") {
      elevation = sv.value as Elevation;
    }
    const shadows = SHADOW_TABLE[elevation] ?? SHADOW_TABLE.none;
    Object.assign(ext, shadows);
  }

  return ext;
}

// ── Font / color builders ──────────────────────────────────────────────────

/**
 * Build a FontSpec from a node's font-related properties and the theme defaults.
 */
function fontFromProps(
  node: MosaicNode,
  theme: MosaicLayoutTheme,
  slots: SlotMap,
  component: MosaicComponent,
  loopContext: Map<string, SlotValue>
): FontSpec {
  let f = { ...theme.defaultFont };

  const size = prop(node, "font-size", slots, component, loopContext);
  if (typeof size === "number" && size > 0) f = { ...f, size };

  const weight = prop(node, "font-weight", slots, component, loopContext);
  if (typeof weight === "number") f = { ...f, weight };

  const italic = prop(node, "font-style", slots, component, loopContext);
  if (italic === "italic") f = { ...f, italic: true };

  // style: heading.large etc. → map to predefined sizes
  const styleProp = node.properties.find(p => p.name === "style");
  if (styleProp) {
    const sv = styleProp.value;
    if (sv.kind === "enum" && sv.namespace === "heading") {
      switch (sv.member) {
        case "large":  f = { ...f, size: 32, weight: 700 }; break;
        case "medium": f = { ...f, size: 24, weight: 600 }; break;
        case "small":  f = { ...f, size: 18, weight: 600 }; break;
      }
    } else if (sv.kind === "ident") {
      switch (sv.value) {
        case "large":  f = { ...f, size: 32 }; break;
        case "caption": f = { ...f, size: 12 }; break;
      }
    }
  }

  return f;
}

/** Align prop value → FlexContainerExt.alignItems */
function alignToItems(v: string): FlexContainerExt["alignItems"] {
  switch (v) {
    case "start":             return "start";
    case "center":
    case "center-horizontal": return "center";
    case "end":               return "end";
    case "stretch":           return "stretch";
    default:                  return "start";
  }
}

// ── Node conversion ────────────────────────────────────────────────────────

/**
 * Convert a `MosaicNode` to a `LayoutNode`.
 *
 * This is the core recursive function. It dispatches on the node's `tag` for
 * primitive nodes, and produces placeholder containers for non-primitive nodes.
 */
function convertNode(
  mNode: MosaicNode,
  slots: SlotMap,
  component: MosaicComponent,
  theme: MosaicLayoutTheme,
  loopContext: Map<string, SlotValue>
): LayoutNode {
  if (!mNode.isPrimitive) {
    // Non-primitive: placeholder container for the caller to fill
    return {
      ...emptyContainer(),
      ext: { flex: {} as FlexContainerExt, _componentRef: mNode.tag },
    };
  }

  const get = (name: string, fallback = "") =>
    strProp(mNode, name, slots, component, loopContext, fallback);
  const getN = (name: string, fallback = 0) =>
    numProp(mNode, name, slots, component, loopContext, fallback);
  const getSize = (name: string, fallback: SizeValue) =>
    resolveSize(mNode, name, fallback, slots, component, loopContext);
  const paint = () => paintExtFromProps(mNode, slots, component, loopContext);
  const children = () => convertChildren(mNode.children, slots, component, theme, loopContext);

  switch (mNode.tag) {
    // ── Column ──────────────────────────────────────────────────────────
    case "Column": {
      const flexExt: FlexContainerExt = {
        direction: "column",
        gap: getN("gap"),
        alignItems: alignToItems(get("align", "start")),
        justifyContent: get("justify", "start") as FlexContainerExt["justifyContent"],
      };
      const p = getN("padding");
      const padding = p > 0
        ? edges_all(p)
        : {
            top: getN("padding-top"), right: getN("padding-right"),
            bottom: getN("padding-bottom"), left: getN("padding-left"),
          };
      return {
        width: getSize("width", size_fill()),
        height: getSize("height", size_wrap()),
        content: null,
        children: children(),
        ext: { flex: flexExt, paint: paint() },
        minWidth: undefined, maxWidth: undefined,
        minHeight: undefined, maxHeight: undefined,
        margin: edges_zero(),
        padding,
      };
    }

    // ── Row ─────────────────────────────────────────────────────────────
    case "Row": {
      const flexExt: FlexContainerExt = {
        direction: "row",
        gap: getN("gap"),
        alignItems: alignToItems(get("align", "start")),
        justifyContent: get("justify", "start") as FlexContainerExt["justifyContent"],
      };
      const p = getN("padding");
      const padding = p > 0
        ? edges_all(p)
        : {
            top: getN("padding-top"), right: getN("padding-right"),
            bottom: getN("padding-bottom"), left: getN("padding-left"),
          };
      return {
        width: getSize("width", size_fill()),
        height: getSize("height", size_wrap()),
        content: null,
        children: children(),
        ext: { flex: flexExt, paint: paint() },
        minWidth: undefined, maxWidth: undefined,
        minHeight: undefined, maxHeight: undefined,
        margin: edges_zero(),
        padding,
      };
    }

    // ── Box ─────────────────────────────────────────────────────────────
    case "Box": {
      const flexExt: FlexContainerExt = { direction: "column", wrap: "wrap" };
      return {
        width: getSize("width", size_fill()),
        height: getSize("height", size_wrap()),
        content: null,
        children: children(),
        ext: { flex: flexExt, paint: paint() },
        minWidth: undefined, maxWidth: undefined,
        minHeight: undefined, maxHeight: undefined,
        margin: edges_zero(),
        padding: edges_zero(),
      };
    }

    // ── Text ─────────────────────────────────────────────────────────────
    case "Text": {
      const textValue = get("content");
      const font = fontFromProps(mNode, theme, slots, component, loopContext);
      const color = colorProp(mNode, "color", slots, component, loopContext)
        ?? theme.defaultTextColor;
      const maxLinesRaw = prop(mNode, "max-lines", slots, component, loopContext);
      const maxLines = typeof maxLinesRaw === "number" ? maxLinesRaw : null;
      const textAlign = get("text-align", "start") as "start" | "center" | "end" | "justify";

      const flexItemExt: FlexItemExt = { grow: 0, shrink: 1 };

      return {
        width: size_wrap(),
        height: size_wrap(),
        content: { kind: "text", value: textValue, font, color, maxLines, textAlign },
        children: [],
        ext: { flex: flexItemExt },
        minWidth: undefined, maxWidth: undefined,
        minHeight: undefined, maxHeight: undefined,
        margin: edges_zero(),
        padding: edges_zero(),
      };
    }

    // ── Image ────────────────────────────────────────────────────────────
    case "Image": {
      const src = get("source");
      const fit = (get("fit", "contain") || "contain") as "contain" | "cover" | "fill" | "none";

      // shape: "circle" → cornerRadius (use 9999 as sentinel = "half the width")
      // shape: "rounded" → cornerRadius = 8
      const shapePropEntry = mNode.properties.find(p => p.name === "shape");
      const paintExt: PaintExt = paint();
      if (shapePropEntry) {
        const sv = shapePropEntry.value;
        const shapeVal = sv.kind === "ident" ? sv.value :
                         (sv.kind === "enum" ? sv.member : "");
        if (shapeVal === "circle") {
          paintExt.cornerRadius = 9999;
        } else if (shapeVal === "rounded") {
          paintExt.cornerRadius = 8;
        }
      }

      return {
        width: getSize("width", getSize("size", size_wrap())),
        height: getSize("height", getSize("size", size_wrap())),
        content: { kind: "image", src, fit },
        children: [],
        ext: { paint: paintExt },
        minWidth: undefined, maxWidth: undefined,
        minHeight: undefined, maxHeight: undefined,
        margin: edges_zero(),
        padding: edges_zero(),
      };
    }

    // ── Spacer ───────────────────────────────────────────────────────────
    case "Spacer": {
      const flexItemExt: FlexItemExt = { grow: 1, shrink: 0 };
      return {
        width: size_fill(),
        height: size_fill(),
        content: null,
        children: [],
        ext: { flex: flexItemExt },
        minWidth: undefined, maxWidth: undefined,
        minHeight: undefined, maxHeight: undefined,
        margin: edges_zero(),
        padding: edges_zero(),
      };
    }

    // ── Divider ──────────────────────────────────────────────────────────
    case "Divider": {
      // A thin horizontal rule at 20% opacity of the default text color
      const { r, g, b } = theme.defaultTextColor;
      const dividerPaint: PaintExt = {
        backgroundColor: rgba(r, g, b, Math.round(255 * 0.2)),
      };
      return {
        width: size_fill(),
        height: size_fixed(1),
        content: null,
        children: [],
        ext: { paint: dividerPaint },
        minWidth: undefined, maxWidth: undefined,
        minHeight: undefined, maxHeight: undefined,
        margin: edges_zero(),
        padding: edges_zero(),
      };
    }

    // ── Scroll ───────────────────────────────────────────────────────────
    case "Scroll": {
      const flexExt: FlexContainerExt = { direction: "column", wrap: "nowrap" };
      const overflowPaint: PaintExt = { overflow: "scroll" };
      return {
        width: getSize("width", size_fill()),
        height: getSize("height", size_fill()),
        content: null,
        children: children(),
        ext: { flex: flexExt, paint: overflowPaint },
        minWidth: undefined, maxWidth: undefined,
        minHeight: undefined, maxHeight: undefined,
        margin: edges_zero(),
        padding: edges_zero(),
      };
    }

    // ── Unknown primitive → empty container ──────────────────────────────
    default:
      return { ...emptyContainer(), ext: { _unknownTag: mNode.tag } };
  }
}

/**
 * Convert a list of `MosaicChild` entries to `LayoutNode[]`.
 *
 * Handles: node children, slot-ref children (placeholder), when blocks,
 * each blocks (iteration).
 */
function convertChildren(
  children: MosaicChild[],
  slots: SlotMap,
  component: MosaicComponent,
  theme: MosaicLayoutTheme,
  loopContext: Map<string, SlotValue>
): LayoutNode[] {
  const result: LayoutNode[] = [];

  for (const child of children) {
    switch (child.kind) {
      case "node":
        result.push(convertNode(child.node, slots, component, theme, loopContext));
        break;

      case "slot_ref": {
        // A slot reference used as a child: `@action;`
        // If the slot holds a LayoutNode, use it. Otherwise produce a placeholder.
        const val = slots.get(child.slotName);
        if (val && typeof val === "object" && !Array.isArray(val) && "content" in val) {
          result.push(val as LayoutNode);
        } else {
          result.push({ ...emptyContainer(), ext: { _slotRef: child.slotName } });
        }
        break;
      }

      case "when": {
        // Include children only when the bool slot resolves to true
        const condition = slots.get(child.slotName) ?? false;
        if (condition === true) {
          result.push(...convertChildren(child.children, slots, component, theme, loopContext));
        }
        break;
      }

      case "each": {
        // Iterate over the list slot and convert the template for each element
        const listVal = slots.get(child.slotName);
        const list: SlotValue[] = Array.isArray(listVal) ? listVal : [];
        for (const element of list) {
          const innerLoop = new Map(loopContext);
          innerLoop.set(child.itemName, element);
          result.push(...convertChildren(child.children, slots, component, theme, innerLoop));
        }
        break;
      }
    }
  }

  return result;
}

// ── Public API ─────────────────────────────────────────────────────────────

/**
 * Convert a `MosaicComponent` IR to a `LayoutNode` tree.
 *
 * This is a **runtime** converter — it runs when a component is about to be
 * rendered, with all slot values known. Contrast with the React/WebComponent
 * emitters (compile-time), which generate code with conditional expressions.
 *
 * @param component  The MosaicComponent IR (output of mosaic-analyzer)
 * @param slots      Runtime slot values keyed by slot name
 * @param theme      Visual style defaults (font, text color, scale)
 * @returns Root `LayoutNode` with `ext["flex"]` and `ext["paint"]` populated
 */
export function mosaic_ir_to_layout(
  component: MosaicComponent,
  slots: SlotMap,
  theme: MosaicLayoutTheme
): LayoutNode {
  const loopContext = new Map<string, SlotValue>();
  return convertNode(component.tree, slots, component, theme, loopContext);
}
