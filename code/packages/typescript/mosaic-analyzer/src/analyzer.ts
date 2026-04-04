/**
 * Mosaic Analyzer — walks a Mosaic AST and produces a typed MosaicIR.
 *
 * The analyzer is the third stage of the Mosaic compiler pipeline:
 *
 *   Source text → Lexer → Tokens → Parser → ASTNode → **Analyzer** → MosaicIR
 *
 * What the Analyzer Does
 * ----------------------
 *
 * The AST produced by the parser is a faithful, unvalidated representation of
 * the source text. Every token is preserved, including keywords, semicolons,
 * and braces. The analyzer's job is to:
 *
 *   1. **Strip syntax noise** — remove KEYWORD, SEMICOLON, LBRACE/RBRACE tokens
 *      and retain only the semantically meaningful tokens.
 *   2. **Resolve types** — convert keyword strings ("text", "bool", etc.) to
 *      typed `MosaicType` discriminated union values.
 *   3. **Normalize values** — parse `"16dp"` → `{ kind: "dimension", value: 16, unit: "dp" }`,
 *      parse `"#2563eb"` → `{ kind: "color_hex", value: "#2563eb" }`, etc.
 *   4. **Determine required/optional** — slots with defaults are optional;
 *      slots without are required.
 *   5. **Identify primitives** — classify nodes as primitive (Row, Column, Text, etc.)
 *      or component (imported types).
 *
 * Note: This analyzer is **permissive by design**. It does not enforce that
 * property names are known or that slot types match property expectations.
 * The goal is to produce a valid IR for any syntactically correct `.mosaic`
 * source. Stricter validation can be layered on top.
 *
 * AST Navigation
 * --------------
 *
 * The AST uses a heterogeneous `children` array where each element is either
 * an `ASTNode` (with `.ruleName`) or a `Token` (with `.type` and `.value`).
 * The helper functions at the bottom of this file handle the discrimination.
 *
 * Primitive Nodes
 * ---------------
 *
 * Primitive nodes are the built-in layout and display elements:
 *
 *   Row, Column, Box, Stack, Text, Image, Icon, Spacer, Divider, Scroll
 *
 * All other node names are component types (imported or self-referencing).
 */

import { parseMosaic } from "@coding-adventures/mosaic-parser";
import type { ASTNode } from "@coding-adventures/parser";
import type {
  MosaicIR,
  MosaicComponent,
  MosaicImport,
  MosaicSlot,
  MosaicType,
  MosaicNode,
  MosaicChild,
  MosaicProperty,
  MosaicValue,
} from "./ir.js";

// ============================================================================
// Primitive Node Registry
// ============================================================================

/**
 * The set of built-in layout and display elements.
 *
 * When a node's tag name is in this set, `isPrimitive` is set to `true`.
 * All other names (e.g., imported component names like `Button`, `Badge`)
 * are treated as composite components with `isPrimitive: false`.
 *
 * Note: This is an open-world set — custom primitives can be added by backends
 * by inspecting the `isPrimitive` flag. The analyzer itself is authoritative
 * about these 10 standard elements.
 */
const PRIMITIVE_NODES = new Set([
  "Row", "Column", "Box", "Stack",
  "Text", "Image", "Icon",
  "Spacer", "Divider", "Scroll",
]);

// ============================================================================
// Public API
// ============================================================================

/**
 * Analyze Mosaic source text and return a typed MosaicIR.
 *
 * This is the main entry point. It parses the source, then analyzes the
 * resulting AST to produce a validated intermediate representation.
 *
 * @param source - The `.mosaic` source text.
 * @returns A typed `MosaicIR` ready for code generation.
 * @throws ParseError if the source is syntactically invalid.
 * @throws AnalysisError if the AST contains semantic errors.
 *
 * @example
 *     const ir = analyzeMosaic(`
 *       component Label {
 *         slot text: text;
 *         Text { content: @text; }
 *       }
 *     `);
 *     console.log(ir.component.name); // "Label"
 *     console.log(ir.component.slots[0].type); // { kind: "text" }
 */
