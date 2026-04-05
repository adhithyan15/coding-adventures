/**
 * Tests for the Mosaic Parser (TypeScript).
 *
 * These tests verify that parseMosaic correctly converts Mosaic source text
 * into an ASTNode tree. The parser wraps the generic GrammarParser with the
 * Mosaic grammar, so tests focus on the Mosaic-specific structure.
 *
 * AST Navigation Convention
 * -------------------------
 *
 * `ASTNode.children` contains a mix of ASTNode and Token objects. To find
 * a specific child node, search by `ruleName`. To find a specific leaf token,
 * search by token `type` or `value`.
 *
 * Test Categories
 * ---------------
 *
 *   1. **File structure** — root rule, no imports, with imports
 *   2. **Component declaration** — component name, slot list, node tree
 *   3. **Slot declarations** — primitive types, component types, list types, defaults
 *   4. **Node elements** — simple elements, nested elements, properties
 *   5. **Property values** — slot refs, literals, enum values, color/dimension
 *   6. **Slot references as children** — @header; @footer; etc.
 *   7. **When blocks** — conditional rendering
 *   8. **Each blocks** — iteration over list slots
 *   9. **Import declarations** — simple and aliased imports
 *  10. **Error cases** — invalid syntax throws
 */

import { describe, it, expect } from "vitest";
import { parseMosaic } from "../src/parser.js";
import type { ASTNode } from "@coding-adventures/parser";

// ============================================================================
// Helpers
// ============================================================================

/**
 * Find a child ASTNode by ruleName within a parent's children.
 */
function findRule(node: ASTNode, ruleName: string): ASTNode | undefined {
  for (const child of node.children) {
    if ("ruleName" in child && child.ruleName === ruleName) {
      return child as ASTNode;
    }
  }
  return undefined;
}

/**
 * Collect all descendant ASTNodes with a given ruleName (depth-first).
 */
function findAllRules(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  function walk(n: ASTNode) {
    if (n.ruleName === ruleName) results.push(n);
    for (const child of n.children) {
      if ("ruleName" in child) walk(child as ASTNode);
    }
  }
  walk(node);
  return results;
}

/**
 * Extract the string value of a leaf token child.
 */
function tokenValue(node: ASTNode, type: string): string | undefined {
  for (const child of node.children) {
    if ("type" in child && (child as { type: string }).type === type) {
      return (child as { value: string }).value;
    }
  }
  return undefined;
}

// ============================================================================
// 1. File Structure
// ============================================================================

describe("file structure", () => {
  it("parses a minimal component and returns a 'file' root node", () => {
    const ast = parseMosaic("component X { Row {} }");
    expect(ast.ruleName).toBe("file");
  });

  it("file node contains component_decl", () => {
    const ast = parseMosaic("component X { Row {} }");
    const decl = findRule(ast, "component_decl");
    expect(decl).toBeDefined();
  });

  it("parses without imports (no import_decl children)", () => {
    const ast = parseMosaic("component X { Row {} }");
    const imports = findAllRules(ast, "import_decl");
    expect(imports).toHaveLength(0);
  });

  it("parses a file with one import", () => {
    const src = `
      import Button from "./button.mosaic";
      component Card { Row {} }
    `;
    const ast = parseMosaic(src);
    const imports = findAllRules(ast, "import_decl");
    expect(imports).toHaveLength(1);
  });

  it("parses a file with two imports", () => {
    const src = `
      import Button from "./button.mosaic";
      import Badge from "./badge.mosaic";
      component Card { Row {} }
    `;
    const ast = parseMosaic(src);
    const imports = findAllRules(ast, "import_decl");
    expect(imports).toHaveLength(2);
  });
});

// ============================================================================
// 2. Component Declaration
// ============================================================================

describe("component declaration", () => {
  it("captures component name", () => {
    const ast = parseMosaic("component ProfileCard { Row {} }");
    const decl = findRule(ast, "component_decl")!;
    // component_decl = KEYWORD NAME ...
    const names = decl.children.filter(
      (c) => "type" in c && (c as { type: string }).type === "NAME"
    );
    expect((names[0] as { value: string }).value).toBe("ProfileCard");
  });

  it("component with zero slots is valid", () => {
    expect(() => parseMosaic("component Empty { Row {} }")).not.toThrow();
  });

  it("component with multiple slots is valid", () => {
    const src = `
      component Card {
        slot title: text;
        slot count: number;
        Row {}
      }
    `;
    expect(() => parseMosaic(src)).not.toThrow();
    const ast = parseMosaic(src);
    const slots = findAllRules(ast, "slot_decl");
    expect(slots).toHaveLength(2);
  });
});

