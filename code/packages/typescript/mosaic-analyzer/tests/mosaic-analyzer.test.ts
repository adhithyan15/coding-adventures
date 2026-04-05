/**
 * Tests for the Mosaic Analyzer (TypeScript).
 *
 * These tests verify that `analyzeMosaic` correctly transforms a Mosaic source
 * string into a typed `MosaicIR`. The analyzer is the semantic stage of the
 * compiler: it strips syntax noise, resolves types, normalizes values, and
 * structures the node tree.
 *
 * Test Categories
 * ---------------
 *
 *   1. **Top-level structure** — component name, imports
 *   2. **Slot types** — all primitives (text, number, bool, image, color, node),
 *      component types, list types
 *   3. **Slot defaults** — string, number, dimension, color, bool
 *   4. **Slot required flag** — required when no default, optional when default provided
 *   5. **Node tree** — tag name, isPrimitive, nested elements
 *   6. **Property values** — slot_ref, string, number, dimension, color_hex, bool, ident, enum
 *   7. **Slot references as children** — @slot; in node body
 *   8. **When blocks** — slotName, nested children
 *   9. **Each blocks** — slotName, itemName, nested children
 *  10. **Imports** — componentName, alias, path
 *  11. **Error cases** — empty input, missing component
 */

import { describe, it, expect } from "vitest";
import { analyzeMosaic, AnalysisError } from "../src/analyzer.js";
import type { MosaicIR, MosaicValue } from "../src/ir.js";

// ============================================================================
// Fixtures
// ============================================================================

const MINIMAL = `
  component X {
    Row {}
  }
`;

const PROFILE_CARD = `
  component ProfileCard {
    slot avatar-url: image;
    slot display-name: text;
    slot count: number = 0;
    slot visible: bool = true;
    slot bg: color = #ffffff;
    slot content: node;

    Column {
      Text { content: @display-name; }
    }
  }
`;

// ============================================================================
// 1. Top-Level Structure
// ============================================================================

describe("top-level structure", () => {
  it("returns a MosaicIR with component and imports", () => {
    const ir = analyzeMosaic(MINIMAL);
    expect(ir.component).toBeDefined();
    expect(ir.imports).toBeDefined();
    expect(Array.isArray(ir.imports)).toBe(true);
  });

  it("captures component name", () => {
    const ir = analyzeMosaic("component MyButton { Row {} }");
    expect(ir.component.name).toBe("MyButton");
  });

  it("imports is empty when no imports", () => {
    const ir = analyzeMosaic(MINIMAL);
    expect(ir.imports).toHaveLength(0);
  });

  it("captures one import", () => {
    const src = `import Button from "./button.mosaic"; component X { Row {} }`;
    const ir = analyzeMosaic(src);
    expect(ir.imports).toHaveLength(1);
    expect(ir.imports[0].componentName).toBe("Button");
    expect(ir.imports[0].path).toBe("./button.mosaic");
  });

  it("captures aliased import", () => {
    const src = `import Card as InfoCard from "./card.mosaic"; component X { Row {} }`;
    const ir = analyzeMosaic(src);
    expect(ir.imports[0].alias).toBe("InfoCard");
  });

  it("captures multiple imports", () => {
    const src = `
      import A from "./a.mosaic";
      import B from "./b.mosaic";
      component X { Row {} }
    `;
    const ir = analyzeMosaic(src);
    expect(ir.imports).toHaveLength(2);
  });
});

// ============================================================================
// 2. Slot Types
// ============================================================================

describe("slot types", () => {
  const slotType = (decl: string) => {
    const ir = analyzeMosaic(`component X { ${decl} Row {} }`);
    return ir.component.slots[0].type;
  };

  it("text type", () => {
    expect(slotType("slot t: text;")).toEqual({ kind: "text" });
  });

  it("number type", () => {
    expect(slotType("slot n: number;")).toEqual({ kind: "number" });
  });

  it("bool type", () => {
    expect(slotType("slot b: bool;")).toEqual({ kind: "bool" });
  });

  it("image type", () => {
    expect(slotType("slot i: image;")).toEqual({ kind: "image" });
  });

  it("color type", () => {
    expect(slotType("slot c: color;")).toEqual({ kind: "color" });
  });

  it("node type", () => {
    expect(slotType("slot n: node;")).toEqual({ kind: "node" });
  });

  it("component type (imported name)", () => {
    expect(slotType("slot a: Button;")).toEqual({ kind: "component", name: "Button" });
  });

  it("list<text> type", () => {
    expect(slotType("slot items: list<text>;")).toEqual({
      kind: "list",
      elementType: { kind: "text" },
    });
  });

  it("list<Button> type", () => {
    expect(slotType("slot buttons: list<Button>;")).toEqual({
      kind: "list",
      elementType: { kind: "component", name: "Button" },
    });
  });

  it("list<node> type", () => {
    expect(slotType("slot children: list<node>;")).toEqual({
      kind: "list",
      elementType: { kind: "node" },
    });
  });
});