export function analyzeMosaic(source: string): MosaicIR {
  const ast = parseMosaic(source);
  return analyzeFile(ast);
}

/**
 * Analyze a pre-parsed ASTNode and return a typed MosaicIR.
 *
 * Use this variant when you already have an AST and want to avoid re-parsing.
 *
 * @param ast - The root ASTNode (ruleName must be `"file"`).
 */
export function analyzeMosaicAST(ast: ASTNode): MosaicIR {
  return analyzeFile(ast);
}

// ============================================================================
// Errors
// ============================================================================

/**
 * Thrown when the analyzer encounters a structural problem in the AST.
 *
 * These are "should not happen" errors that indicate either a bug in the
 * parser or a malformed AST. Syntactic errors are caught earlier by the parser.
 */
export class AnalysisError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AnalysisError";
  }
}

// ============================================================================
// File-level Analysis
// ============================================================================

function analyzeFile(ast: ASTNode): MosaicIR {
  if (ast.ruleName !== "file") {
    throw new AnalysisError(`Expected root rule "file", got "${ast.ruleName}"`);
  }

  const imports: MosaicImport[] = [];
  let componentDecl: ASTNode | undefined;

  for (const child of ast.children) {
    if (!isNode(child)) continue;
    if (child.ruleName === "import_decl") {
      imports.push(analyzeImport(child));
    } else if (child.ruleName === "component_decl") {
      componentDecl = child;
    }
  }

  if (!componentDecl) {
    throw new AnalysisError("No component declaration found in file");
  }

  const component = analyzeComponent(componentDecl);
  return { component, imports };
}

// ============================================================================
// Import Analysis
// ============================================================================

function analyzeImport(node: ASTNode): MosaicImport {
  // import_decl = KEYWORD NAME [ KEYWORD NAME ] KEYWORD STRING SEMICOLON ;
  // Tokens in order: "import", NAME, [optional: "as", NAME], "from", STRING, ";"
  const names = tokenValues(node, "NAME");
  const strings = tokenValues(node, "STRING");

  if (names.length === 0) throw new AnalysisError("import_decl missing component name");
  if (strings.length === 0) throw new AnalysisError("import_decl missing path");

  const componentName = names[0];
  const alias = names.length >= 2 ? names[1] : undefined;
  const path = strings[0];

  return { componentName, alias, path };
}

// ============================================================================
// Component Analysis
// ============================================================================

function analyzeComponent(node: ASTNode): MosaicComponent {
  // component_decl = KEYWORD NAME LBRACE { slot_decl } node_tree RBRACE ;
  const names = tokenValues(node, "NAME");
  if (names.length === 0) throw new AnalysisError("component_decl missing name");

  const name = names[0];
  const slots: MosaicSlot[] = [];
  let treeNode: ASTNode | undefined;

  for (const child of node.children) {
    if (!isNode(child)) continue;
    if (child.ruleName === "slot_decl") {
      slots.push(analyzeSlot(child));
    } else if (child.ruleName === "node_tree") {
      treeNode = child;
    }
  }

  if (!treeNode) throw new AnalysisError(`component "${name}" has no node tree`);

  const tree = analyzeNodeTree(treeNode);
  return { name, slots, tree };
}

// ============================================================================
// Slot Analysis
// ============================================================================

function analyzeSlot(node: ASTNode): MosaicSlot {
  // slot_decl = KEYWORD NAME COLON slot_type [ EQUALS default_value ] SEMICOLON ;
  const names = tokenValues(node, "NAME");
  if (names.length === 0) throw new AnalysisError("slot_decl missing name");

  const name = names[0];
  const slotTypeNode = findChild(node, "slot_type");
  if (!slotTypeNode) throw new AnalysisError(`slot "${name}" missing type`);

  const type = analyzeSlotType(slotTypeNode);
  const defaultValueNode = findChild(node, "default_value");
  const defaultValue = defaultValueNode ? analyzeDefaultValue(defaultValueNode) : undefined;
  const required = defaultValue === undefined;

  return { name, type, defaultValue, required };
}

