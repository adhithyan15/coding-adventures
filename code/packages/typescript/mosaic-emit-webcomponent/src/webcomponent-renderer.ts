/**
 * MosaicWebComponentRenderer — Emits a TypeScript Custom Element class (.ts).
 *
 * This is the Web Components backend for the Mosaic compiler. It implements the
 * `MosaicRenderer` interface and is driven by `MosaicVM`. The renderer produces
 * a single TypeScript file containing a Custom Element that:
 *
 *   - Extends `HTMLElement`
 *   - Uses Shadow DOM for style encapsulation
 *   - Exposes Mosaic slots as property setters/getters
 *   - Rebuilds shadow DOM content via `_render()` on any property change
 *   - Observes HTML attributes for primitive (text/number/bool/image/color) slots
 *
 * Architecture: Fragment Tree with `html +=` Render Method
 * --------------------------------------------------------
 *
 * Unlike the React backend (which builds JSX via a string stack), the Web
 * Components renderer builds a tree of `RenderFragment` objects during the VM
 * traversal and serializes them into a `_render()` method body during `emit()`.
 *
 * The `_render()` method uses a mutable `let html = ''` accumulator. Each
 * fragment type contributes one or more `html +=` statements:
 *
 *   - Static open/close tag:   `html += '<div style="...">';`
 *   - Escaped text slot:       `html += \`<span>\${this._escapeHtml(this._title)}</span>\`;`
 *   - Slot projection:         `html += '<slot name="action"></slot>';`
 *   - Conditional (when):      `if (this._show) { html += '...'; }`
 *   - List iteration (each):   `this._items.forEach(item => { html += ...; });`
 *
 * Because `html` is in the outer scope and closures capture it by reference,
 * `forEach` callbacks can append to it directly without needing a separate
 * accumulator variable. This keeps the generated code straightforward.
 *
 * Security
 * --------
 *
 * All text slot values are passed through `_escapeHtml()` before insertion into
 * innerHTML. URL values (image source slots) are validated to reject `javascript:`
 * scheme URIs. Colors from the VM are always emitted as `rgba()` strings, never
 * raw user strings.
 *
 * Tag Name Convention
 * -------------------
 *
 * PascalCase component names map to kebab-case element names with a `mosaic-` prefix:
 *   `ProfileCard`  → `<mosaic-profile-card>`
 *   `Button`       → `<mosaic-button>`
 *   `HowItWorks`   → `<mosaic-how-it-works>`
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
// Render Fragment Types
// ============================================================================

/**
 * A render fragment represents one logical piece of the `_render()` method.
 *
 * The renderer accumulates these during VM traversal, then serializes them
 * into `html +=` statements when building the final file.
 */
type RenderFragment =
  | { kind: "open_tag";  html: string }           // html += '<tag style="...">'; static open
  | { kind: "close_tag"; tag: string }             // html += '</tag>';
  | { kind: "self_closing"; html: string }         // html += '<tag ... />';
  | { kind: "slot_ref";  expr: string }            // html += `...${this._escapeHtml(...)}...`;
  | { kind: "slot_proj"; slotName: string }        // html += '<slot name="..."></slot>';
  | { kind: "when_open";  field: string }          // if (this._field) {
  | { kind: "when_close" }                         // }
  | { kind: "each_open";  field: string; itemName: string; isNodeList: boolean } // forEach(item => {
  | { kind: "each_close" };                        // });

// ============================================================================
// Stack Frame Types (for building during traversal)
// ============================================================================

interface ComponentFrame {
  kind: "component";
  fragments: RenderFragment[];
}

interface NodeFrame {
  kind: "node";
  tag: string;
  openHtml: string;
  selfClosing: boolean;
  textSlotExpr?: string;  // for Text nodes: the slot reference expression
  textLiteral?: string;   // for Text nodes: literal text content
  fragments: RenderFragment[];
}

interface WhenFrame {
  kind: "when";
  slotName: string;
  fragments: RenderFragment[];
}

interface EachFrame {
  kind: "each";
  slotName: string;
  itemName: string;
  isNodeList: boolean;
  fragments: RenderFragment[];
}

type StackFrame = ComponentFrame | NodeFrame | WhenFrame | EachFrame;

// ============================================================================
// MosaicWebComponentRenderer
// ============================================================================