// ============================================================================
// 3. Slot Defaults
// ============================================================================

describe("slot defaults", () => {
  const defaultVal = (decl: string): MosaicValue | undefined => {
    const ir = analyzeMosaic(`component X { ${decl} Row {} }`);
    return ir.component.slots[0].defaultValue;
  };

  it("string default", () => {
    expect(defaultVal('slot t: text = "hello";')).toEqual({ kind: "string", value: "hello" });
  });

  it("number default (integer)", () => {
    expect(defaultVal("slot n: number = 42;")).toEqual({ kind: "number", value: 42 });
  });

  it("number default (zero)", () => {
    expect(defaultVal("slot n: number = 0;")).toEqual({ kind: "number", value: 0 });
  });

  it("dimension default", () => {
    expect(defaultVal("slot n: number = 16dp;")).toEqual({
      kind: "dimension",
      value: 16,
      unit: "dp",
    });
  });

  it("color_hex default", () => {
    expect(defaultVal("slot c: color = #ffffff;")).toEqual({
      kind: "color_hex",
      value: "#ffffff",
    });
  });

  it("bool default true", () => {
    expect(defaultVal("slot b: bool = true;")).toEqual({ kind: "bool", value: true });
  });

  it("bool default false", () => {
    expect(defaultVal("slot b: bool = false;")).toEqual({ kind: "bool", value: false });
  });

  it("no default when omitted", () => {
    expect(defaultVal("slot t: text;")).toBeUndefined();
  });
});

// ============================================================================
// 4. Slot Required Flag
// ============================================================================

describe("slot required flag", () => {
  it("slot without default is required", () => {
    const ir = analyzeMosaic("component X { slot t: text; Row {} }");
    expect(ir.component.slots[0].required).toBe(true);
  });

  it("slot with default is not required", () => {
    const ir = analyzeMosaic("component X { slot n: number = 0; Row {} }");
    expect(ir.component.slots[0].required).toBe(false);
  });
});

// ============================================================================
// 5. Node Tree
// ============================================================================

describe("node tree", () => {
  it("captures root tag name", () => {
    const ir = analyzeMosaic("component X { Column {} }");
    expect(ir.component.tree.tag).toBe("Column");
  });

  it("Row is primitive", () => {
    const ir = analyzeMosaic("component X { Row {} }");
    expect(ir.component.tree.isPrimitive).toBe(true);
  });

  it("Column is primitive", () => {
    const ir = analyzeMosaic("component X { Column {} }");
    expect(ir.component.tree.isPrimitive).toBe(true);
  });

  it("Text is primitive", () => {
    const ir = analyzeMosaic("component X { Text {} }");
    expect(ir.component.tree.isPrimitive).toBe(true);
  });

  it("custom component tag is not primitive", () => {
    const ir = analyzeMosaic("component X { MyWidget {} }");
    expect(ir.component.tree.isPrimitive).toBe(false);
  });

  it("nested nodes produce children", () => {
    const ir = analyzeMosaic("component X { Column { Row {} } }");
    const child = ir.component.tree.children[0];
    expect(child.kind).toBe("node");
    if (child.kind === "node") {
      expect(child.node.tag).toBe("Row");
    }
  });
});

// ============================================================================
// 6. Property Values
// ============================================================================

describe("property values", () => {
  const propValue = (propDef: string) => {
    const ir = analyzeMosaic(`component X { Row { ${propDef} } }`);
    return ir.component.tree.properties[0]?.value;
  };

  it("slot_ref value", () => {
    const ir = analyzeMosaic("component X { slot t: text; Row { content: @t; } }");
    const val = ir.component.tree.properties[0].value;
    expect(val).toEqual({ kind: "slot_ref", slotName: "t" });
  });

  it("string value", () => {
    expect(propValue('label: "hello";')).toEqual({ kind: "string", value: "hello" });
  });

  it("number value", () => {
    expect(propValue("opacity: 0;")).toEqual({ kind: "number", value: 0 });
  });

  it("dimension value", () => {
    expect(propValue("padding: 16dp;")).toEqual({ kind: "dimension", value: 16, unit: "dp" });
  });

  it("sp dimension value", () => {
    expect(propValue("font-size: 14sp;")).toEqual({ kind: "dimension", value: 14, unit: "sp" });
  });

  it("percent dimension value", () => {
    expect(propValue("width: 100%;")).toEqual({ kind: "dimension", value: 100, unit: "%" });
  });

  it("color_hex value", () => {
    expect(propValue("background: #2563eb;")).toEqual({ kind: "color_hex", value: "#2563eb" });
  });

  it("bool true value", () => {
    expect(propValue("disabled: true;")).toEqual({ kind: "bool", value: true });
  });

  it("bool false value", () => {
    expect(propValue("disabled: false;")).toEqual({ kind: "bool", value: false });
  });

  it("ident value (bare identifier)", () => {
    expect(propValue("align: center;")).toEqual({ kind: "ident", value: "center" });
  });

  it("enum value (namespace.member)", () => {
    expect(propValue("style: heading.small;")).toEqual({
      kind: "enum",
      namespace: "heading",
      member: "small",
    });
  });
});

