/**
 * MosaicReactRenderer — Emits a TypeScript React functional component (.tsx).
 *
 * This is the React backend for the Mosaic compiler. It implements the
 * `MosaicRenderer` interface and is driven by `MosaicVM`. Every time the VM
 * traverses the Mosaic IR tree, it calls methods on this renderer; the renderer
 * accumulates JSX strings and finalizes them when `emit()` is called.
 *
 * Architecture: String Stack
 * --------------------------
 *
 * The renderer maintains a **stack of string buffers**, one per open node.
 * When `beginNode` is called, a new buffer is pushed. When `endNode` is called,
 * the buffer is popped, wrapped in a JSX element string, and appended to the
 * parent buffer. This pattern handles arbitrary nesting without lookahead:
 *
 *   beginComponent("Card")         → stack: [component-buf]
 *   beginNode("Column")            → stack: [component-buf, column-buf]
 *   beginNode("Text")              → stack: [component-buf, column-buf, text-buf]
 *   endNode("Text")                → pop text-buf → "<span>...</span>"
 *                                    append to column-buf
 *   endNode("Column")              → pop column-buf → "<div>...<span>...</span></div>"
 *                                    append to component-buf
 *   endComponent()                 → no-op; component-buf holds the root JSX
 *   emit()                         → wrap component-buf in the full function file
 *
 * Output File Structure
 * ---------------------
 *
 * The generated .tsx file contains:
 *
 *   1. File header (auto-generated warning)
 *   2. `import React from "react";`
 *   3. Optional: `import "./mosaic-type-scale.css";` if any Text uses `style:`
 *   4. Optional: `import type { TProps } from "./T.js";` for component-type slots
 *   5. Props interface (`interface ComponentNameProps { ... }`)
 *   6. Exported function component with destructured props
 *
 * Primitive Node → JSX Element Mapping
 * -------------------------------------
 *
 *   Box     → <div style={{ position: 'relative' }}>
 *   Column  → <div style={{ display: 'flex', flexDirection: 'column' }}>
 *   Row     → <div style={{ display: 'flex', flexDirection: 'row' }}>
 *   Text    → <span> (or <h2> if a11y-role: heading)
 *   Image   → <img ... /> (self-closing)
 *   Spacer  → <div style={{ flex: 1 }}>
 *   Scroll  → <div style={{ overflow: 'auto' }}>
 *   Divider → <hr style={{ border: 'none', borderTop: '1px solid currentColor' }} />
 *
 * Property → Inline Style Mapping
 * --------------------------------
 *
 * Most Mosaic properties map directly to inline React styles. Exceptions:
 *   - `style: heading.large` (typography) → `className="mosaic-heading-large"`
 *   - `a11y-label`, `a11y-role`, `a11y-hidden` → ARIA attributes
 *   - `content` (Text) → JSX children text
 *   - `source` (Image) → `src` attribute
 *
 * Colors are always emitted as `rgba(r, g, b, alpha)`. Dimensions use `px`
 * for `dp` and `sp` (both map to CSS pixels); `%` passes through unchanged.
 *
 * Slot Refs in JSX
 * ----------------
 *
 * Since the generated function destructures its props, slot references emit
 * the variable name directly (no `props.` prefix):
 *   `@title` in content → `{title}`
 *   `@item` (loop var)  → `{item}`
 *   `@action` as child  → `{action}`
 */

import type {
  MosaicSlot,
  MosaicType,
  MosaicValue,
  MosaicRenderer,
  MosaicEmitResult,
  ResolvedProperty,
  ResolvedValue,
  SlotContext,
} from "@coding-adventures/mosaic-vm";

// ============================================================================
// Stack Frame Types
// ============================================================================

/**
 * A component-level buffer. The root frame — created in `beginComponent`,
 * holds the root JSX element(s) by the time `endComponent` is called.
 */
interface ComponentFrame {
  kind: "component";
  lines: string[];
}

/**
 * A node-level frame pushed by `beginNode` and popped by `endNode`.
 *
 * All properties are processed at push time. The `lines` array accumulates
 * JSX strings for direct children (child nodes, slot refs, when/each blocks).
 *
 * At pop time (`endNode`), the frame is converted to a JSX element string:
 *   `<${jsxTag} style={{...}} ...attrs>\n  ${children}\n</${jsxTag}>`
 */