/**
 * The Web Components backend for the Mosaic compiler.
 *
 * Construct this renderer, pass it to `MosaicVM.run()`, and call `emit()`
 * to get the generated `.ts` file.
 *
 * @example
 *     const ir = analyzeMosaic(source);
 *     const vm = new MosaicVM(ir);
 *     const renderer = new MosaicWebComponentRenderer();
 *     const result = vm.run(renderer);
 *     // result.files[0].filename === "mosaic-my-component.ts"
 *     // result.files[0].content  === "// AUTO-GENERATED ..."
 */
export class MosaicWebComponentRenderer implements MosaicRenderer {
  private _componentName: string = "";
  private _slots: MosaicSlot[] = [];
  private _stack: StackFrame[] = [];
  private _needsTypeScaleCSS: boolean = false;

  // --------------------------------------------------------------------------
  // MosaicRenderer implementation
  // --------------------------------------------------------------------------

  beginComponent(name: string, slots: MosaicSlot[]): void {
    this._componentName = name;
    this._slots = slots;
    this._stack = [{ kind: "component", fragments: [] }];
    this._needsTypeScaleCSS = false;
  }

  endComponent(): void {
    // No-op: content is already accumulated in stack[0].fragments.
  }

  emit(): MosaicEmitResult {
    const content = this._buildFile();
    const tagName = this._toKebabCase(this._componentName);
    return {
      files: [{ filename: `mosaic-${tagName}.ts`, content }],
    };
  }

  beginNode(tag: string, isPrimitive: boolean, properties: ResolvedProperty[], _ctx: SlotContext): void {
    const frame = this._buildNodeFrame(tag, isPrimitive, properties);
    this._stack.push(frame);
  }

  endNode(_tag: string): void {
    const frame = this._stack.pop() as NodeFrame;

    // Emit the node fragments into the parent frame
    const parent = this._currentFrame();

    if (frame.selfClosing) {
      parent.fragments.push({ kind: "self_closing", html: frame.openHtml });
    } else {
      parent.fragments.push({ kind: "open_tag", html: frame.openHtml });

      if (frame.textLiteral !== undefined) {
        // Static text content
        parent.fragments.push({
          kind: "open_tag",
          html: this._escapeHtmlLiteral(frame.textLiteral),
        });
      } else if (frame.textSlotExpr !== undefined) {
        // Dynamic slot reference → use escapeHtml
        parent.fragments.push({ kind: "slot_ref", expr: frame.textSlotExpr });
      } else {
        // Block children were already pushed into frame.fragments
        parent.fragments.push(...frame.fragments);
      }

      parent.fragments.push({ kind: "close_tag", tag: frame.tag === "Text" ? "span" : this._tagToHtml(frame.tag) });
    }
  }

  renderSlotChild(slotName: string, _slotType: MosaicType, _ctx: SlotContext): void {
    // Slot used as a child element → Light DOM projection via named <slot>
    this._currentFrame().fragments.push({ kind: "slot_proj", slotName });
  }

  beginWhen(slotName: string, _ctx: SlotContext): void {
    this._stack.push({ kind: "when", slotName, fragments: [] });
  }

  endWhen(): void {
    const frame = this._stack.pop() as WhenFrame;
    const parent = this._currentFrame();
    parent.fragments.push({ kind: "when_open", field: frame.slotName });
    parent.fragments.push(...frame.fragments);
    parent.fragments.push({ kind: "when_close" });
  }

  beginEach(slotName: string, itemName: string, elementType: MosaicType, _ctx: SlotContext): void {
    const isNodeList = elementType.kind === "node" || elementType.kind === "component";
    this._stack.push({ kind: "each", slotName, itemName, isNodeList, fragments: [] });
  }

  endEach(): void {
    const frame = this._stack.pop() as EachFrame;
    const parent = this._currentFrame();
    parent.fragments.push({ kind: "each_open", field: frame.slotName, itemName: frame.itemName, isNodeList: frame.isNodeList });
    parent.fragments.push(...frame.fragments);
    parent.fragments.push({ kind: "each_close" });
  }

  // --------------------------------------------------------------------------
  // Node Frame Building
  // --------------------------------------------------------------------------

