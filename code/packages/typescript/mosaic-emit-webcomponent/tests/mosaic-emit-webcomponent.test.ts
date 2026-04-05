/**
 * Tests for MosaicWebComponentRenderer.
 *
 * Each test runs the full pipeline: analyzeMosaic → MosaicVM → MosaicWebComponentRenderer
 * and asserts on the generated Custom Element TypeScript source.
 *
 * Test Categories
 * ---------------
 *
 *   1.  File structure — header, class, customElements.define
 *   2.  Tag name — PascalCase → mosaic-kebab-case
 *   3.  Class naming — Mosaic{Name}Element
 *   4.  Slot backing fields
 *   5.  Observed attributes
 *   6.  Property setters/getters
 *   7.  connectedCallback / disconnectedCallback
 *   8.  Layout styles (CSS property names)
 *   9.  Visual styles
 *  10.  Text content
 *  11.  Image properties
 *  12.  Alignment
 *  13.  Accessibility
 *  14.  Slot child projection
 *  15.  When blocks
 *  16.  Each blocks (list<text>)
 *  17.  Type scale CSS
 *  18.  Security: _escapeHtml
 */

import { describe, it, expect } from "vitest";
import { MosaicWebComponentRenderer } from "../src/webcomponent-renderer.js";
import { MosaicVM } from "@coding-adventures/mosaic-vm";
import { analyzeMosaic } from "@coding-adventures/mosaic-analyzer";

function compile(source: string): string {
  const ir = analyzeMosaic(source);
  const vm = new MosaicVM(ir);
  const renderer = new MosaicWebComponentRenderer();
  const result = vm.run(renderer);
  return result.files[0].content;
}

function filename(source: string): string {
  const ir = analyzeMosaic(source);
  const vm = new MosaicVM(ir);
  const renderer = new MosaicWebComponentRenderer();
  const result = vm.run(renderer);
  return result.files[0].filename;
}

// ============================================================================
// 1. File Structure
// ============================================================================

describe("file structure", () => {
  it("emits a single .ts file", () => {
    const ir = analyzeMosaic("component MyCard { Row {} }");
    const vm = new MosaicVM(ir);
    const result = vm.run(new MosaicWebComponentRenderer());
    expect(result.files).toHaveLength(1);
    expect(result.files[0].filename).toMatch(/\.ts$/);
  });

  it("starts with auto-generated header", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("// AUTO-GENERATED from X.mosaic — do not edit");
  });

  it("exports the class", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("export class");
  });

  it("extends HTMLElement", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("extends HTMLElement");
  });

  it("calls customElements.define", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("customElements.define(");
  });

  it("has a _render() method", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("private _render(): void {");
  });

  it("has a constructor with attachShadow", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("constructor()");
    expect(code).toContain("attachShadow(");
  });

  it("has connectedCallback", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("connectedCallback(): void");
  });
});

// ============================================================================
// 2. Tag Name Conversion
// ============================================================================

describe("tag name conversion", () => {
  it("ProfileCard → mosaic-profile-card", () => {
    expect(filename("component ProfileCard { Row {} }")).toBe("mosaic-profile-card.ts");
  });

  it("Button → mosaic-button", () => {
    expect(filename("component Button { Row {} }")).toBe("mosaic-button.ts");
  });

  it("HowItWorks → mosaic-how-it-works", () => {
    expect(filename("component HowItWorks { Row {} }")).toBe("mosaic-how-it-works.ts");
  });

  it("customElements.define uses the kebab element name", () => {
    const code = compile("component ProfileCard { Row {} }");
    expect(code).toContain("customElements.define('mosaic-profile-card'");
  });
});

// ============================================================================
// 3. Class Naming
// ============================================================================