interface NodeFrame {
  kind: "node";
  /** The original Mosaic tag name (e.g., "Column"), used for align logic. */
  tag: string;
  /** The HTML element or component name for JSX (e.g., "div", "span", "img"). */
  jsxTag: string;
  /**
   * Inline style entries. Keys are camelCase CSS properties; values are
   * already formatted as TypeScript source strings:
   *   display: '"flex"'        → emits: display: "flex"
   *   opacity: '0.5'           → emits: opacity: 0.5
   *   backgroundColor: '"rgba(37, 99, 235, 1)"'
   */
  styles: Record<string, string>;
  /** Complete JSX attribute strings, e.g. ['aria-label="Save"', 'role="button"']. */
  attrs: string[];
  /** CSS class names for the className prop, e.g. ["mosaic-heading-large"]. */
  classNames: string[];
  /**
   * For `Text` nodes: the content of the JSX children (inline, not in `lines`).
   * Example: `"{title}"` or `"Hello world"`.
   */
  textContent?: string;
  /**
   * For `Image` and `Divider`: the element is self-closing (`<img ... />`).
   * When true, `lines` and `textContent` are ignored.
   */
  selfClosing: boolean;
  /** Accumulated JSX strings from child nodes, slot refs, and blocks. */
  lines: string[];
}

/**
 * A `when @flag { ... }` block frame.
 * Children accumulate in `lines`; `endWhen` wraps them in a conditional expression.
 */
interface WhenFrame {
  kind: "when";
  slotName: string;
  lines: string[];
}

/**
 * An `each @items as item { ... }` block frame.
 * Children accumulate in `lines`; `endEach` wraps them in a `.map()` expression.
 */
interface EachFrame {
  kind: "each";
  slotName: string;
  itemName: string;
  lines: string[];
}

type StackFrame = ComponentFrame | NodeFrame | WhenFrame | EachFrame;

// ============================================================================
// MosaicReactRenderer
// ============================================================================

/**
 * The React backend for the Mosaic compiler.
 *
 * Construct this renderer, pass it to `MosaicVM.run()`, and call `emit()`
 * on the result to get the generated `.tsx` file.
 *
 * @example
 *     const ir = analyzeMosaic(source);
 *     const vm = new MosaicVM(ir);
 *     const renderer = new MosaicReactRenderer();
 *     const result = vm.run(renderer);
 *     // result.files[0].filename === "MyComponent.tsx"
 *     // result.files[0].content  === "// AUTO-GENERATED ..."
 */
export class MosaicReactRenderer implements MosaicRenderer {
  private _componentName: string = "";
  private _slots: MosaicSlot[] = [];
  private _stack: StackFrame[] = [];

  /**
   * Component type names that appear in slot types (e.g., `slot action: Button`).
   * These require `import type { ButtonProps } from "./Button.js"` in the output.
   */
  private _slotComponentImports: Set<string> = new Set();

  /**
   * Non-primitive node tags encountered during traversal (e.g., `Button { ... }`
   * as a child element). These require `import { Button } from "./Button.js"`.
   */
  private _nodeComponentImports: Set<string> = new Set();

  /**
   * Whether any `Text` node used the `style:` property (typography scale).
   * If true, the generated file will import `mosaic-type-scale.css`.
   */
  private _needsTypeScaleCSS: boolean = false;

  // --------------------------------------------------------------------------
  // MosaicRenderer implementation
  // --------------------------------------------------------------------------

  beginComponent(name: string, slots: MosaicSlot[]): void {
    this._componentName = name;
    this._slots = slots;
    this._stack = [{ kind: "component", lines: [] }];
    this._slotComponentImports = new Set();
    this._nodeComponentImports = new Set();
    this._needsTypeScaleCSS = false;

    // Pre-scan slots for component-type imports
    for (const slot of slots) {
      if (slot.type.kind === "component") {
        this._slotComponentImports.add(slot.type.name);
      } else if (slot.type.kind === "list" && slot.type.elementType.kind === "component") {
        this._slotComponentImports.add(slot.type.elementType.name);
      }
    }
  }

  endComponent(): void {
    // No-op. By the time this is called, endNode has already fired for the
    // root node, so stack[0].lines contains the complete root JSX element.
  }

  emit(): MosaicEmitResult {
    const content = this._buildFile();
    return {
      files: [{ filename: `${this._componentName}.tsx`, content }],
    };
  }

  beginNode(tag: string, isPrimitive: boolean, properties: ResolvedProperty[], _ctx: SlotContext): void {
    const frame = this._buildNodeFrame(tag, isPrimitive, properties);
    this._stack.push(frame);
  }

  endNode(_tag: string): void {
    const frame = this._stack.pop() as NodeFrame;
    const jsx = this._buildJSXElement(frame);
    this._appendToParent(jsx);
  }