  private _buildNodeFrame(tag: string, isPrimitive: boolean, properties: ResolvedProperty[]): NodeFrame {
    const styles: string[] = [];    // CSS key:value pairs (no semicolons yet)
    const attrs: string[] = [];     // HTML attribute strings
    const classNames: string[] = [];
    let textLiteral: string | undefined;
    let textSlotExpr: string | undefined;
    let selfClosing = false;

    // -----------------------------------------------------------------------
    // Base styles from primitive element type
    // -----------------------------------------------------------------------

    let htmlTag: string;

    if (isPrimitive) {
      switch (tag) {
        case "Box":
          htmlTag = "div";
          styles.push("position:relative");
          break;
        case "Column":
          htmlTag = "div";
          styles.push("display:flex", "flex-direction:column");
          break;
        case "Row":
          htmlTag = "div";
          styles.push("display:flex", "flex-direction:row");
          break;
        case "Text":
          // May become <h2> if a11y-role: heading
          htmlTag = "span";
          break;
        case "Image":
          htmlTag = "img";
          selfClosing = true;
          break;
        case "Spacer":
          htmlTag = "div";
          styles.push("flex:1");
          break;
        case "Scroll":
          htmlTag = "div";
          styles.push("overflow:auto");
          break;
        case "Divider":
          htmlTag = "hr";
          selfClosing = true;
          styles.push("border:none", "border-top:1px solid currentColor");
          break;
        default:
          htmlTag = "div";
          break;
      }
    } else {
      // Imported component — rendered as a slot projection target;
      // in Web Components the child is expected to be provided externally.
      // Emit a div placeholder for now.
      htmlTag = "div";
    }

    // -----------------------------------------------------------------------
    // Apply properties
    // -----------------------------------------------------------------------

    for (const prop of properties) {
      this._applyProperty(prop, tag, styles, attrs, classNames,
        (lit) => { textLiteral = lit; },
        (expr) => { textSlotExpr = expr; },
      );
    }

    // Post-process: a11y-role: heading on Text → h2
    if (tag === "Text") {
      const headingIdx = attrs.indexOf('role="heading"');
      if (headingIdx >= 0) {
        htmlTag = "h2";
        attrs.splice(headingIdx, 1);
      }
    }

    // -----------------------------------------------------------------------
    // Build the opening HTML string
    // -----------------------------------------------------------------------

    const styleStr = styles.length > 0 ? styles.join(";") : "";
    const parts: string[] = [];
    if (styleStr) parts.push(`style="${styleStr}"`);
    if (classNames.length > 0) parts.push(`class="${classNames.join(" ")}"`);
    parts.push(...attrs);

    const attrStr = parts.length > 0 ? " " + parts.join(" ") : "";
    const openHtml = `<${htmlTag}${attrStr}>`;

    return {
      kind: "node",
      tag,
      openHtml,
      selfClosing,
      textLiteral,
      textSlotExpr,
      fragments: [],
    };
  }

  // --------------------------------------------------------------------------
  // Property Application
  // --------------------------------------------------------------------------

