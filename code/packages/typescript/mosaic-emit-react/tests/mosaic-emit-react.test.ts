/**
 * Tests for MosaicReactRenderer.
 *
 * Each test compiles a small Mosaic snippet through the full pipeline
 * (analyzeMosaic → MosaicVM → MosaicReactRenderer) and asserts on the
 * generated .tsx content.
 *
 * We use `toContain` for most assertions — this avoids brittle whitespace
 * dependencies while still verifying the important structural invariants.
 * For ordering tests (e.g., imports before interface), we use indexOf comparisons.
 *
 * Test Categories
 * ---------------
 *
 *   1. File structure — header, imports, interface, function
 *   2. Props interface — required/optional, default values, types
 *   3. Primitive node → JSX element
 *   4. Layout properties → inline style
 *   5. Visual properties → inline style
 *   6. Text-specific properties
 *   7. Image-specific properties
 *   8. Alignment properties
 *   9. Accessibility attributes
 *  10. Slot references as children (renderSlotChild)
 *  11. When blocks
 *  12. Each blocks
 *  13. Slot type → TypeScript type mapping
 *  14. Non-primitive (imported) components
 */

import { describe, it, expect } from "vitest";
import { MosaicReactRenderer } from "../src/react-renderer.js";
import { MosaicVM } from "@coding-adventures/mosaic-vm";
import { analyzeMosaic } from "@coding-adventures/mosaic-analyzer";

// Helper: run the full pipeline and return the generated file content.
function compile(source: string): string {
  const ir = analyzeMosaic(source);
  const vm = new MosaicVM(ir);
  const renderer = new MosaicReactRenderer();
  const result = vm.run(renderer);
  return result.files[0].content;
}

// ============================================================================
// 1. File Structure
// ============================================================================

describe("file structure", () => {
  it("emits a single .tsx file named after the component", () => {
    const ir = analyzeMosaic("component MyCard { Row {} }");
    const vm = new MosaicVM(ir);
    const result = vm.run(new MosaicReactRenderer());
    expect(result.files).toHaveLength(1);
    expect(result.files[0].filename).toBe("MyCard.tsx");
  });

  it("starts with the auto-generated header comment", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("// AUTO-GENERATED from X.mosaic — do not edit");
  });

  it("imports React", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain('import React from "react"');
  });

  it("imports type-scale CSS only when Text uses style:", () => {
    const withStyle = compile(`component X { Text { style: heading.large; } }`);
    const withoutStyle = compile("component X { Text {} }");
    expect(withStyle).toContain('import "./mosaic-type-scale.css"');
    expect(withoutStyle).not.toContain("mosaic-type-scale.css");
  });

  it("emits a Props interface", () => {
    const code = compile("component Card { slot title: text; Row {} }");
    expect(code).toContain("interface CardProps {");
  });

  it("emits an exported function component", () => {
    const code = compile("component Card { Row {} }");
    expect(code).toContain("export function Card(");
  });

  it("function returns JSX.Element", () => {
    const code = compile("component Card { Row {} }");
    expect(code).toContain("): JSX.Element {");
  });

  it("function has a return statement", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("return (");
    expect(code).toContain(");");
  });

  it("React import comes before the Props interface", () => {
    const code = compile("component X { Row {} }");
    expect(code.indexOf('import React')).toBeLessThan(code.indexOf('interface X'));
  });

  it("Props interface comes before the function", () => {
    const code = compile("component X { Row {} }");
    expect(code.indexOf('interface X')).toBeLessThan(code.indexOf('export function X'));
  });
});

// ============================================================================
// 2. Props Interface
// ============================================================================