// ============================================================================
// 3. Slot Declarations
// ============================================================================

describe("slot declarations", () => {
  const parseSlots = (body: string) => {
    const ast = parseMosaic(`component X { ${body} Row {} }`);
    return findAllRules(ast, "slot_decl");
  };

  it("slot with primitive type 'text'", () => {
    const slots = parseSlots("slot title: text;");
    expect(slots).toHaveLength(1);
    const slotType = findRule(slots[0], "slot_type");
    expect(slotType).toBeDefined();
  });

  it("slot with primitive type 'number' and default value 0", () => {
    const slots = parseSlots("slot count: number = 0;");
    expect(slots).toHaveLength(1);
    const defaultVal = findRule(slots[0], "default_value");
    expect(defaultVal).toBeDefined();
  });

  it("slot with primitive type 'bool' and default 'true'", () => {
    const slots = parseSlots("slot visible: bool = true;");
    expect(slots).toHaveLength(1);
  });

  it("slot with 'image' type", () => {
    const slots = parseSlots("slot avatar: image;");
    expect(slots).toHaveLength(1);
  });

  it("slot with 'color' type and default color hex", () => {
    const slots = parseSlots("slot bg: color = #ffffff;");
    expect(slots).toHaveLength(1);
  });

  it("slot with component type 'Button'", () => {
    const slots = parseSlots("slot action: Button;");
    expect(slots).toHaveLength(1);
  });

  it("slot with 'list<text>' type", () => {
    const slots = parseSlots("slot items: list<text>;");
    expect(slots).toHaveLength(1);
    const slotType = findRule(slots[0], "slot_type")!;
    const listType = findRule(slotType, "list_type");
    expect(listType).toBeDefined();
  });

  it("slot with 'list<Button>' (nested component type)", () => {
    const slots = parseSlots("slot buttons: list<Button>;");
    expect(slots).toHaveLength(1);
  });

  it("slot with 'node' type", () => {
    const slots = parseSlots("slot content: node;");
    expect(slots).toHaveLength(1);
  });

  it("slot with default string value", () => {
    const slots = parseSlots('slot label: text = "hello";');
    expect(slots).toHaveLength(1);
    const defaultVal = findRule(slots[0], "default_value");
    expect(defaultVal).toBeDefined();
  });

  it("slot with default dimension value", () => {
    const slots = parseSlots("slot size: number = 16dp;");
    expect(slots).toHaveLength(1);
  });
});

// ============================================================================
// 4. Node Elements
// ============================================================================

describe("node elements", () => {
  const parseNode = (body: string) => {
    return parseMosaic(`component X { ${body} }`);
  };

  it("parses a simple node with no content", () => {
    const ast = parseNode("Row {}");
    const elements = findAllRules(ast, "node_element");
    expect(elements.length).toBeGreaterThanOrEqual(1);
  });

  it("parses nested node elements", () => {
    const ast = parseNode("Column { Row { Text {} } }");
    const elements = findAllRules(ast, "node_element");
    expect(elements.length).toBeGreaterThanOrEqual(3);
  });

  it("parses a node with a property assignment", () => {
    const ast = parseNode("Text { content: @title; }");
    const props = findAllRules(ast, "property_assignment");
    expect(props).toHaveLength(1);
  });

  it("parses multiple property assignments", () => {
    const ast = parseNode("Box { padding: 16dp; background: #ffffff; }");
    const props = findAllRules(ast, "property_assignment");
    expect(props).toHaveLength(2);
  });
});

// ============================================================================
// 5. Property Values
// ============================================================================