function analyzeSlotType(node: ASTNode): MosaicType {
  // slot_type = KEYWORD | NAME | list_type
  const listTypeNode = findChild(node, "list_type");
  if (listTypeNode) return analyzeListType(listTypeNode);

  // KEYWORD or NAME directly under slot_type
  const keyword = firstTokenValue(node, "KEYWORD");
  if (keyword) return parsePrimitiveType(keyword);

  const name = firstTokenValue(node, "NAME");
  if (name) return { kind: "component", name };

  throw new AnalysisError("slot_type has no recognizable content");
}

function analyzeListType(node: ASTNode): MosaicType {
  // list_type = KEYWORD LANGLE slot_type RANGLE
  const elementTypeNode = findChild(node, "slot_type");
  if (!elementTypeNode) throw new AnalysisError("list_type missing element type");

  const elementType = analyzeSlotType(elementTypeNode);
  return { kind: "list", elementType };
}

function parsePrimitiveType(keyword: string): MosaicType {
  switch (keyword) {
    case "text":   return { kind: "text" };
    case "number": return { kind: "number" };
    case "bool":   return { kind: "bool" };
    case "image":  return { kind: "image" };
    case "color":  return { kind: "color" };
    case "node":   return { kind: "node" };
    default:
      throw new AnalysisError(`Unknown primitive type keyword: "${keyword}"`);
  }
}

function analyzeDefaultValue(node: ASTNode): MosaicValue {
  // default_value = STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD
  const str = firstTokenValue(node, "STRING");
  if (str !== undefined) return { kind: "string", value: str };

  const dim = firstTokenValue(node, "DIMENSION");
  if (dim !== undefined) return parseDimension(dim);

  const num = firstTokenValue(node, "NUMBER");
  if (num !== undefined) return { kind: "number", value: parseFloat(num) };

  const color = firstTokenValue(node, "COLOR_HEX");
  if (color !== undefined) return { kind: "color_hex", value: color };

  const kw = firstTokenValue(node, "KEYWORD");
  if (kw === "true") return { kind: "bool", value: true };
  if (kw === "false") return { kind: "bool", value: false };

  throw new AnalysisError("default_value has no recognizable content");
}

// ============================================================================
// Node Tree Analysis
// ============================================================================

function analyzeNodeTree(node: ASTNode): MosaicNode {
  // node_tree = node_element
  const element = findChild(node, "node_element");
  if (!element) throw new AnalysisError("node_tree missing node_element");
  return analyzeNodeElement(element);
}

function analyzeNodeElement(node: ASTNode): MosaicNode {
  // node_element = NAME LBRACE { node_content } RBRACE
  const tag = firstTokenValue(node, "NAME");
  if (!tag) throw new AnalysisError("node_element missing tag name");

  const isPrimitive = PRIMITIVE_NODES.has(tag);
  const properties: MosaicProperty[] = [];
  const children: MosaicChild[] = [];

  for (const child of node.children) {
    if (!isNode(child)) continue;
    if (child.ruleName === "node_content") {
      const { prop, childItem } = analyzeNodeContent(child);
      if (prop) properties.push(prop);
      if (childItem) children.push(childItem);
    }
  }

  return { tag, isPrimitive, properties, children };
}

function analyzeNodeContent(
  node: ASTNode
): { prop?: MosaicProperty; childItem?: MosaicChild } {
  // node_content = property_assignment | child_node | slot_reference | when_block | each_block
  for (const child of node.children) {
    if (!isNode(child)) continue;

    if (child.ruleName === "property_assignment") {
      return { prop: analyzePropertyAssignment(child) };
    }
    if (child.ruleName === "child_node") {
      const element = findChild(child, "node_element");
      if (element) return { childItem: { kind: "node", node: analyzeNodeElement(element) } };
    }
    if (child.ruleName === "slot_reference") {
      const name = firstTokenValue(child, "NAME");
      if (name) return { childItem: { kind: "slot_ref", slotName: name } };
    }
    if (child.ruleName === "when_block") {
      return { childItem: analyzeWhenBlock(child) };
    }
    if (child.ruleName === "each_block") {
      return { childItem: analyzeEachBlock(child) };
    }
  }
  return {};
}