describe("props interface", () => {
  it("required slot has no ? and no default", () => {
    const code = compile("component X { slot title: text; Row {} }");
    expect(code).toContain("  title: string;");
    expect(code).not.toContain("title?:");
  });

  it("optional slot (with default) has ? in interface", () => {
    const code = compile('component X { slot count: number = 0; Row {} }');
    expect(code).toContain("  count?: number;");
  });

  it("default value appears in function parameter", () => {
    const code = compile('component X { slot count: number = 0; Row {} }');
    expect(code).toContain("  count = 0,");
  });

  it("bool default false appears in function parameter", () => {
    const code = compile('component X { slot expanded: bool = false; Row {} }');
    expect(code).toContain("  expanded = false,");
  });

  it("string default appears in function parameter", () => {
    const code = compile('component X { slot label: text = "Hello"; Row {} }');
    expect(code).toContain('  label = "Hello",');
  });

  it("required slot has no default in parameters", () => {
    const code = compile("component X { slot title: text; Row {} }");
    expect(code).toContain("  title,");
  });

  it("function destructures all slots", () => {
    const code = compile("component X { slot a: text; slot b: number; Row {} }");
    expect(code).toContain("  a,");
    expect(code).toContain("  b,");
  });

  it("empty component has empty props interface", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain("interface XProps {");
    expect(code).toContain("}");
  });
});

// ============================================================================
// 3. Primitive Node → JSX Element
// ============================================================================

describe("primitive node to JSX element", () => {
  it("Column → div with flex column", () => {
    const code = compile("component X { Column {} }");
    expect(code).toContain('<div style={{ display: "flex", flexDirection: "column" }}');
  });

  it("Row → div with flex row", () => {
    const code = compile("component X { Row {} }");
    expect(code).toContain('<div style={{ display: "flex", flexDirection: "row" }}');
  });

  it("Box → div with position relative", () => {
    const code = compile("component X { Box {} }");
    expect(code).toContain('<div style={{ position: "relative" }}');
  });

  it("Text → span", () => {
    // Text with content renders as non-self-closing span
    const code = compile('component X { Column { Text { content: "hi"; } } }');
    expect(code).toContain("<span");
    expect(code).toContain("</span>");
  });

  it("Image → img self-closing", () => {
    const code = compile("component X { Column { Image {} } }");
    expect(code).toContain("<img");
    expect(code).toContain("/>");
    // img should be self-closing
    expect(code).not.toContain("</img>");
  });

  it("Spacer → div with flex 1", () => {
    const code = compile("component X { Row { Spacer {} } }");
    expect(code).toContain('<div style={{ flex: 1 }}');
  });

  it("Scroll → div with overflow auto", () => {
    const code = compile("component X { Scroll {} }");
    expect(code).toContain('<div style={{ overflow: "auto" }}');
  });

  it("Divider → hr self-closing with border styles", () => {
    const code = compile("component X { Column { Divider {} } }");
    expect(code).toContain("<hr");
    expect(code).toContain("/>");
    expect(code).not.toContain("</hr>");
    expect(code).toContain('border: "none"');
    expect(code).toContain('borderTop: "1px solid currentColor"');
  });

  it("nested nodes: Column containing Text", () => {
    const code = compile("component X { Column { Text {} } }");
    const colStart = code.indexOf('<div style={{ display: "flex", flexDirection: "column" }}');
    const spanStart = code.indexOf("<span");
    const colEnd = code.indexOf("</div>");
    // Column opens before Text opens
    expect(colStart).toBeLessThan(spanStart);
    // Text closes before Column closes
    expect(spanStart).toBeLessThan(colEnd);
  });

  it("empty node renders self-closing", () => {
    const code = compile("component X { Spacer {} }");
    expect(code).toContain("/>");
  });
});

// ============================================================================
// 4. Layout Properties
// ============================================================================