describe("class naming", () => {
  it("class is Mosaic{Name}Element", () => {
    const code = compile("component ProfileCard { Row {} }");
    expect(code).toContain("class MosaicProfileCardElement extends HTMLElement");
  });

  it("Button → MosaicButtonElement", () => {
    const code = compile("component Button { Row {} }");
    expect(code).toContain("class MosaicButtonElement extends HTMLElement");
  });
});

// ============================================================================
// 4. Slot Backing Fields
// ============================================================================

describe("slot backing fields", () => {
  it("text slot → private _field: string = ''", () => {
    const code = compile("component X { slot title: text; Row {} }");
    expect(code).toContain("private _title: string = '';");
  });

  it("number slot → private _field: number = 0", () => {
    const code = compile("component X { slot count: number; Row {} }");
    expect(code).toContain("private _count: number = 0;");
  });

  it("bool slot → private _field: boolean = false", () => {
    const code = compile("component X { slot visible: bool; Row {} }");
    expect(code).toContain("private _visible: boolean = false;");
  });

  it("number slot with default → correct default value", () => {
    const code = compile("component X { slot count: number = 42; Row {} }");
    expect(code).toContain("private _count: number = 42;");
  });

  it("bool slot with default true", () => {
    const code = compile("component X { slot show: bool = true; Row {} }");
    expect(code).toContain("private _show: boolean = true;");
  });

  it("node slot → private _field: HTMLElement | null = null", () => {
    const code = compile("component X { slot action: node; Row {} }");
    expect(code).toContain("private _action: HTMLElement | null = null;");
  });

  it("list<text> slot → private _field: string[] = []", () => {
    const code = compile("component X { slot items: list<text>; Row {} }");
    expect(code).toContain("private _items: string[] = [];");
  });

  it("list<number> slot → number[]", () => {
    const code = compile("component X { slot vals: list<number>; Row {} }");
    expect(code).toContain("private _vals: number[] = [];");
  });
});

// ============================================================================
// 5. Observed Attributes
// ============================================================================

describe("observed attributes", () => {
  it("text slot is observable", () => {
    const code = compile("component X { slot title: text; Row {} }");
    expect(code).toContain("observedAttributes");
    expect(code).toContain("'title'");
  });

  it("number slot is observable", () => {
    const code = compile("component X { slot count: number; Row {} }");
    expect(code).toContain("'count'");
  });

  it("bool slot is observable", () => {
    const code = compile("component X { slot visible: bool; Row {} }");
    expect(code).toContain("'visible'");
  });

  it("node slot is NOT in observedAttributes", () => {
    const code = compile("component X { slot action: node; Row {} }");
    // node slots can't be set via HTML attributes
    const obsMatch = code.match(/observedAttributes.*?\[([^\]]*)\]/s);
    if (obsMatch) {
      expect(obsMatch[1]).not.toContain("action");
    }
  });

  it("list slot is NOT in observedAttributes", () => {
    const code = compile("component X { slot items: list<text>; Row {} }");
    const obsMatch = code.match(/observedAttributes.*?\[([^\]]*)\]/s);
    if (obsMatch) {
      expect(obsMatch[1]).not.toContain("items");
    }
  });
});

// ============================================================================
// 6. Property Setters/Getters
// ============================================================================

describe("property setters and getters", () => {
  it("text slot has setter and getter", () => {
    const code = compile("component X { slot title: text; Row {} }");
    expect(code).toContain("set title(v: string)");
    expect(code).toContain("get title(): string");
  });

  it("text slot setter calls _render()", () => {
    const code = compile("component X { slot title: text; Row {} }");
    // Setter should update backing field and call render
    expect(code).toContain("_title = v; this._render()");
  });

  it("bool slot setter", () => {
    const code = compile("component X { slot visible: bool; Row {} }");
    expect(code).toContain("set visible(v: boolean)");
  });

  it("node slot has setter (uses _projectSlot)", () => {
    const code = compile("component X { slot action: node; Row {} }");
    expect(code).toContain("set action(v: HTMLElement)");
    expect(code).toContain("_projectSlot(");
  });

  it("list slot has setter only", () => {
    const code = compile("component X { slot items: list<text>; Row {} }");
    expect(code).toContain("set items(v: string[])");
  });

  it("image slot setter validates URL", () => {
    const code = compile("component X { slot photo: image; Row {} }");
    expect(code).toContain("set photo(v: string)");
    expect(code).toContain("javascript:");
  });
});