// ============================================================================
// Property Analysis
// ============================================================================

function analyzePropertyAssignment(node: ASTNode): MosaicProperty {
  // property_assignment = (NAME | KEYWORD) COLON property_value SEMICOLON
  // Property names may be NAME tokens or KEYWORD tokens (e.g., "color", "node").
  const name = firstTokenValue(node, "NAME") ?? firstTokenValue(node, "KEYWORD");
  if (!name) throw new AnalysisError("property_assignment missing name");

  const valueNode = findChild(node, "property_value");
  if (!valueNode) throw new AnalysisError(`property "${name}" missing value`);

  const value = analyzePropertyValue(valueNode);
  return { name, value };
}

function analyzePropertyValue(node: ASTNode): MosaicValue {
  // property_value = slot_ref | STRING | DIMENSION | NUMBER | COLOR_HEX | KEYWORD | enum_value | NAME

  // Check for child rule nodes first.
  for (const child of node.children) {
    if (!isNode(child)) continue;

    if (child.ruleName === "slot_ref") {
      const name = firstTokenValue(child, "NAME");
      if (name) return { kind: "slot_ref", slotName: name };
    }
    if (child.ruleName === "enum_value") {
      const names = tokenValues(child, "NAME");
      if (names.length >= 2) return { kind: "enum", namespace: names[0], member: names[1] };
    }
  }

  // Leaf tokens.
  const str = firstTokenValue(node, "STRING");
  if (str !== undefined) return { kind: "string", value: str };

  const dim = firstTokenValue(node, "DIMENSION");
  if (dim !== undefined) return parseDimension(dim);

  const num = firstTokenValue(node, "NUMBER");
  if (num !== undefined) return { kind: "number", value: parseFloat(num) };

  const color = firstTokenValue(node, "COLOR_HEX");
  if (color !== undefined) return { kind: "color_hex", value: color };

  const kw = firstTokenValue(node, "KEYWORD");
  if (kw === "true") return { kind: "bool", value: true };
  if (kw === "false") return { kind: "bool", value: false };
  if (kw !== undefined) return { kind: "ident", value: kw };

  const ident = firstTokenValue(node, "NAME");
  if (ident !== undefined) return { kind: "ident", value: ident };

  throw new AnalysisError("property_value has no recognizable content");
}

// ============================================================================
// When / Each Block Analysis
// ============================================================================

function analyzeWhenBlock(node: ASTNode): MosaicChild & { kind: "when" } {
  // when_block = KEYWORD slot_ref LBRACE { node_content } RBRACE
  const slotRefNode = findChild(node, "slot_ref");
  if (!slotRefNode) throw new AnalysisError("when_block missing slot_ref");

  const slotName = firstTokenValue(slotRefNode, "NAME");
  if (!slotName) throw new AnalysisError("when_block slot_ref missing name");

  const children = analyzeNodeContents(node);
  return { kind: "when", slotName, children };
}

function analyzeEachBlock(node: ASTNode): MosaicChild & { kind: "each" } {
  // each_block = KEYWORD slot_ref KEYWORD NAME LBRACE { node_content } RBRACE
  const slotRefNode = findChild(node, "slot_ref");
  if (!slotRefNode) throw new AnalysisError("each_block missing slot_ref");

  const slotName = firstTokenValue(slotRefNode, "NAME");
  if (!slotName) throw new AnalysisError("each_block slot_ref missing name");

  // The loop variable is the NAME token after the "as" keyword.
  // In the each_block AST, we skip the slot_ref's NAMEs and find the loop var.
  // The each_block children include: KEYWORD(each), slot_ref, KEYWORD(as), NAME(item), ...
  // We need to find the NAME that is NOT inside the slot_ref.
  const itemName = findLoopVariable(node, slotRefNode);
  if (!itemName) throw new AnalysisError("each_block missing loop variable name");

  const children = analyzeNodeContents(node);
  return { kind: "each", slotName, itemName, children };
}