describe("layout properties", () => {
  it("padding dp → px", () => {
    const code = compile("component X { Column { padding: 16dp; } }");
    expect(code).toContain('padding: "16px"');
  });

  it("padding sp → px", () => {
    const code = compile("component X { Column { padding: 14sp; } }");
    expect(code).toContain('padding: "14px"');
  });

  it("padding % passes through", () => {
    const code = compile("component X { Column { padding: 10%; } }");
    expect(code).toContain('padding: "10%"');
  });

  it("padding-left → paddingLeft", () => {
    const code = compile("component X { Column { padding-left: 8dp; } }");
    expect(code).toContain('paddingLeft: "8px"');
  });

  it("padding-right → paddingRight", () => {
    const code = compile("component X { Column { padding-right: 8dp; } }");
    expect(code).toContain('paddingRight: "8px"');
  });

  it("padding-top → paddingTop", () => {
    const code = compile("component X { Column { padding-top: 8dp; } }");
    expect(code).toContain('paddingTop: "8px"');
  });

  it("padding-bottom → paddingBottom", () => {
    const code = compile("component X { Column { padding-bottom: 8dp; } }");
    expect(code).toContain('paddingBottom: "8px"');
  });

  it("gap dp → px", () => {
    const code = compile("component X { Column { gap: 12dp; } }");
    expect(code).toContain('gap: "12px"');
  });

  it("width: fill → 100%", () => {
    const code = compile("component X { Column { width: fill; } }");
    expect(code).toContain('width: "100%"');
  });

  it("width: wrap → fit-content", () => {
    const code = compile("component X { Column { width: wrap; } }");
    expect(code).toContain('width: "fit-content"');
  });

  it("width dimension → px", () => {
    const code = compile("component X { Column { width: 200dp; } }");
    expect(code).toContain('width: "200px"');
  });

  it("height: fill → 100%", () => {
    const code = compile("component X { Column { height: fill; } }");
    expect(code).toContain('height: "100%"');
  });

  it("height: wrap → fit-content", () => {
    const code = compile("component X { Column { height: wrap; } }");
    expect(code).toContain('height: "fit-content"');
  });

  it("min-width → minWidth", () => {
    const code = compile("component X { Column { min-width: 100dp; } }");
    expect(code).toContain('minWidth: "100px"');
  });

  it("max-width → maxWidth", () => {
    const code = compile("component X { Column { max-width: 400dp; } }");
    expect(code).toContain('maxWidth: "400px"');
  });

  it("min-height → minHeight", () => {
    const code = compile("component X { Column { min-height: 48dp; } }");
    expect(code).toContain('minHeight: "48px"');
  });

  it("max-height → maxHeight", () => {
    const code = compile("component X { Column { max-height: 300dp; } }");
    expect(code).toContain('maxHeight: "300px"');
  });

  it("overflow: visible → visible", () => {
    const code = compile("component X { Column { overflow: visible; } }");
    expect(code).toContain('overflow: "visible"');
  });

  it("overflow: hidden → hidden", () => {
    const code = compile("component X { Column { overflow: hidden; } }");
    expect(code).toContain('overflow: "hidden"');
  });

  it("overflow: scroll → auto", () => {
    const code = compile("component X { Column { overflow: scroll; } }");
    expect(code).toContain('overflow: "auto"');
  });

  it("fractional dimension (1.5dp)", () => {
    const code = compile("component X { Column { padding: 1.5dp; } }");
    expect(code).toContain('padding: "1.5px"');
  });
});

// ============================================================================
// 5. Visual Properties
// ============================================================================