// ============================================================================
// 7. Lifecycle Callbacks
// ============================================================================

describe("lifecycle callbacks", () => {
  it("connectedCallback calls _render", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("connectedCallback(): void { this._render(); }");
  });

  it("disconnectedCallback removes projected slots (when node slots present)", () => {
    const code = compile("component X { slot action: node; Row {} }");
    expect(code).toContain("disconnectedCallback(): void {");
    expect(code).toContain("data-mosaic-slot");
  });

  it("disconnectedCallback not present when no node slots", () => {
    const code = compile("component X { slot title: text; Row {} }");
    expect(code).not.toContain("disconnectedCallback");
  });

  it("_projectSlot helper present when node slots exist", () => {
    const code = compile("component X { slot action: node; Row {} }");
    expect(code).toContain("private _projectSlot(");
  });
});

// ============================================================================
// 8. Layout Styles (CSS format)
// ============================================================================

describe("layout styles (CSS property names)", () => {
  it("Column → display:flex;flex-direction:column in style attr", () => {
    const code = compile("component X { Column {} }");
    expect(code).toContain("display:flex");
    expect(code).toContain("flex-direction:column");
  });

  it("Row → display:flex;flex-direction:row", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("display:flex");
    expect(code).toContain("flex-direction:row");
  });

  it("padding dp → padding:16px", () => {
    const code = compile("component X { Column { padding: 16dp; } }");
    expect(code).toContain("padding:16px");
  });

  it("gap → gap:12px", () => {
    const code = compile("component X { Column { gap: 12dp; } }");
    expect(code).toContain("gap:12px");
  });

  it("width: fill → width:100%", () => {
    const code = compile("component X { Column { width: fill; } }");
    expect(code).toContain("width:100%");
  });

  it("width: wrap → width:fit-content", () => {
    const code = compile("component X { Column { width: wrap; } }");
    expect(code).toContain("width:fit-content");
  });

  it("min-width → min-width (kebab-case, not minWidth)", () => {
    const code = compile("component X { Column { min-width: 100dp; } }");
    expect(code).toContain("min-width:100px");
    expect(code).not.toContain("minWidth");
  });

  it("overflow: hidden → overflow:hidden", () => {
    const code = compile("component X { Column { overflow: hidden; } }");
    expect(code).toContain("overflow:hidden");
  });
});

// ============================================================================
// 9. Visual Styles
// ============================================================================

describe("visual styles", () => {
  it("background color → background-color:rgba(...)", () => {
    const code = compile("component X { Column { background: #2563eb; } }");
    expect(code).toContain("background-color:rgba(37, 99, 235, 1)");
    // Must use CSS property name, not camelCase
    expect(code).not.toContain("backgroundColor");
  });

  it("corner-radius → border-radius:8px", () => {
    const code = compile("component X { Column { corner-radius: 8dp; } }");
    expect(code).toContain("border-radius:8px");
  });

  it("border-width and border-style:solid", () => {
    const code = compile("component X { Column { border-width: 2dp; } }");
    expect(code).toContain("border-width:2px");
    expect(code).toContain("border-style:solid");
  });

  it("opacity → opacity:0 (no quotes)", () => {
    const code = compile("component X { Column { opacity: 0; } }");
    expect(code).toContain("opacity:0");
  });

  it("shadow elevation.low → box-shadow", () => {
    const code = compile("component X { Column { shadow: elevation.low; } }");
    expect(code).toContain("box-shadow:0 1px 3px rgba(0,0,0,0.12)");
  });

  it("visible: false → display:none", () => {
    const code = compile("component X { Column { visible: false; } }");
    expect(code).toContain("display:none");
  });
});