  renderSlotChild(slotName: string, _slotType: MosaicType, _ctx: SlotContext): void {
    // Slot refs used as children render as the destructured prop variable.
    this._appendToParent(`{${slotName}}`);
  }

  beginWhen(slotName: string, _ctx: SlotContext): void {
    this._stack.push({ kind: "when", slotName, lines: [] });
  }

  endWhen(): void {
    const frame = this._stack.pop() as WhenFrame;
    const children = frame.lines;

    // Wrap in a React conditional expression.
    // Single child: `{flag && (<child />)}`
    // Multiple children: `{flag && (<>{...children}</>)}`
    let body: string;
    if (children.length === 1) {
      body = children[0];
    } else {
      const inner = children.map(l => "  " + l).join("\n");
      body = `<>\n${inner}\n</>`;
    }

    // Indent the body inside the conditional
    const indentedBody = body.split("\n").map(l => "  " + l).join("\n");
    const jsx = `{${frame.slotName} && (\n${indentedBody}\n)}`;
    this._appendToParent(jsx);
  }

  beginEach(slotName: string, itemName: string, _elementType: MosaicType, _ctx: SlotContext): void {
    this._stack.push({ kind: "each", slotName, itemName, lines: [] });
  }

  endEach(): void {
    const frame = this._stack.pop() as EachFrame;
    const bodyLines = frame.lines;

    // Wrap body in a React.Fragment map expression.
    // Each item renders inside <React.Fragment key={_index}>...</React.Fragment>
    // to support multiple root elements in the loop body.
    const indentedBody = bodyLines.map(l => "    " + l).join("\n");
    const jsx =
      `{${frame.slotName}.map((${frame.itemName}, _index) => (\n` +
      `  <React.Fragment key={_index}>\n` +
      `${indentedBody}\n` +
      `  </React.Fragment>\n` +
      `))}`;
    this._appendToParent(jsx);
  }

  // --------------------------------------------------------------------------
  // Node Frame Building
  // --------------------------------------------------------------------------

  /**
   * Process all properties for a node and build the NodeFrame.
   *
   * This is called at `beginNode` time, so the renderer knows the full property
   * set immediately. Child content accumulates in `frame.lines` afterwards.
   */
  private _buildNodeFrame(tag: string, isPrimitive: boolean, properties: ResolvedProperty[]): NodeFrame {
    const styles: Record<string, string> = {};
    const attrs: string[] = [];
    const classNames: string[] = [];
    let textContent: string | undefined;
    let selfClosing = false;

    // -----------------------------------------------------------------------
    // Step 1: Base styles and JSX tag from the primitive element type
    // -----------------------------------------------------------------------

    let jsxTag: string;

    if (isPrimitive) {
      switch (tag) {
        case "Box":
          jsxTag = "div";
          styles["position"] = '"relative"';
          break;
        case "Column":
          jsxTag = "div";
          styles["display"] = '"flex"';
          styles["flexDirection"] = '"column"';
          break;
        case "Row":
          jsxTag = "div";
          styles["display"] = '"flex"';
          styles["flexDirection"] = '"row"';
          break;
        case "Text":
          // May change to "h2" if a11y-role: heading is set (post-processing below)
          jsxTag = "span";
          break;
        case "Image":
          jsxTag = "img";
          selfClosing = true;
          break;
        case "Spacer":
          jsxTag = "div";
          styles["flex"] = "1";
          break;
        case "Scroll":
          jsxTag = "div";
          styles["overflow"] = '"auto"';
          break;
        case "Divider":
          jsxTag = "hr";
          selfClosing = true;
          styles["border"] = '"none"';
          styles["borderTop"] = '"1px solid currentColor"';
          break;
        default:
          jsxTag = "div";
          break;
      }
    } else {
      // Imported component — JSX uses the component name directly
      jsxTag = tag;
      this._nodeComponentImports.add(tag);
    }

    // -----------------------------------------------------------------------
    // Step 2: Apply each property to the frame
    // -----------------------------------------------------------------------

    for (const prop of properties) {
      this._applyProperty(prop, tag, styles, attrs, classNames, (tc) => {
        textContent = tc;
      });
    }

    // -----------------------------------------------------------------------
    // Step 3: Post-process — a11y-role: heading → <h2> for Text
    // -----------------------------------------------------------------------

    if (tag === "Text") {
      const headingIdx = attrs.indexOf('role="heading"');
      if (headingIdx >= 0) {
        jsxTag = "h2";
        // Remove the explicit role="heading" — <h2> carries that semantics implicitly
        attrs.splice(headingIdx, 1);
      }
    }

    return { kind: "node", tag, jsxTag, styles, attrs, classNames, textContent, selfClosing, lines: [] };
  }