describe("visual properties", () => {
  it("background color → backgroundColor as rgba()", () => {
    const code = compile("component X { Column { background: #2563eb; } }");
    expect(code).toContain('backgroundColor: "rgba(37, 99, 235, 1)"');
  });

  it("background color with alpha → correct alpha fraction", () => {
    const code = compile("component X { Column { background: #00000080; } }");
    expect(code).toContain("backgroundColor:");
    expect(code).toContain("rgba(0, 0, 0,");
  });

  it("color (text color) → color as rgba()", () => {
    const code = compile("component X { Text { color: #fff; } }");
    expect(code).toContain('color: "rgba(255, 255, 255, 1)"');
  });

  it("corner-radius → borderRadius in px", () => {
    const code = compile("component X { Column { corner-radius: 8dp; } }");
    expect(code).toContain('borderRadius: "8px"');
  });

  it("border-width → borderWidth and borderStyle solid", () => {
    const code = compile("component X { Column { border-width: 2dp; } }");
    expect(code).toContain('borderWidth: "2px"');
    expect(code).toContain('borderStyle: "solid"');
  });

  it("border-color → borderColor as rgba()", () => {
    const code = compile("component X { Column { border-color: #000; } }");
    expect(code).toContain("borderColor:");
    expect(code).toContain("rgba(0, 0, 0, 1)");
  });

  it("opacity → opacity as number (no quotes)", () => {
    const code = compile("component X { Column { opacity: 0; } }");
    expect(code).toContain("opacity: 0");
    // Opacity should be a number, not a string in JSX style
    expect(code).not.toContain('opacity: "0"');
  });

  it("opacity 0.5", () => {
    const code = compile("component X { Column { opacity: 0; } }");
    // Check it's numeric (no string quotes around the number)
    const opacityMatch = code.match(/opacity: (\S+)/);
    expect(opacityMatch).not.toBeNull();
    expect(opacityMatch![1]).not.toMatch(/^"/);
  });

  it("shadow elevation.none → none", () => {
    const code = compile("component X { Column { shadow: elevation.none; } }");
    expect(code).toContain('boxShadow: "none"');
  });

  it("shadow elevation.low → low shadow", () => {
    const code = compile("component X { Column { shadow: elevation.low; } }");
    expect(code).toContain('boxShadow: "0 1px 3px rgba(0,0,0,0.12)"');
  });

  it("shadow elevation.medium → medium shadow", () => {
    const code = compile("component X { Column { shadow: elevation.medium; } }");
    expect(code).toContain('boxShadow: "0 4px 12px rgba(0,0,0,0.15)"');
  });

  it("shadow elevation.high → high shadow", () => {
    const code = compile("component X { Column { shadow: elevation.high; } }");
    expect(code).toContain('boxShadow: "0 8px 24px rgba(0,0,0,0.20)"');
  });

  it("visible: false → display none", () => {
    const code = compile("component X { Column { visible: false; } }");
    expect(code).toContain('display: "none"');
  });
});

// ============================================================================
// 6. Text-Specific Properties
// ============================================================================

describe("text-specific properties", () => {
  it("content: string literal → inline text children", () => {
    const code = compile('component X { Text { content: "Hello"; } }');
    expect(code).toContain(">Hello<");
  });

  it("content: slot ref → expression children", () => {
    const code = compile("component X { slot title: text; Text { content: @title; } }");
    expect(code).toContain(">{title}<");
  });

  it("text-align: start → textAlign left", () => {
    const code = compile("component X { Text { text-align: start; } }");
    expect(code).toContain('textAlign: "left"');
  });

  it("text-align: center → textAlign center", () => {
    const code = compile("component X { Text { text-align: center; } }");
    expect(code).toContain('textAlign: "center"');
  });

  it("text-align: end → textAlign right", () => {
    const code = compile("component X { Text { text-align: end; } }");
    expect(code).toContain('textAlign: "right"');
  });

  it("font-weight: bold → fontWeight bold", () => {
    const code = compile("component X { Text { font-weight: bold; } }");
    expect(code).toContain('fontWeight: "bold"');
  });

  it("font-weight: normal → fontWeight normal", () => {
    const code = compile("component X { Text { font-weight: normal; } }");
    expect(code).toContain('fontWeight: "normal"');
  });

  it("max-lines → webkit line clamp styles", () => {
    const code = compile("component X { Text { max-lines: 3; } }");
    expect(code).toContain("WebkitLineClamp: 3");
    expect(code).toContain('overflow: "hidden"');
    expect(code).toContain('display: "-webkit-box"');
    expect(code).toContain('WebkitBoxOrient: "vertical"');
  });

  it("style: heading.large → className + CSS import", () => {
    const code = compile("component X { Text { style: heading.large; } }");
    expect(code).toContain('className="mosaic-heading-large"');
    expect(code).toContain("mosaic-type-scale.css");
  });

  it("style: heading.medium → correct class name", () => {
    const code = compile("component X { Text { style: heading.medium; } }");
    expect(code).toContain('className="mosaic-heading-medium"');
  });

  it("style: body.small → correct class name", () => {
    const code = compile("component X { Text { style: body.small; } }");
    expect(code).toContain('className="mosaic-body-small"');
  });
});

// ============================================================================
// 7. Image-Specific Properties
// ============================================================================

describe("image-specific properties", () => {
  it("source: slot ref → src={slotName}", () => {
    const code = compile("component X { slot photo: text; Column { Image { source: @photo; } } }");
    expect(code).toContain("src={photo}");
  });

  it('source: string literal → src="literal"', () => {
    const code = compile('component X { Column { Image { source: "logo.png"; } } }');
    expect(code).toContain('src="logo.png"');
  });

  it("size → width and height both set to px", () => {
    const code = compile("component X { Column { Image { size: 48dp; } } }");
    expect(code).toContain('width: "48px"');
    expect(code).toContain('height: "48px"');
  });

  it("shape: circle → borderRadius 50%", () => {
    const code = compile("component X { Column { Image { shape: circle; } } }");
    expect(code).toContain('borderRadius: "50%"');
  });

  it("shape: rounded → borderRadius 8px", () => {
    const code = compile("component X { Column { Image { shape: rounded; } } }");
    expect(code).toContain('borderRadius: "8px"');
  });

  it("fit: cover → objectFit cover", () => {
    const code = compile("component X { Column { Image { fit: cover; } } }");
    expect(code).toContain('objectFit: "cover"');
  });

  it("fit: contain → objectFit contain", () => {
    const code = compile("component X { Column { Image { fit: contain; } } }");
    expect(code).toContain('objectFit: "contain"');
  });
});

// ============================================================================
// 8. Alignment Properties
// ============================================================================

describe("alignment properties", () => {
  it("Column align: center → alignItems center", () => {
    const code = compile("component X { Column { align: center; } }");
    expect(code).toContain('alignItems: "center"');
  });

  it("Row align: center → alignItems center and justifyContent center", () => {
    const code = compile("component X { Row { align: center; } }");
    expect(code).toContain('alignItems: "center"');
    expect(code).toContain('justifyContent: "center"');
  });

  it("Column align: start → alignItems flex-start", () => {
    const code = compile("component X { Column { align: start; } }");
    expect(code).toContain('alignItems: "flex-start"');
  });

  it("Column align: end → alignItems flex-end", () => {
    const code = compile("component X { Column { align: end; } }");
    expect(code).toContain('alignItems: "flex-end"');
  });

  it("Column align: stretch → alignItems stretch", () => {
    const code = compile("component X { Column { align: stretch; } }");
    expect(code).toContain('alignItems: "stretch"');
  });

  it("Column align: center-horizontal → alignItems center (cross axis)", () => {
    const code = compile("component X { Column { align: center-horizontal; } }");
    expect(code).toContain('alignItems: "center"');
  });

  it("Column align: center-vertical → justifyContent center (main axis)", () => {
    const code = compile("component X { Column { align: center-vertical; } }");
    expect(code).toContain('justifyContent: "center"');
  });

  it("Row align: center-horizontal → justifyContent center (main axis)", () => {
    const code = compile("component X { Row { align: center-horizontal; } }");
    expect(code).toContain('justifyContent: "center"');
  });

  it("Row align: center-vertical → alignItems center (cross axis)", () => {
    const code = compile("component X { Row { align: center-vertical; } }");
    expect(code).toContain('alignItems: "center"');
  });

  it("Box align → sets display flex additionally", () => {
    const code = compile("component X { Box { align: center; } }");
    expect(code).toContain('display: "flex"');
    expect(code).toContain('alignItems: "center"');
  });
});

// ============================================================================
// 9. Accessibility Attributes
// ============================================================================

describe("accessibility attributes", () => {
  it("a11y-label: string → aria-label attribute", () => {
    const code = compile('component X { Row { a11y-label: "Save"; } }');
    expect(code).toContain('aria-label="Save"');
  });

  it("a11y-label: slot ref → aria-label expression", () => {
    const code = compile("component X { slot desc: text; Row { a11y-label: @desc; } }");
    expect(code).toContain("aria-label={desc}");
  });

  it("a11y-role: button → role button", () => {
    const code = compile('component X { Row { a11y-role: button; } }');
    expect(code).toContain('role="button"');
  });

  it("a11y-role: heading on Text → becomes h2 element", () => {
    // h2 with content renders non-self-closing
    const code = compile('component X { Column { Text { a11y-role: heading; content: "Title"; } } }');
    expect(code).toContain("<h2");
    expect(code).toContain("</h2>");
    // Should not have explicit role="heading" (it's implied by <h2>)
    expect(code).not.toContain('role="heading"');
  });

  it("a11y-role: image → role img", () => {
    const code = compile('component X { Row { a11y-role: image; } }');
    expect(code).toContain('role="img"');
  });

  it("a11y-role: list → role list", () => {
    const code = compile('component X { Column { a11y-role: list; } }');
    expect(code).toContain('role="list"');
  });

  it("a11y-role: none → aria-hidden true", () => {
    const code = compile('component X { Row { a11y-role: none; } }');
    expect(code).toContain('aria-hidden="true"');
  });

  it("a11y-hidden: true → aria-hidden true", () => {
    const code = compile('component X { Row { a11y-hidden: true; } }');
    expect(code).toContain('aria-hidden="true"');
  });
});

// ============================================================================
// 10. Slot References as Children
// ============================================================================

describe("slot references as children", () => {
  it("@slot; as child emits {slotName}", () => {
    const code = compile("component X { slot header: node; Column { @header; } }");
    expect(code).toContain("{header}");
  });

  it("slot child inside a Column", () => {
    const code = compile("component X { slot action: node; Column { Row {} @action; } }");
    // Both the Row and the action slot should appear
    const colStart = code.indexOf('display: "flex", flexDirection: "column"');
    const actionIdx = code.indexOf("{action}");
    expect(colStart).toBeGreaterThan(-1);
    expect(actionIdx).toBeGreaterThan(colStart);
  });
});

// ============================================================================
// 11. When Blocks
// ============================================================================

describe("when blocks", () => {
  it("emits a conditional expression for when block", () => {
    const code = compile(`
      component X {
        slot show: bool;
        Column { when @show { Row {} } }
      }
    `);
    expect(code).toContain("{show && (");
  });

  it("when block content is between the conditional and closing paren", () => {
    const code = compile(`
      component X {
        slot show: bool;
        Column { when @show { Text {} } }
      }
    `);
    const whenStart = code.indexOf("{show && (");
    const whenContent = code.indexOf("<span", whenStart);
    const whenEnd = code.indexOf(")}", whenStart);
    expect(whenContent).toBeGreaterThan(whenStart);
    expect(whenContent).toBeLessThan(whenEnd);
  });
});

// ============================================================================
// 12. Each Blocks
// ============================================================================

describe("each blocks", () => {
  it("emits a .map() expression", () => {
    const code = compile(`
      component X {
        slot items: list<text>;
        Column { each @items as item { Text {} } }
      }
    `);
    expect(code).toContain("items.map((item, _index) =>");
  });

  it("uses React.Fragment with key={_index}", () => {
    const code = compile(`
      component X {
        slot items: list<text>;
        Column { each @items as item { Text {} } }
      }
    `);
    expect(code).toContain("<React.Fragment key={_index}>");
    expect(code).toContain("</React.Fragment>");
  });

  it("loop variable ref emits bare variable name", () => {
    const code = compile(`
      component X {
        slot items: list<text>;
        Column {
          each @items as item {
            Text { content: @item; }
          }
        }
      }
    `);
    // The item variable should appear as {item} (not {props.item})
    expect(code).toContain(">{item}<");
  });

  it("each body content is inside the Fragment", () => {
    const code = compile(`
      component X {
        slot tags: list<text>;
        Column { each @tags as tag { Row {} } }
      }
    `);
    const mapStart = code.indexOf("tags.map");
    const fragEnd = code.indexOf("</React.Fragment>", mapStart);
    const rowStart = code.indexOf('<div style={{ display: "flex", flexDirection: "row"', mapStart);
    expect(rowStart).toBeGreaterThan(mapStart);
    expect(rowStart).toBeLessThan(fragEnd);
  });
});

// ============================================================================
// 13. Slot Type → TypeScript Type Mapping
// ============================================================================

describe("slot type to TypeScript type", () => {
  it("text → string", () => {
    const code = compile("component X { slot s: text; Row {} }");
    expect(code).toContain("s: string;");
  });

  it("number → number", () => {
    const code = compile("component X { slot n: number; Row {} }");
    expect(code).toContain("n: number;");
  });

  it("bool → boolean", () => {
    const code = compile("component X { slot b: bool; Row {} }");
    expect(code).toContain("b: boolean;");
  });

  it("node → React.ReactNode", () => {
    const code = compile("component X { slot c: node; Row {} }");
    expect(code).toContain("c: React.ReactNode;");
  });

  it("list<text> → string[]", () => {
    const code = compile("component X { slot tags: list<text>; Row {} }");
    expect(code).toContain("tags: string[];");
  });

  it("list<number> → number[]", () => {
    const code = compile("component X { slot vals: list<number>; Row {} }");
    expect(code).toContain("vals: number[];");
  });

  it("list<node> → React.ReactNode[]", () => {
    const code = compile("component X { slot children: list<node>; Row {} }");
    expect(code).toContain("children: React.ReactNode[];");
  });
});

// ============================================================================
// 14. Non-Primitive (Imported) Component Nodes
// ============================================================================

describe("non-primitive component nodes", () => {
  it("imported component slot emits type import", () => {
    const code = compile("component X { slot btn: Button; Row {} }");
    expect(code).toContain('import type { ButtonProps } from "./Button.js"');
  });

  it("imported component slot type uses ReactElement<TProps>", () => {
    const code = compile("component X { slot btn: Button; Row {} }");
    expect(code).toContain("React.ReactElement<ButtonProps>");
  });

  it("list<component> slot emits type import", () => {
    const code = compile("component X { slot btns: list<Card>; Row {} }");
    expect(code).toContain('import type { CardProps } from "./Card.js"');
    expect(code).toContain("Array<React.ReactElement<CardProps>>");
  });
});

// ============================================================================
// 15. Full Integration — Example Component
// ============================================================================

describe("full integration", () => {
  it("profile card example generates correct structure", () => {
    const source = `
      component ProfileCard {
        slot name: text;
        slot count: number = 0;
        Column {
          padding: 16dp;
          gap: 12dp;
          Text { content: @name; }
          Text { content: @count; font-weight: bold; }
        }
      }
    `;
    const code = compile(source);

    // File basics
    expect(code).toContain("interface ProfileCardProps {");
    expect(code).toContain("export function ProfileCard(");

    // Props
    expect(code).toContain("  name: string;");
    expect(code).toContain("  count?: number;");
    expect(code).toContain("  name,");
    expect(code).toContain("  count = 0,");

    // JSX
    expect(code).toContain('padding: "16px"');
    expect(code).toContain('gap: "12px"');
    expect(code).toContain(">{name}<");
    expect(code).toContain(">{count}<");
    expect(code).toContain('fontWeight: "bold"');
  });
});