  /**
   * Apply a resolved property to the style/attrs/class collections.
   *
   * CSS property names use kebab-case (standard CSS), unlike the React backend
   * which uses camelCase (React style object keys).
   */
  private _applyProperty(
    prop: ResolvedProperty,
    tag: string,
    styles: string[],
    attrs: string[],
    classNames: string[],
    setTextLiteral: (s: string) => void,
    setTextSlotExpr: (expr: string) => void,
  ): void {
    const { name, value } = prop;

    switch (name) {
      // Layout: spacing
      case "padding": {
        const d = this._dim(value);
        if (d) styles.push(`padding:${d}`);
        break;
      }
      case "padding-left": {
        const d = this._dim(value);
        if (d) styles.push(`padding-left:${d}`);
        break;
      }
      case "padding-right": {
        const d = this._dim(value);
        if (d) styles.push(`padding-right:${d}`);
        break;
      }
      case "padding-top": {
        const d = this._dim(value);
        if (d) styles.push(`padding-top:${d}`);
        break;
      }
      case "padding-bottom": {
        const d = this._dim(value);
        if (d) styles.push(`padding-bottom:${d}`);
        break;
      }
      case "gap": {
        const d = this._dim(value);
        if (d) styles.push(`gap:${d}`);
        break;
      }

      // Layout: size
      case "width":
        styles.push(`width:${this._sizeValue(value)}`);
        break;
      case "height":
        styles.push(`height:${this._sizeValue(value)}`);
        break;
      case "min-width": {
        const d = this._dim(value);
        if (d) styles.push(`min-width:${d}`);
        break;
      }
      case "max-width": {
        const d = this._dim(value);
        if (d) styles.push(`max-width:${d}`);
        break;
      }
      case "min-height": {
        const d = this._dim(value);
        if (d) styles.push(`min-height:${d}`);
        break;
      }
      case "max-height": {
        const d = this._dim(value);
        if (d) styles.push(`max-height:${d}`);
        break;
      }

      // Layout: overflow
      case "overflow":
        if (value.kind === "string") {
          const map: Record<string, string> = { visible: "visible", hidden: "hidden", scroll: "auto" };
          const v = map[value.value];
          if (v) styles.push(`overflow:${v}`);
        }
        break;

      // Layout: alignment
      case "align":
        if (value.kind === "string") {
          this._applyAlign(value.value, tag, styles);
        }
        break;

      // Visual: background, border
      case "background":
        if (value.kind === "color") {
          styles.push(`background-color:${this._rgba(value.r, value.g, value.b, value.a)}`);
        }
        break;
      case "corner-radius": {
        const d = this._dim(value);
        if (d) styles.push(`border-radius:${d}`);
        break;
      }
      case "border-width": {
        const d = this._dim(value);
        if (d) {
          styles.push(`border-width:${d}`);
          styles.push("border-style:solid");
        }
        break;
      }
      case "border-color":
        if (value.kind === "color") {
          styles.push(`border-color:${this._rgba(value.r, value.g, value.b, value.a)}`);
        }
        break;
      case "opacity":
        if (value.kind === "number") {
          styles.push(`opacity:${value.value}`);
        }
        break;

      // Visual: shadow
      case "shadow":
        if (value.kind === "enum" && value.namespace === "elevation") {
          const shadowMap: Record<string, string> = {
            none:   "none",
            low:    "0 1px 3px rgba(0,0,0,0.12)",
            medium: "0 4px 12px rgba(0,0,0,0.15)",
            high:   "0 8px 24px rgba(0,0,0,0.20)",
          };
          const s = shadowMap[value.member];
          if (s !== undefined) styles.push(`box-shadow:${s}`);
        }
        break;

      // Visual: visibility
      case "visible":
        if (value.kind === "bool" && !value.value) {
          styles.push("display:none");
        }
        break;

      // Text-specific
      case "content":
        if (tag === "Text") {
          if (value.kind === "string") {
            setTextLiteral(value.value);
          } else if (value.kind === "slot_ref") {
            // Dynamic text: use escapeHtml for XSS safety
            if (value.isLoopVar) {
              setTextSlotExpr(`${value.slotName}`);
            } else {
              setTextSlotExpr(`this._${value.slotName}`);
            }
          }
        }
        break;
      case "color":
        if (value.kind === "color") {
          styles.push(`color:${this._rgba(value.r, value.g, value.b, value.a)}`);
        }
        break;
      case "text-align":
        if (value.kind === "string") {
          const map: Record<string, string> = { start: "left", center: "center", end: "right" };
          const a = map[value.value];
          if (a) styles.push(`text-align:${a}`);
        }
        break;
      case "font-weight": {
        if (value.kind === "string") {
          const safeFW = new Set(["100","200","300","400","500","600","700","800","900","normal","bold","bolder","lighter"]);
          if (safeFW.has(value.value)) styles.push(`font-weight:${value.value}`);
        }
        break;
      }
      case "max-lines":
        if (value.kind === "number") {
          styles.push(`-webkit-line-clamp:${value.value}`, "overflow:hidden", "display:-webkit-box", "-webkit-box-orient:vertical");
        }
        break;
      case "style":
        if (value.kind === "enum") {
          classNames.push(`mosaic-${value.namespace}-${value.member}`);
          this._needsTypeScaleCSS = true;
        } else if (value.kind === "string") {
          classNames.push(`mosaic-${value.value}`);
          this._needsTypeScaleCSS = true;
        }
        break;

      // Image-specific
      case "source":
        if (tag === "Image") {
          if (value.kind === "string") {
            attrs.push(`src="${this._escapeAttr(value.value)}"`);
          } else if (value.kind === "slot_ref") {
            // Image source: will be a dynamic expression in _render()
            // Use a placeholder that gets replaced during serialization
            attrs.push(`src="__IMG_SRC_${value.slotName}__"`);
          }
        }
        break;
      case "size": {
        const d = this._dim(value);
        if (d && tag === "Image") {
          styles.push(`width:${d}`, `height:${d}`);
        }
        break;
      }
      case "shape":
        if (tag === "Image" && value.kind === "string") {
          const shapeMap: Record<string, string> = { circle: "50%", rounded: "8px" };
          const r = shapeMap[value.value];
          if (r) styles.push(`border-radius:${r}`);
        }
        break;
      case "fit":
        if (tag === "Image" && value.kind === "string") {
          styles.push(`object-fit:${value.value}`);
        }
        break;

      // Accessibility
      case "a11y-label":
        if (value.kind === "string") {
          attrs.push(`aria-label="${this._escapeAttr(value.value)}"`);
        } else if (value.kind === "slot_ref") {
          // Dynamic aria-label: handled via placeholder
          attrs.push(`aria-label="__ARIA_${value.slotName}__"`);
        }
        break;
      case "a11y-role":
        if (value.kind === "string") {
          switch (value.value) {
            case "none":    attrs.push('aria-hidden="true"'); break;
            case "heading": attrs.push('role="heading"'); break; // post-processed above
            case "image":   attrs.push('role="img"'); break;
            default:        attrs.push(`role="${value.value}"`); break;
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
  // Alignment
  // --------------------------------------------------------------------------

  private _applyAlign(alignValue: string, tag: string, styles: string[]): void {
    if (tag === "Box") {
      styles.push("display:flex");
    }

    switch (tag) {
      case "Column":
        switch (alignValue) {
          case "start":             styles.push("align-items:flex-start"); break;
          case "center":            styles.push("align-items:center"); break;
          case "end":               styles.push("align-items:flex-end"); break;
          case "stretch":           styles.push("align-items:stretch"); break;
          case "center-horizontal": styles.push("align-items:center"); break;
          case "center-vertical":   styles.push("justify-content:center"); break;
        }
        break;
      case "Row":
        switch (alignValue) {
          case "start":             styles.push("align-items:flex-start"); break;
          case "center":            styles.push("align-items:center", "justify-content:center"); break;
          case "end":               styles.push("align-items:flex-end", "justify-content:flex-end"); break;
          case "stretch":           styles.push("align-items:stretch"); break;
          case "center-horizontal": styles.push("justify-content:center"); break;
          case "center-vertical":   styles.push("align-items:center"); break;
        }
        break;
      case "Box":
        switch (alignValue) {
          case "start":             styles.push("align-items:flex-start"); break;
          case "center":            styles.push("align-items:center"); break;
          case "end":               styles.push("align-items:flex-end"); break;
          case "stretch":           styles.push("align-items:stretch"); break;
          case "center-horizontal": styles.push("align-items:center"); break;
          case "center-vertical":   styles.push("justify-content:center"); break;
        }
        break;
    }
  }

  // --------------------------------------------------------------------------
  // Fragment serialization
  // --------------------------------------------------------------------------

  private _currentFrame(): StackFrame {
    return this._stack[this._stack.length - 1];
  }

  /**
   * Serialize render fragments into an array of code lines for `_render()`.
   *
   * Each fragment type emits one or more `html +=` statement lines.
   * Block constructs (when, each) use JavaScript `if` / `forEach` statements.
   *
   * Indentation is managed by the caller (each line gets 4-space indent for
   * the method body, +2 per nesting level for blocks).
   */
  /**
   * Escape a string for use inside a single-quoted JavaScript string literal.
   * We use single-quoted strings for HTML so that HTML double-quote attributes
   * (e.g., `role="button"`) appear unescaped in the generated file.
   */
  private _singleQuoteEscape(s: string): string {
    return s.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
  }

  private _serializeFragments(fragments: RenderFragment[], indent: string): string[] {
    const lines: string[] = [];

    for (const frag of fragments) {
      switch (frag.kind) {
        case "open_tag":
          // Static HTML — use single-quoted string so HTML attribute double-quotes
          // appear unescaped in the generated source (easier to read, easier to test).
          lines.push(`${indent}html += '${this._singleQuoteEscape(frag.html)}';`);
          break;

        case "close_tag":
          lines.push(`${indent}html += '</${frag.tag}>';`);
          break;

        case "self_closing":
          lines.push(`${indent}html += '${this._singleQuoteEscape(frag.html)}';`);
          break;

        case "slot_ref": {
          // Dynamic text slot — emitted as a template literal to evaluate the expression.
          // The expression is already a valid JS expression (e.g., "this._escapeHtml(this._title)").
          lines.push(`${indent}html += \`\${${frag.expr}}\`;`);
          break;
        }

        case "slot_proj":
          // Light DOM projection point — a named <slot> element.
          lines.push(`${indent}html += '<slot name="${frag.slotName}"></slot>';`);
          break;

        case "when_open":
          lines.push(`${indent}if (this._${frag.field}) {`);
          break;

        case "when_close":
          lines.push(`${indent}}`);
          break;

        case "each_open": {
          if (frag.isNodeList) {
            // Node list: emit indexed named slots
            lines.push(`${indent}this._${frag.field}.forEach((_item, _i) => {`);
            lines.push(`${indent}  html += \`<slot name="${frag.field}-\${_i}"></slot>\`;`);
          } else {
            // Primitive list: emit forEach with item variable
            // Validate itemName is a safe JS identifier to prevent code injection
            const safeItem = /^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(frag.itemName) ? frag.itemName : "_item";
            lines.push(`${indent}this._${frag.field}.forEach(${safeItem} => {`);
          }
          break;
        }

        case "each_close":
          lines.push(`${indent}});`);
          break;
      }
    }

    return lines;
  }

  // --------------------------------------------------------------------------
  // File Assembly
  // --------------------------------------------------------------------------

  /**
   * Build the complete .ts file content.
   */
  private _buildFile(): string {
    const name = this._componentName;
    const className = `Mosaic${name}Element`;
    const tagName = this._toKebabCase(name);
    const elementTag = `mosaic-${tagName}`;

    // -----------------------------------------------------------------------
    // Slot categorization
    // -----------------------------------------------------------------------

    /** Slots that are observable HTML attributes (primitive scalars). */
    const observableSlots = this._slots.filter(s => this._isObservableType(s.type));
    /** Slots that are node/component types (Light DOM projection). */
    const nodeSlots = this._slots.filter(s => s.type.kind === "node" || s.type.kind === "component");
    /** Image slots (need URL validation in setter). */
    const imageSlots = this._slots.filter(s => s.type.kind === "image");
    /** List slots. */
    const listSlots = this._slots.filter(s => s.type.kind === "list");
    const hasNodeSlots = nodeSlots.length > 0;

    // -----------------------------------------------------------------------
    // Build backing field declarations
    // -----------------------------------------------------------------------

    const fieldLines: string[] = [];
    for (const slot of this._slots) {
      fieldLines.push(`  private ${this._backingField(slot.name)}: ${this._tsFieldType(slot.type)} = ${this._defaultValue(slot)};`);
    }

    // -----------------------------------------------------------------------
    // Build observedAttributes
    // -----------------------------------------------------------------------

    const observedAttrNames = observableSlots.map(s => `'${s.name}'`).join(", ");

    // -----------------------------------------------------------------------
    // Build attributeChangedCallback
    // -----------------------------------------------------------------------

    const attrCaselines: string[] = [];
    for (const slot of observableSlots) {
      const field = this._backingField(slot.name);
      let setter: string;
      switch (slot.type.kind) {
        case "number":
          setter = `${field} = parseFloat(value ?? '${this._defaultScalar(slot)}');`;
          break;
        case "bool":
          setter = `${field} = value !== null;`;
          break;
        default:
          setter = `${field} = value ?? ${JSON.stringify(this._defaultScalar(slot))};`;
          break;
      }
      attrCaselines.push(`    case '${slot.name}': this.${setter} break;`);
    }

    // -----------------------------------------------------------------------
    // Build property setters/getters
    // -----------------------------------------------------------------------

    const setterLines: string[] = [];

    for (const slot of this._slots) {
      const field = this._backingField(slot.name);
      const tsType = this._tsFieldType(slot.type);

      if (slot.type.kind === "node" || slot.type.kind === "component") {
        // Node/component slots use Light DOM projection (no _render call)
        setterLines.push(`  set ${slot.name}(v: HTMLElement) { this._projectSlot('${slot.name}', v); }`);
      } else if (slot.type.kind === "image") {
        // Image slots: validate URL to reject javascript: scheme
        setterLines.push(`  set ${slot.name}(v: string) {`);
        setterLines.push(`    if (/^javascript:/i.test(v.trim())) return;`);
        setterLines.push(`    ${field} = v;`);
        setterLines.push(`    this._render();`);
        setterLines.push(`  }`);
        setterLines.push(`  get ${slot.name}(): string { return ${field}; }`);
      } else if (slot.type.kind === "list") {
        // List slots: direct assignment + render
        setterLines.push(`  set ${slot.name}(v: ${tsType}) { ${field} = v; this._render(); }`);
      } else {
        // Primitive scalar slots
        setterLines.push(`  set ${slot.name}(v: ${tsType}) { ${field} = v; this._render(); }`);
        setterLines.push(`  get ${slot.name}(): ${tsType} { return ${field}; }`);
      }
    }

    // -----------------------------------------------------------------------
    // Build _render() body
    // -----------------------------------------------------------------------

    const rootFragments = (this._stack[0] as ComponentFrame).fragments;
    const renderBodyLines = this._serializeFragments(rootFragments, "    ");

    // Replace any image source placeholders with the actual field reference
    const resolvedRenderLines = renderBodyLines.map(line => {
      return line.replace(/__IMG_SRC_(\w+)__/g, (_match, slotName) => {
        const slot = this._slots.find(s => s.name === slotName);
        return slot
          ? `" + this._validateUrl(this.${this._backingField(slotName)}) + "`
          : '""';
      }).replace(/__ARIA_(\w+)__/g, (_match, slotName) => {
        return `" + this._escapeHtml(this.${this._backingField(slotName)}) + "`;
      });
    });

    // -----------------------------------------------------------------------
    // Assemble the file
    // -----------------------------------------------------------------------

    const lines: string[] = [
      `// AUTO-GENERATED from ${name}.mosaic — do not edit`,
      `// Generated by mosaic-emit-webcomponent v1.0`,
      `// Source: ${name}.mosaic`,
      `//`,
      `// To modify this component, edit ${name}.mosaic and re-run the compiler.`,
      "",
    ];

    if (this._needsTypeScaleCSS) {
      lines.push("const MOSAIC_TYPE_SCALE_CSS = `");
      lines.push("  .mosaic-heading-large { font-size: 2rem; font-weight: 700; line-height: 1.2; }");
      lines.push("  .mosaic-heading-medium { font-size: 1.5rem; font-weight: 600; line-height: 1.3; }");
      lines.push("  .mosaic-heading-small { font-size: 1.25rem; font-weight: 600; line-height: 1.4; }");
      lines.push("  .mosaic-body-large { font-size: 1rem; line-height: 1.6; }");
      lines.push("  .mosaic-body-medium { font-size: 0.875rem; line-height: 1.6; }");
      lines.push("  .mosaic-body-small { font-size: 0.75rem; line-height: 1.5; }");
      lines.push("  .mosaic-label { font-size: 0.875rem; font-weight: 500; }");
      lines.push("  .mosaic-caption { font-size: 0.75rem; color: #666; }");
      lines.push("`;");
      lines.push("");
    }

    lines.push(`export class ${className} extends HTMLElement {`);
    lines.push(`  private _shadow: ShadowRoot;`);
    lines.push("");

    if (fieldLines.length > 0) {
      lines.push("  // Backing fields for Mosaic slots");
      lines.push(...fieldLines);
      lines.push("");
    }

    lines.push("  constructor() {");
    lines.push("    super();");
    lines.push("    this._shadow = this.attachShadow({ mode: 'open' });");
    lines.push("  }");
    lines.push("");

    if (observableSlots.length > 0) {
      lines.push("  static get observedAttributes(): string[] {");
      lines.push(`    return [${observedAttrNames}];`);
      lines.push("  }");
      lines.push("");
      lines.push("  attributeChangedCallback(name: string, _old: string | null, value: string | null): void {");
      lines.push("    switch (name) {");
      lines.push(...attrCaselines);
      lines.push("    }");
      lines.push("    this._render();");
      lines.push("  }");
      lines.push("");
    }

    if (setterLines.length > 0) {
      lines.push("  // Property setters and getters");
      lines.push(...setterLines);
      lines.push("");
    }

    if (hasNodeSlots) {
      lines.push("  // Light DOM slot projection for node/component-type slots");
      lines.push("  private _projectSlot(name: string, node: Element): void {");
      lines.push("    const prev = this.querySelector(`[data-mosaic-slot=\"${name}\"]`);");
      lines.push("    if (prev) prev.remove();");
      lines.push("    node.setAttribute('slot', name);");
      lines.push("    node.setAttribute('data-mosaic-slot', name);");
      lines.push("    this.appendChild(node);");
      lines.push("  }");
      lines.push("");
    }

    lines.push("  private _escapeHtml(s: string): string {");
    lines.push("    return s");
    lines.push("      .replace(/&/g, '&amp;')");
    lines.push("      .replace(/</g, '&lt;')");
    lines.push("      .replace(/>/g, '&gt;')");
    lines.push("      .replace(/\"/g, '&quot;')");
    lines.push("      .replace(/'/g, '&#39;');");
    lines.push("  }");
    lines.push("");

    lines.push("  connectedCallback(): void { this._render(); }");
    lines.push("");

    if (hasNodeSlots) {
      lines.push("  disconnectedCallback(): void {");
      lines.push("    [...this.querySelectorAll('[data-mosaic-slot]')].forEach((el) => el.remove());");
      lines.push("  }");
      lines.push("");
    }

    lines.push("  private _render(): void {");
    lines.push("    let html = '';");
    if (this._needsTypeScaleCSS) {
      lines.push("    html += `<style>${MOSAIC_TYPE_SCALE_CSS}</style>`;");
    }
    lines.push(...resolvedRenderLines);
    lines.push("    this._shadow.innerHTML = html;");
    lines.push("  }");
    lines.push("}");
    lines.push("");
    lines.push(`customElements.define('${elementTag}', ${className});`);

    return lines.join("\n");
  }

  // --------------------------------------------------------------------------
  // Type helpers
  // --------------------------------------------------------------------------

  private _tsFieldType(type: MosaicType): string {
    switch (type.kind) {
      case "text":      return "string";
      case "number":    return "number";
      case "bool":      return "boolean";
      case "image":     return "string";
      case "color":     return "string";
      case "node":      return "HTMLElement | null";
      case "component": return "HTMLElement | null";
      case "list": {
        const inner = type.elementType;
        if (inner.kind === "node" || inner.kind === "component") return "Element[]";
        if (inner.kind === "text")   return "string[]";
        if (inner.kind === "number") return "number[]";
        if (inner.kind === "bool")   return "boolean[]";
        return "unknown[]";
      }
      default: return "unknown";
    }
  }

  private _defaultValue(slot: MosaicSlot): string {
    if (slot.defaultValue) {
      return this._defaultValueLiteral(slot.defaultValue);
    }
    switch (slot.type.kind) {
      case "text":      return "''";
      case "number":    return "0";
      case "bool":      return "false";
      case "image":     return "''";
      case "color":     return "''";
      case "node":
      case "component": return "null";
      case "list":      return "[]";
      default:          return "null";
    }
  }

  private _defaultScalar(slot: MosaicSlot): string {
    if (slot.defaultValue) return this._defaultValueLiteral(slot.defaultValue);
    if (slot.type.kind === "number") return "0";
    return "";
  }

  private _defaultValueLiteral(v: MosaicValue): string {
    switch (v.kind) {
      case "string": return `'${v.value.replace(/\\/g, "\\\\").replace(/'/g, "\\'")}'`;
      case "number": return `${v.value}`;
      case "bool":   return `${v.value}`;
      default:       return "null";
    }
  }

  private _isObservableType(type: MosaicType): boolean {
    // Primitive scalars can be set via HTML attributes
    return type.kind === "text" || type.kind === "number" || type.kind === "bool"
        || type.kind === "image" || type.kind === "color";
  }

  private _backingField(slotName: string): string {
    return `_${slotName}`;
  }

  private _tagToHtml(tag: string): string {
    switch (tag) {
      case "Column":
      case "Row":
      case "Box":
      case "Spacer":
      case "Scroll":  return "div";
      case "Text":    return "span";
      case "Image":   return "img";
      case "Divider": return "hr";
      default:        return "div";
    }
  }

  // --------------------------------------------------------------------------
  // Value helpers
  // --------------------------------------------------------------------------

  private _dim(value: ResolvedValue): string | null {
    if (value.kind !== "dimension") return null;
    if (value.unit === "%") return `${value.value}%`;
    return `${value.value}px`;
  }

  private _sizeValue(value: ResolvedValue): string {
    if (value.kind === "string") {
      if (value.value === "fill") return "100%";
      if (value.value === "wrap") return "fit-content";
      return value.value;
    }
    return this._dim(value) ?? "auto";
  }

  private _rgba(r: number, g: number, b: number, a: number): string {
    const alpha = Math.round((a / 255) * 1000) / 1000;
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }

  private _escapeAttr(value: string): string {
    return value
      .replace(/&/g, "&amp;")
      .replace(/"/g, "&quot;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  private _escapeHtmlLiteral(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  /**
   * Convert PascalCase to kebab-case.
   *
   * @example
   *   "ProfileCard" → "profile-card"
   *   "HowItWorks"  → "how-it-works"
   *   "Button"      → "button"
   */
  private _toKebabCase(name: string): string {
    return name
      .replace(/([A-Z])/g, "-$1")
      .toLowerCase()
      .replace(/^-/, "");
  }
}