  // --------------------------------------------------------------------------
  // Property Application
  // --------------------------------------------------------------------------

  /**
   * Apply a single resolved property to the current node's style/attr/className
   * collections. This is the central dispatch for Mosaic's abstract property system.
   *
   * The `tag` parameter is needed for align logic (Column vs Row vs Box) and for
   * tag-specific properties (content → Text, source → Image, etc.).
   */
  private _applyProperty(
    prop: ResolvedProperty,
    tag: string,
    styles: Record<string, string>,
    attrs: string[],
    classNames: string[],
    setTextContent: (tc: string) => void,
  ): void {
    const { name, value } = prop;

    switch (name) {
      // ----------------------------------------------------------------
      // Layout: Spacing
      // ----------------------------------------------------------------

      case "padding": {
        const d = this._dim(value);
        if (d) styles["padding"] = `"${d}"`;
        break;
      }
      case "padding-left": {
        const d = this._dim(value);
        if (d) styles["paddingLeft"] = `"${d}"`;
        break;
      }
      case "padding-right": {
        const d = this._dim(value);
        if (d) styles["paddingRight"] = `"${d}"`;
        break;
      }
      case "padding-top": {
        const d = this._dim(value);
        if (d) styles["paddingTop"] = `"${d}"`;
        break;
      }
      case "padding-bottom": {
        const d = this._dim(value);
        if (d) styles["paddingBottom"] = `"${d}"`;
        break;
      }
      case "gap": {
        const d = this._dim(value);
        if (d) styles["gap"] = `"${d}"`;
        break;
      }

      // ----------------------------------------------------------------
      // Layout: Size
      // ----------------------------------------------------------------

      case "width":
        styles["width"] = `"${this._sizeValue(value)}"`;
        break;
      case "height":
        styles["height"] = `"${this._sizeValue(value)}"`;
        break;
      case "min-width": {
        const d = this._dim(value);
        if (d) styles["minWidth"] = `"${d}"`;
        break;
      }
      case "max-width": {
        const d = this._dim(value);
        if (d) styles["maxWidth"] = `"${d}"`;
        break;
      }
      case "min-height": {
        const d = this._dim(value);
        if (d) styles["minHeight"] = `"${d}"`;
        break;
      }
      case "max-height": {
        const d = this._dim(value);
        if (d) styles["maxHeight"] = `"${d}"`;
        break;
      }

      // ----------------------------------------------------------------
      // Layout: Overflow
      // ----------------------------------------------------------------

      case "overflow":
        if (value.kind === "string") {
          const overflowMap: Record<string, string> = {
            visible: "visible",
            hidden: "hidden",
            scroll: "auto",
          };
          const v = overflowMap[value.value];
          if (v) styles["overflow"] = `"${v}"`;
        }
        break;

      // ----------------------------------------------------------------
      // Layout: Alignment
      // ----------------------------------------------------------------

      case "align":
        if (value.kind === "string") {
          this._applyAlign(value.value, tag, styles);
        }
        break;

      // ----------------------------------------------------------------
      // Visual: Background and Border
      // ----------------------------------------------------------------

      case "background":
        if (value.kind === "color") {
          styles["backgroundColor"] = `"${this._rgba(value.r, value.g, value.b, value.a)}"`;
        }
        break;
      case "corner-radius": {
        const d = this._dim(value);
        if (d) styles["borderRadius"] = `"${d}"`;
        break;
      }
      case "border-width": {
        const d = this._dim(value);
        if (d) {
          styles["borderWidth"] = `"${d}"`;
          // Setting border-width alone has no visual effect without a border-style.
          // React/CSS requires borderStyle to make the border appear.
          styles["borderStyle"] = '"solid"';
        }
        break;
      }
      case "border-color":
        if (value.kind === "color") {
          styles["borderColor"] = `"${this._rgba(value.r, value.g, value.b, value.a)}"`;
        }
        break;
      case "opacity":
        if (value.kind === "number") {
          // Opacity is a unitless number in React inline styles
          styles["opacity"] = `${value.value}`;
        }
        break;

      // ----------------------------------------------------------------
      // Visual: Shadow (elevation scale)
      // ----------------------------------------------------------------

      case "shadow":
        if (value.kind === "enum" && value.namespace === "elevation") {
          // The elevation scale maps to CSS box-shadow values.
          // These are tuned for material-design-style depth perception.
          const shadowMap: Record<string, string> = {
            none:   "none",
            low:    "0 1px 3px rgba(0,0,0,0.12)",
            medium: "0 4px 12px rgba(0,0,0,0.15)",
            high:   "0 8px 24px rgba(0,0,0,0.20)",
          };
          const s = shadowMap[value.member];
          if (s !== undefined) styles["boxShadow"] = `"${s}"`;
        }
        break;

      // ----------------------------------------------------------------
      // Visual: Visibility
      // ----------------------------------------------------------------

      case "visible":
        if (value.kind === "bool" && !value.value) {
          // `visible: false` → hide the element entirely via CSS display:none
          styles["display"] = '"none"';
        }
        // `visible: @slot` (conditional) is handled at the when-block level.
        break;

      // ----------------------------------------------------------------
      // Text-specific
      // ----------------------------------------------------------------

      case "content":
        if (tag === "Text" || tag === "span" || tag === "h2") {
          // The text content becomes the JSX children of the span/h2.
          setTextContent(this._valueToJSX(value));
        }
        break;
      case "color":
        if (value.kind === "color") {
          styles["color"] = `"${this._rgba(value.r, value.g, value.b, value.a)}"`;
        }
        break;
      case "text-align":
        if (value.kind === "string") {
          // Mosaic uses start/center/end (logical); React uses left/center/right (physical)
          const textAlignMap: Record<string, string> = {
            start: "left",
            center: "center",
            end: "right",
          };
          const a = textAlignMap[value.value];
          if (a) styles["textAlign"] = `"${a}"`;
        }
        break;
      case "font-weight": {
        if (value.kind === "string") {
          const safeFW = new Set(["100","200","300","400","500","600","700","800","900","normal","bold","bolder","lighter"]);
          if (safeFW.has(value.value)) styles["fontWeight"] = `"${value.value}"`;
        }
        break;
      }
      case "max-lines":
        if (value.kind === "number") {
          // WebKit CSS multi-line text truncation (widely supported as vendor extension)
          styles["WebkitLineClamp"] = `${value.value}`;
          styles["overflow"] = '"hidden"';
          styles["display"] = '"-webkit-box"';
          styles["WebkitBoxOrient"] = '"vertical"';
        }
        break;
      case "style":
        // Typography style enum → CSS class name (not inline style).
        // A companion `mosaic-type-scale.css` file defines the actual metrics.
        if (value.kind === "enum") {
          // e.g., heading.large → mosaic-heading-large
          classNames.push(`mosaic-${value.namespace}-${value.member}`);
          this._needsTypeScaleCSS = true;
        } else if (value.kind === "string") {
          // label and caption are treated as flat class names
          classNames.push(`mosaic-${value.value}`);
          this._needsTypeScaleCSS = true;
        }
        break;

      // ----------------------------------------------------------------
      // Image-specific
      // ----------------------------------------------------------------

      case "source":
        if (tag === "Image") {
          // image src — either a literal URL or a slot reference
          attrs.push(`src=${this._attrValue(value)}`);
        }
        break;
      case "size": {
        // Square image: set both width and height to the same dimension
        const d = this._dim(value);
        if (d && tag === "Image") {
          styles["width"] = `"${d}"`;
          styles["height"] = `"${d}"`;
        }
        break;
      }
      case "shape":
        if (tag === "Image" && value.kind === "string") {
          const shapeMap: Record<string, string> = {
            circle: "50%",
            rounded: "8px",
          };
          const r = shapeMap[value.value];
          if (r) styles["borderRadius"] = `"${r}"`;
        }
        break;
      case "fit":
        if (tag === "Image" && value.kind === "string") {
          styles["objectFit"] = `"${value.value}"`;
        }
        break;

      // ----------------------------------------------------------------
      // Accessibility
      // ----------------------------------------------------------------

      case "a11y-label":
        attrs.push(`aria-label=${this._attrValue(value)}`);
        break;
      case "a11y-role":
        if (value.kind === "string") {
          switch (value.value) {
            case "none":
              // aria-hidden removes the element from the accessibility tree entirely
              attrs.push('aria-hidden="true"');
              break;
            case "heading":
              // For Text nodes: the element becomes <h2> (post-processed above).
              // We emit the role here; _buildNodeFrame will change jsxTag and remove it.
              attrs.push('role="heading"');
              break;
            case "image":
              // Use the standard ARIA role name "img" (not "image")
              attrs.push('role="img"');
              break;
            default: {
              // Escape " to prevent HTML attribute injection in generated JSX source
              const safeRole = value.value.replace(/"/g, "&quot;").replace(/'/g, "&#39;");
              attrs.push(`role="${safeRole}"`);
              break;
            }
          }
        }
        break;
      case "a11y-hidden":
        if (value.kind === "bool" && value.value) {
          attrs.push('aria-hidden="true"');
        }
        break;
    }
  }

  // --------------------------------------------------------------------------
  // Alignment Logic
  // --------------------------------------------------------------------------

  /**
   * Apply the Mosaic `align` property to the style record.
   *
   * Alignment semantics differ by container type because the main/cross axes
   * are opposite for Column (main = block / cross = inline) vs Row (vice versa).
   *
   * Box gets `display: flex` set as a side-effect of alignment, since it is a
   * positioned container that doesn't have flex by default.
   *
   * | Mosaic align        | Column                                          | Row                                          |
   * |---------------------|-----------------------------------------------|----------------------------------------------|
   * | start               | alignItems: flex-start                          | alignItems: flex-start                       |
   * | center              | alignItems: center                              | alignItems: center, justifyContent: center   |
   * | end                 | alignItems: flex-end                            | alignItems: flex-end, justifyContent: flex-end|
   * | stretch             | alignItems: stretch                             | alignItems: stretch                          |
   * | center-horizontal   | alignItems: center (cross axis)                 | justifyContent: center (main axis)           |
   * | center-vertical     | justifyContent: center (main axis)              | alignItems: center (cross axis)              |
   */
  private _applyAlign(alignValue: string, tag: string, styles: Record<string, string>): void {
    if (tag === "Box") {
      // Box needs flex enabled for alignment to work
      styles["display"] = '"flex"';
    }

    switch (tag) {
      case "Column":
        switch (alignValue) {
          case "start":
            styles["alignItems"] = '"flex-start"'; break;
          case "center":
            styles["alignItems"] = '"center"'; break;
          case "end":
            styles["alignItems"] = '"flex-end"'; break;
          case "stretch":
            styles["alignItems"] = '"stretch"'; break;
          case "center-horizontal":
            // Horizontal = cross axis for a column
            styles["alignItems"] = '"center"'; break;
          case "center-vertical":
            // Vertical = main axis for a column
            styles["justifyContent"] = '"center"'; break;
        }
        break;

      case "Row":
        switch (alignValue) {
          case "start":
            styles["alignItems"] = '"flex-start"'; break;
          case "center":
            styles["alignItems"] = '"center"';
            styles["justifyContent"] = '"center"'; break;
          case "end":
            styles["alignItems"] = '"flex-end"';
            styles["justifyContent"] = '"flex-end"'; break;
          case "stretch":
            styles["alignItems"] = '"stretch"'; break;
          case "center-horizontal":
            // Horizontal = main axis for a row
            styles["justifyContent"] = '"center"'; break;
          case "center-vertical":
            // Vertical = cross axis for a row
            styles["alignItems"] = '"center"'; break;
        }
        break;

      case "Box":
        switch (alignValue) {
          case "start":
            styles["alignItems"] = '"flex-start"'; break;
          case "center":
            styles["alignItems"] = '"center"'; break;
          case "end":
            styles["alignItems"] = '"flex-end"'; break;
          case "stretch":
            styles["alignItems"] = '"stretch"'; break;
          case "center-horizontal":
            styles["alignItems"] = '"center"'; break;
          case "center-vertical":
            styles["justifyContent"] = '"center"'; break;
        }
        break;
    }
  }

  // --------------------------------------------------------------------------
  // JSX Building
  // --------------------------------------------------------------------------

  /**
   * Append a JSX string to the parent frame's `lines` array.
   * The parent is always the frame below the top of the stack.
   */
  private _appendToParent(content: string): void {
    const top = this._stack[this._stack.length - 1];
    if (top) top.lines.push(content);
  }

  /**
   * Convert a completed NodeFrame into a JSX element string.
   *
   * Format:
   *   Self-closing:  `<tag style={{...}} ...attrs />`
   *   With children: `<tag style={{...}} ...attrs>\n  children\n</tag>`
   *   Empty:         `<tag style={{...}} ...attrs />`
   */
  private _buildJSXElement(frame: NodeFrame): string {
    const { jsxTag, styles, attrs, classNames, textContent, selfClosing, lines } = frame;

    // Build the JSX attribute parts
    const parts: string[] = [];

    if (Object.keys(styles).length > 0) {
      const styleEntries = Object.entries(styles)
        .map(([k, v]) => `${k}: ${v}`)
        .join(", ");
      parts.push(`style={{ ${styleEntries} }}`);
    }

    if (classNames.length > 0) {
      parts.push(`className="${classNames.join(" ")}"`);
    }

    parts.push(...attrs);

    const attrStr = parts.length > 0 ? " " + parts.join(" ") : "";

    if (selfClosing) {
      return `<${jsxTag}${attrStr} />`;
    }

    // Determine children content
    let children: string;
    if (textContent !== undefined) {
      // Text nodes: inline content, no newlines needed
      children = textContent;
    } else {
      children = lines.join("\n");
    }

    if (!children) {
      // Empty element — use self-closing syntax
      return `<${jsxTag}${attrStr} />`;
    }

    if (textContent !== undefined) {
      // Inline text content: `<span style={{...}}>{title}</span>`
      return `<${jsxTag}${attrStr}>${children}</${jsxTag}>`;
    }

    // Block children: indent each child line by 2 spaces
    const indented = children.split("\n").map(l => "  " + l).join("\n");
    return `<${jsxTag}${attrStr}>\n${indented}\n</${jsxTag}>`;
  }

  // --------------------------------------------------------------------------
  // File Assembly
  // --------------------------------------------------------------------------

  /**
   * Assemble the complete .tsx file content.
   *
   * Called by `emit()` after all renderer methods have fired.
   * At this point, `this._stack[0].lines` contains the root JSX element.
   */
  private _buildFile(): string {
    const name = this._componentName;

    // -----------------------------------------------------------------------
    // Props interface lines and function parameter lines
    // -----------------------------------------------------------------------

    const propLines: string[] = [];
    const paramLines: string[] = [];

    for (const slot of this._slots) {
      const tsType = this._slotTypeToTS(slot.type);
      const optional = slot.defaultValue !== undefined ? "?" : "";
      const comment = slot.defaultValue !== undefined
        ? ` // default: ${this._defaultValueLiteral(slot.defaultValue)}`
        : "";
      propLines.push(`  ${slot.name}${optional}: ${tsType};${comment}`);

      if (slot.defaultValue !== undefined) {
        paramLines.push(`  ${slot.name} = ${this._defaultValueLiteral(slot.defaultValue)},`);
      } else {
        paramLines.push(`  ${slot.name},`);
      }
    }

    // -----------------------------------------------------------------------
    // Import statements
    // -----------------------------------------------------------------------

    const importLines: string[] = [];

    // Slot-type component imports (import type { ButtonProps } from "./Button.js")
    for (const compName of [...this._slotComponentImports].sort()) {
      importLines.push(`import type { ${compName}Props } from "./${compName}.js";`);
    }

    // Node-level component imports (import { Button } from "./Button.js")
    for (const compName of [...this._nodeComponentImports].sort()) {
      importLines.push(`import { ${compName} } from "./${compName}.js";`);
    }

    // -----------------------------------------------------------------------
    // Root JSX content
    // -----------------------------------------------------------------------

    const rootLines = (this._stack[0] as ComponentFrame).lines;
    const rootJSX = rootLines.join("\n");
    // Indent 4 spaces (2 for return body, 2 for JSX root)
    const indentedRoot = rootJSX.split("\n").map(l => "    " + l).join("\n");

    // -----------------------------------------------------------------------
    // Assemble the file
    // -----------------------------------------------------------------------

    const lines: string[] = [
      `// AUTO-GENERATED from ${name}.mosaic — do not edit`,
      `// Generated by mosaic-emit-react v1.0`,
      `// Source: ${name}.mosaic`,
      `//`,
      `// To modify this component, edit ${name}.mosaic and re-run the compiler.`,
      "",
      `import React from "react";`,
    ];

    if (this._needsTypeScaleCSS) {
      lines.push(`import "./mosaic-type-scale.css";`);
    }

    if (importLines.length > 0) {
      lines.push("");
      lines.push(...importLines);
    }

    lines.push("");
    lines.push(`interface ${name}Props {`);
    if (propLines.length > 0) {
      lines.push(...propLines);
    }
    lines.push("}");
    lines.push("");
    lines.push(`export function ${name}({`);
    if (paramLines.length > 0) {
      lines.push(...paramLines);
    }
    lines.push(`}: ${name}Props): JSX.Element {`);
    lines.push("  return (");
    lines.push(indentedRoot);
    lines.push("  );");
    lines.push("}");

    return lines.join("\n");
  }

  // --------------------------------------------------------------------------
  // Value Helpers
  // --------------------------------------------------------------------------

  /**
   * Convert a dimension ResolvedValue to a CSS string.
   * Returns null if the value is not a dimension.
   *
   * Both `dp` and `sp` map to CSS `px` — on the web, these are equivalent to
   * density-independent pixels. The `%` unit passes through unchanged.
   */
  private _dim(value: ResolvedValue): string | null {
    if (value.kind !== "dimension") return null;
    if (value.unit === "%") return `${value.value}%`;
    return `${value.value}px`; // dp and sp → px
  }

  /**
   * Convert a size value to a CSS string.
   * Handles fill ("100%"), wrap ("fit-content"), and dimensions.
   */
  private _sizeValue(value: ResolvedValue): string {
    if (value.kind === "string") {
      if (value.value === "fill") return "100%";
      if (value.value === "wrap") return "fit-content";
      return value.value;
    }
    return this._dim(value) ?? "auto";
  }

  /**
   * Format a color as a CSS `rgba()` string.
   *
   * Alpha is normalized from 0–255 to 0–1 for CSS. We always use `rgba()` —
   * never hex strings — for consistent output regardless of source format.
   *
   * @example
   *   _rgba(37, 99, 235, 255) → "rgba(37, 99, 235, 1)"
   *   _rgba(0, 0, 0, 128)     → "rgba(0, 0, 0, 0.502)"
   */
  private _rgba(r: number, g: number, b: number, a: number): string {
    // Round to 3 decimal places to avoid long floating-point strings
    const alpha = Math.round((a / 255) * 1000) / 1000;
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }

  /**
   * Convert a ResolvedValue to a JSX children expression.
   *
   * Used for `Text { content: ... }` where the content becomes JSX children.
   *   "hello world"  → 'hello world'   (literal text, no braces needed)
   *   @title         → '{title}'       (expression)
   */
  private _valueToJSX(value: ResolvedValue): string {
    switch (value.kind) {
      case "string": return value.value
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/\{/g, "&#123;")
        .replace(/\}/g, "&#125;");
      case "number": return String(value.value);
      case "bool":   return String(value.value);
      case "slot_ref":
        // Loop variables and component slots both emit the bare variable name
        // (since the function destructures its props).
        return `{${value.slotName}}`;
      default: return "";
    }
  }