describe("property values", () => {
  const parseProp = (propDef: string) => {
    const ast = parseMosaic(`component X { slot s: text; Row { ${propDef} } }`);
    return findAllRules(ast, "property_assignment");
  };

  it("slot_ref as property value", () => {
    const props = parseProp("content: @title;");
    expect(props).toHaveLength(1);
    const val = findRule(props[0], "property_value")!;
    expect(findRule(val, "slot_ref")).toBeDefined();
  });

  it("string literal as property value", () => {
    const props = parseProp('label: "Hello";');
    expect(props).toHaveLength(1);
  });

  it("dimension as property value", () => {
    const props = parseProp("padding: 16dp;");
    expect(props).toHaveLength(1);
  });

  it("color_hex as property value", () => {
    const props = parseProp("background: #2563eb;");
    expect(props).toHaveLength(1);
  });

  it("number as property value", () => {
    const props = parseProp("opacity: 0;");
    expect(props).toHaveLength(1);
  });

  it("keyword as property value (e.g. true/false)", () => {
    const props = parseProp("disabled: false;");
    expect(props).toHaveLength(1);
  });

  it("enum_value (NAME.NAME) as property value", () => {
    const ast = parseMosaic("component X { Row { align: center; } }");
    const props = findAllRules(ast, "property_assignment");
    expect(props.length).toBeGreaterThanOrEqual(1);
    // 'align.center' or bare NAME 'center' — both are valid property values
  });

  it("enum dot-notation (heading.small)", () => {
    const ast = parseMosaic("component X { Row { style: heading.small; } }");
    const enumVals = findAllRules(ast, "enum_value");
    expect(enumVals.length).toBeGreaterThanOrEqual(1);
  });
});

// ============================================================================
// 6. Slot References as Children
// ============================================================================

describe("slot references as children", () => {
  it("parses @header; as slot_reference", () => {
    const ast = parseMosaic("component X { slot header: node; Column { @header; } }");
    const refs = findAllRules(ast, "slot_reference");
    expect(refs.length).toBeGreaterThanOrEqual(1);
  });

  it("parses multiple slot references", () => {
    const ast = parseMosaic("component X { slot a: node; slot b: node; Column { @a; @b; } }");
    const refs = findAllRules(ast, "slot_reference");
    expect(refs.length).toBeGreaterThanOrEqual(2);
  });
});

// ============================================================================
// 7. When Blocks
// ============================================================================

describe("when blocks", () => {
  it("parses when block", () => {
    const src = `
      component X {
        slot visible: bool;
        Column {
          when @visible {
            Text { content: "Hello"; }
          }
        }
      }
    `;
    const ast = parseMosaic(src);
    const whenBlocks = findAllRules(ast, "when_block");
    expect(whenBlocks).toHaveLength(1);
  });

  it("when block contains node_content", () => {
    const src = `
      component X {
        slot show: bool;
        Row {
          when @show {
            Text {}
            Text {}
          }
        }
      }
    `;
    const ast = parseMosaic(src);
    const whenBlock = findAllRules(ast, "when_block")[0];
    expect(whenBlock).toBeDefined();
    const contents = findAllRules(whenBlock, "node_content");
    expect(contents.length).toBeGreaterThanOrEqual(2);
  });
});

// ============================================================================
// 8. Each Blocks
// ============================================================================

describe("each blocks", () => {
  it("parses each block", () => {
    const src = `
      component List {
        slot items: list<text>;
        Column {
          each @items as item {
            Text { content: @item; }
          }
        }
      }
    `;
    const ast = parseMosaic(src);
    const eachBlocks = findAllRules(ast, "each_block");
    expect(eachBlocks).toHaveLength(1);
  });

  it("each block body contains node_content", () => {
    const src = `
      component List {
        slot items: list<text>;
        Column {
          each @items as item {
            Text {}
          }
        }
      }
    `;
    const ast = parseMosaic(src);
    const eachBlock = findAllRules(ast, "each_block")[0];
    expect(eachBlock).toBeDefined();
    const contents = findAllRules(eachBlock, "node_content");
    expect(contents.length).toBeGreaterThanOrEqual(1);
  });
});

// ============================================================================
// 9. Import Declarations
// ============================================================================

describe("import declarations", () => {
  it("parses simple import", () => {
    const ast = parseMosaic('import Button from "./button.mosaic"; component X { Row {} }');
    const imports = findAllRules(ast, "import_decl");
    expect(imports).toHaveLength(1);
  });

  it("parses aliased import (import X as Y from ...)", () => {
    const ast = parseMosaic('import Card as InfoCard from "./info.mosaic"; component X { Row {} }');
    const imports = findAllRules(ast, "import_decl");
    expect(imports).toHaveLength(1);
  });
});

// ============================================================================
// 10. Error Cases
// ============================================================================

describe("error cases", () => {
  it("throws on empty input", () => {
    expect(() => parseMosaic("")).toThrow();
  });

  it("throws when missing closing brace", () => {
    expect(() => parseMosaic("component X { Row {")).toThrow();
  });

  it("throws when missing component keyword", () => {
    expect(() => parseMosaic("X { Row {} }")).toThrow();
  });
});