/**
 * Find the loop variable NAME in an each_block.
 *
 * The each_block has this structure:
 *   KEYWORD(each)  slot_ref  KEYWORD(as)  NAME(item)  LBRACE  ...  RBRACE
 *
 * We need to find the NAME token that is a direct child of each_block (not
 * inside the slot_ref sub-tree).
 */
function findLoopVariable(eachBlock: ASTNode, slotRef: ASTNode): string | undefined {
  let afterAs = false;
  for (const child of eachBlock.children) {
    // Skip the slot_ref subtree entirely.
    if (isNode(child) && child === slotRef) continue;
    if (isNode(child) && child.ruleName === "slot_ref") continue;

    if (!isNode(child)) {
      const token = child as { type: string; value: string };
      if (token.type === "KEYWORD" && token.value === "as") {
        afterAs = true;
        continue;
      }
      if (afterAs && token.type === "NAME") {
        return token.value;
      }
    }
  }
  return undefined;
}

/**
 * Collect all node_content children from a block (when/each/node_element).
 */
function analyzeNodeContents(node: ASTNode): MosaicChild[] {
  const children: MosaicChild[] = [];
  for (const child of node.children) {
    if (!isNode(child)) continue;
    if (child.ruleName === "node_content") {
      const { childItem } = analyzeNodeContent(child);
      if (childItem) children.push(childItem);
    }
  }
  return children;
}

// ============================================================================
// Value Parsing Helpers
// ============================================================================

/**
 * Parse a DIMENSION token value like "16dp" or "100%" into a structured object.
 *
 * The lexer ensures a DIMENSION is always a number followed by a unit suffix.
 * We split at the first non-digit/dot/minus character to separate value from unit.
 *
 * Examples:
 *   "16dp"  → { kind: "dimension", value: 16, unit: "dp" }
 *   "1.5sp" → { kind: "dimension", value: 1.5, unit: "sp" }
 *   "100%"  → { kind: "dimension", value: 100, unit: "%" }
 */
function parseDimension(raw: string): MosaicValue & { kind: "dimension" } {
  const match = raw.match(/^(-?[0-9]*\.?[0-9]+)([a-zA-Z%]+)$/);
  if (!match) throw new AnalysisError(`Invalid DIMENSION token: "${raw}"`);
  return { kind: "dimension", value: parseFloat(match[1]), unit: match[2] };
}

// ============================================================================
// AST Traversal Helpers
// ============================================================================

type ASTChild = ASTNode | { type: string; value: string };

function isNode(child: ASTChild): child is ASTNode {
  return "ruleName" in child;
}

/** Find the first direct child ASTNode with the given ruleName. */
function findChild(node: ASTNode, ruleName: string): ASTNode | undefined {
  for (const child of node.children) {
    if (isNode(child) && child.ruleName === ruleName) return child;
  }
  return undefined;
}

/** Collect all direct-child token values with the given token type. */
function tokenValues(node: ASTNode, tokenType: string): string[] {
  const result: string[] = [];
  for (const child of node.children) {
    if (!isNode(child)) {
      const token = child as { type: string; value: string };
      if (token.type === tokenType) result.push(token.value);
    }
  }
  return result;
}

/** Get the first direct-child token value with the given token type, or undefined. */
function firstTokenValue(node: ASTNode, tokenType: string): string | undefined {
  for (const child of node.children) {
    if (!isNode(child)) {
      const token = child as { type: string; value: string };
      if (token.type === tokenType) return token.value;
    }
  }
  return undefined;
}