// ============================================================================
// 10. Text Content in _render()
// ============================================================================

describe("text content in render", () => {
  it("literal text content is HTML-escaped and static", () => {
    const code = compile('component X { Text { content: "Hello"; } }');
    // The literal "Hello" should appear in the render HTML
    expect(code).toContain("Hello");
  });

  it("slot ref content uses _escapeHtml", () => {
    const code = compile("component X { slot title: text; Text { content: @title; } }");
    expect(code).toContain("_escapeHtml");
    expect(code).toContain("_title");
  });

  it("_escapeHtml method is present", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("private _escapeHtml(s: string): string {");
  });

  it("_escapeHtml escapes special characters", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("replace(/&/g, '&amp;')");
    expect(code).toContain("replace(/</g, '&lt;')");
  });

  it("font-weight → font-weight:bold (CSS kebab-case)", () => {
    const code = compile("component X { Text { font-weight: bold; } }");
    expect(code).toContain("font-weight:bold");
    expect(code).not.toContain("fontWeight");
  });

  it("text-align: center → text-align:center", () => {
    const code = compile("component X { Text { text-align: center; } }");
    expect(code).toContain("text-align:center");
  });

  it("style: heading.large → class name + CSS constant", () => {
    const code = compile("component X { Text { style: heading.large; } }");
    expect(code).toContain("mosaic-heading-large");
    expect(code).toContain("MOSAIC_TYPE_SCALE_CSS");
  });
});

// ============================================================================
// 11. Image Properties
// ============================================================================

describe("image properties", () => {
  it("size → width and height in CSS", () => {
    const code = compile("component X { Column { Image { size: 48dp; } } }");
    expect(code).toContain("width:48px");
    expect(code).toContain("height:48px");
  });

  it("shape: circle → border-radius:50%", () => {
    const code = compile("component X { Column { Image { shape: circle; } } }");
    expect(code).toContain("border-radius:50%");
  });

  it("fit: cover → object-fit:cover (CSS kebab-case)", () => {
    const code = compile("component X { Column { Image { fit: cover; } } }");
    expect(code).toContain("object-fit:cover");
    expect(code).not.toContain("objectFit");
  });
});

// ============================================================================
// 12. Alignment Properties
// ============================================================================

describe("alignment properties", () => {
  it("Column align: center → align-items:center", () => {
    const code = compile("component X { Column { align: center; } }");
    expect(code).toContain("align-items:center");
    expect(code).not.toContain("alignItems");
  });

  it("Row align: center → align-items and justify-content", () => {
    const code = compile("component X { Row { align: center; } }");
    expect(code).toContain("align-items:center");
    expect(code).toContain("justify-content:center");
  });

  it("Row align: end → justify-content:flex-end", () => {
    const code = compile("component X { Row { align: end; } }");
    expect(code).toContain("justify-content:flex-end");
  });
});

// ============================================================================
// 13. Accessibility
// ============================================================================

describe("accessibility", () => {
  it("a11y-role: button → role='button'", () => {
    const code = compile('component X { Row { a11y-role: button; } }');
    expect(code).toContain('role="button"');
  });

  it("a11y-role: none → aria-hidden='true'", () => {
    const code = compile('component X { Row { a11y-role: none; } }');
    expect(code).toContain('aria-hidden="true"');
  });

  it("a11y-hidden: true → aria-hidden='true'", () => {
    const code = compile('component X { Row { a11y-hidden: true; } }');
    expect(code).toContain('aria-hidden="true"');
  });
});

// ============================================================================
// 14. Slot Projection (renderSlotChild)
// ============================================================================