  /**
   * Convert a ResolvedValue to a JSX attribute value expression.
   *
   * For JSX attributes, string literals use `="..."` syntax and expressions
   * use `={...}` syntax. This method returns the value portion:
   *   string "literal" → '"literal"'  (quotes included, for: attr="literal")
   *   slot ref @title  → '{title}'    (braces included, for: attr={title})
   */
  private _attrValue(value: ResolvedValue): string {
    switch (value.kind) {
      case "string":   return `"${value.value.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
      case "slot_ref": return `{${value.slotName}}`;
      default:         return '""';
    }
  }

  // --------------------------------------------------------------------------
  // Type System Helpers
  // --------------------------------------------------------------------------

  /**
   * Convert a MosaicType to its TypeScript prop type string.
   *
   * | Mosaic type      | TypeScript                            |
   * |------------------|---------------------------------------|
   * | text             | string                                |
   * | number           | number                                |
   * | bool             | boolean                               |
   * | image            | string                                |
   * | color            | string                                |
   * | node             | React.ReactNode                       |
   * | Button           | React.ReactElement<ButtonProps>       |
   * | list<text>       | string[]                              |
   * | list<number>     | number[]                              |
   * | list<node>       | React.ReactNode[]                     |
   * | list<Button>     | Array<React.ReactElement<ButtonProps>>|
   */
  private _slotTypeToTS(type: MosaicType): string {
    switch (type.kind) {
      case "text":      return "string";
      case "number":    return "number";
      case "bool":      return "boolean";
      case "image":     return "string";
      case "color":     return "string";
      case "node":      return "React.ReactNode";
      case "component": return `React.ReactElement<${type.name}Props>`;
      case "list": {
        const inner = type.elementType;
        switch (inner.kind) {
          case "text":      return "string[]";
          case "number":    return "number[]";
          case "bool":      return "boolean[]";
          case "image":     return "string[]";
          case "color":     return "string[]";
          case "node":      return "React.ReactNode[]";
          case "component": return `Array<React.ReactElement<${inner.name}Props>>`;
          default:          return "unknown[]";
        }
      }
      default: return "unknown";
    }
  }

  /**
   * Convert a MosaicValue (default value from slot declaration) to a TypeScript
   * literal string for use in function parameter defaults.
   */
  private _defaultValueLiteral(v: MosaicValue): string {
    switch (v.kind) {
      case "string": return `"${v.value.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
      case "number": return `${v.value}`;
      case "bool":   return `${v.value}`;
      default:       return "undefined";
    }
  }
}