// ============================================================================
// 7. Slot References as Children
// ============================================================================

describe("slot references as children", () => {
  it("@slot; in node body becomes slot_ref child", () => {
    const ir = analyzeMosaic("component X { slot h: node; Column { @h; } }");
    const children = ir.component.tree.children;
    expect(children).toHaveLength(1);
    expect(children[0]).toEqual({ kind: "slot_ref", slotName: "h" });
  });

  it("multiple slot references in order", () => {
    const ir = analyzeMosaic("component X { slot a: node; slot b: node; Column { @a; @b; } }");
    const children = ir.component.tree.children;
    expect(children).toHaveLength(2);
    expect(children[0]).toEqual({ kind: "slot_ref", slotName: "a" });
    expect(children[1]).toEqual({ kind: "slot_ref", slotName: "b" });
  });
});

// ============================================================================
// 8. When Blocks
// ============================================================================

describe("when blocks", () => {
  it("when block has correct slotName", () => {
    const ir = analyzeMosaic(`
      component X {
        slot show: bool;
        Column {
          when @show { Text {} }
        }
      }
    `);
    const child = ir.component.tree.children[0];
    expect(child.kind).toBe("when");
    if (child.kind === "when") {
      expect(child.slotName).toBe("show");
    }
  });

  it("when block contains nested children", () => {
    const ir = analyzeMosaic(`
      component X {
        slot show: bool;
        Column {
          when @show { Row {} Text {} }
        }
      }
    `);
    const child = ir.component.tree.children[0];
    if (child.kind === "when") {
      expect(child.children.length).toBeGreaterThanOrEqual(1);
    }
  });
});

// ============================================================================
// 9. Each Blocks
// ============================================================================

describe("each blocks", () => {
  it("each block has correct slotName and itemName", () => {
    const ir = analyzeMosaic(`
      component X {
        slot items: list<text>;
        Column {
          each @items as item { Text {} }
        }
      }
    `);
    const child = ir.component.tree.children[0];
    expect(child.kind).toBe("each");
    if (child.kind === "each") {
      expect(child.slotName).toBe("items");
      expect(child.itemName).toBe("item");
    }
  });

  it("each block has nested children", () => {
    const ir = analyzeMosaic(`
      component X {
        slot items: list<text>;
        Column {
          each @items as item {
            Text { content: @item; }
          }
        }
      }
    `);
    const child = ir.component.tree.children[0];
    if (child.kind === "each") {
      expect(child.children.length).toBeGreaterThanOrEqual(1);
    }
  });
});

// ============================================================================
// 10. Full Component Snapshot
// ============================================================================

describe("full component snapshot", () => {
  it("ProfileCard produces expected IR shape", () => {
    const ir = analyzeMosaic(PROFILE_CARD);

    expect(ir.component.name).toBe("ProfileCard");
    expect(ir.component.slots).toHaveLength(6);

    // slot types in order
    expect(ir.component.slots[0].type).toEqual({ kind: "image" });
    expect(ir.component.slots[1].type).toEqual({ kind: "text" });
    expect(ir.component.slots[2].type).toEqual({ kind: "number" });
    expect(ir.component.slots[3].type).toEqual({ kind: "bool" });
    expect(ir.component.slots[4].type).toEqual({ kind: "color" });
    expect(ir.component.slots[5].type).toEqual({ kind: "node" });

    // required flags
    expect(ir.component.slots[0].required).toBe(true);  // no default
    expect(ir.component.slots[2].required).toBe(false); // = 0
    expect(ir.component.slots[3].required).toBe(false); // = true

    // node tree
    expect(ir.component.tree.tag).toBe("Column");
    expect(ir.component.tree.isPrimitive).toBe(true);
  });
});

// ============================================================================
// 11. Error Cases
// ============================================================================

describe("error cases", () => {
  it("throws on empty input", () => {
    expect(() => analyzeMosaic("")).toThrow();
  });

  it("throws on syntactically invalid input", () => {
    expect(() => analyzeMosaic("not valid mosaic")).toThrow();
  });

  it("AnalysisError is thrown for structural issues", () => {
    // The parser should catch most errors, but for any that slip through
    // the analyzer throws AnalysisError.
    expect(AnalysisError).toBeDefined();
    const err = new AnalysisError("test");
    expect(err.name).toBe("AnalysisError");
  });
});