describe("slot projection", () => {
  it("@slot; as child → <slot name='...'>", () => {
    const code = compile("component X { slot header: node; Column { @header; } }");
    expect(code).toContain('<slot name="header"></slot>');
  });

  it("_projectSlot is called in setter", () => {
    const code = compile("component X { slot action: node; Row {} }");
    expect(code).toContain("this._projectSlot('action', v)");
  });
});

// ============================================================================
// 15. When Blocks
// ============================================================================

describe("when blocks", () => {
  it("when block emits if statement", () => {
    const code = compile(`
      component X {
        slot show: bool;
        Column { when @show { Row {} } }
      }
    `);
    expect(code).toContain("if (this._show)");
  });

  it("content inside when block is between if braces", () => {
    const code = compile(`
      component X {
        slot visible: bool;
        Column { when @visible { Text {} } }
      }
    `);
    const ifIdx = code.indexOf("if (this._visible)");
    const closingBrace = code.indexOf("\n    }", ifIdx);
    const spanStart = code.indexOf("<span", ifIdx);
    expect(spanStart).toBeGreaterThan(ifIdx);
    expect(spanStart).toBeLessThan(closingBrace);
  });
});

// ============================================================================
// 16. Each Blocks
// ============================================================================

describe("each blocks", () => {
  it("each block uses forEach", () => {
    const code = compile(`
      component X {
        slot items: list<text>;
        Column { each @items as item { Text {} } }
      }
    `);
    expect(code).toContain("this._items.forEach(item =>");
  });

  it("each body content is inside forEach", () => {
    const code = compile(`
      component X {
        slot tags: list<text>;
        Column { each @tags as tag { Row {} } }
      }
    `);
    const forEach = code.indexOf("this._tags.forEach");
    const closeForEach = code.indexOf("});", forEach);
    const rowStart = code.indexOf("<div style=", forEach);
    expect(rowStart).toBeGreaterThan(forEach);
    expect(rowStart).toBeLessThan(closeForEach);
  });

  it("list<node> each uses indexed slot names", () => {
    const code = compile(`
      component X {
        slot items: list<node>;
        Column { each @items as item { Row {} } }
      }
    `);
    // Node lists use slot projection with indexed names
    expect(code).toContain("items-");
  });
});

// ============================================================================
// 17. Type Scale CSS
// ============================================================================

describe("type scale CSS", () => {
  it("MOSAIC_TYPE_SCALE_CSS constant present when style: used", () => {
    const code = compile("component X { Text { style: heading.large; } }");
    expect(code).toContain("MOSAIC_TYPE_SCALE_CSS");
  });

  it("type scale CSS not present when no style: used", () => {
    const code = compile("component X { Text {} }");
    expect(code).not.toContain("MOSAIC_TYPE_SCALE_CSS");
  });

  it("type scale is emitted in shadow DOM via <style>", () => {
    const code = compile("component X { Text { style: heading.large; } }");
    expect(code).toContain("MOSAIC_TYPE_SCALE_CSS");
    expect(code).toContain("<style>");
  });
});

// ============================================================================
// 18. Integration test
// ============================================================================

describe("full integration", () => {
  it("complete component with multiple slot types", () => {
    const source = `
      component Card {
        slot title: text;
        slot count: number = 0;
        slot visible: bool = true;
        Column {
          padding: 16dp;
          Text { content: @title; }
        }
      }
    `;
    const code = compile(source);

    // Class structure
    expect(code).toContain("class MosaicCardElement extends HTMLElement");
    expect(code).toContain("customElements.define('mosaic-card'");

    // Fields
    expect(code).toContain("private _title: string = '';");
    expect(code).toContain("private _count: number = 0;");
    expect(code).toContain("private _visible: boolean = true;");

    // Observable attributes
    expect(code).toContain("'title'");
    expect(code).toContain("'count'");
    expect(code).toContain("'visible'");

    // Render
    expect(code).toContain("padding:16px");
    expect(code).toContain("_escapeHtml");
    expect(code).toContain("_title");
  });
});
