/**
 * CSS Emitter — Reconstructs CSS text from a clean AST.
 *
 * After the transformer has expanded all Lattice nodes (variables, mixins,
 * control flow, functions), the AST contains only pure CSS nodes:
 *
 *   stylesheet          — the root
 *   qualified_rule      — selector + block (e.g., h1 { color: red; })
 *   at_rule             — @-rules (e.g., @media, @import)
 *   selector_list       — comma-separated selectors
 *   complex_selector    — compound selectors with combinators
 *   compound_selector   — type/class/id/pseudo selectors
 *   block               — { declarations }
 *   declaration         — property: value;
 *   value_list          — space-separated values
 *   function_call       — rgb(255, 0, 0)
 *   priority            — !important
 *
 * The emitter walks this tree and produces formatted CSS text.
 *
 * How It Works
 * ------------
 *
 * The emitter dispatches on ruleName. Each rule has a handler method that
 * knows how to format that particular CSS construct. Unknown rules fall
 * through to a default handler that recurses into children.
 *
 * Two formatting modes are supported:
 *
 * - Pretty-print (default): 2-space indentation, newlines between
 *   declarations, blank lines between rules.
 *
 * - Minified: No unnecessary whitespace. Every byte counts for production.
 *
 * Design Note
 * -----------
 *
 * The emitter assumes the AST is clean — no Lattice nodes remain. If a
 * Lattice node is encountered, it's silently skipped. The transformer is
 * responsible for removing all Lattice nodes before the emitter runs.
 */

import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

// =============================================================================
// Type Guards
// =============================================================================

/** Check if a child is an ASTNode (not a Token). */
function isASTNode(child: ASTNode | Token): child is ASTNode {
  return "ruleName" in child;
}

/** Get the token type as a string. */
function tokenType(token: Token): string {
  return token.type as string;
}

// =============================================================================
// CSSEmitter
// =============================================================================

/**
 * Emits CSS text from a clean AST.
 *
 * The emitter walks the AST recursively, dispatching on ruleName to produce
 * properly formatted CSS output.
 *
 * @param indent - The indentation string per level (default: "  " = 2 spaces).
 * @param minified - If true, emit minified CSS with no unnecessary whitespace.
 */
export class CSSEmitter {
  private readonly indent: string;
  private readonly minified: boolean;

  constructor(indent: string = "  ", minified: boolean = false) {
    this.indent = indent;
    this.minified = minified;
  }

  /**
   * Emit CSS text from an AST node.
   *
   * This is the main entry point. Pass the root stylesheet node and get
   * back a complete CSS string.
   *
   * @param node - An ASTNode (typically the root "stylesheet").
   * @returns Formatted CSS text.
   */
  emit(node: ASTNode): string {
    const result = this._emitNode(node, 0);
    const trimmed = result.trim();
    return trimmed ? trimmed + "\n" : "";
  }

  // ---------------------------------------------------------------------------
  // Internal Dispatch
  // ---------------------------------------------------------------------------

  /**
   * Dispatch to the appropriate handler based on ruleName.
   *
   * If the node is a token (no ruleName), return its text value.
   * If the ruleName has a specific handler, use it. Otherwise,
   * fall through to the default handler.
   */
  private _emitNode(node: ASTNode | Token, depth: number): string {
    // Raw token — return its value
    if (!isASTNode(node)) {
      return (node as Token).value;
    }

    const ast = node as ASTNode;
    const rule = ast.ruleName;

    // Dispatch to specific handler
    switch (rule) {
      case "stylesheet":
        return this._emitStylesheet(ast, depth);
      case "rule":
        return this._emitRule(ast, depth);
      case "qualified_rule":
        return this._emitQualifiedRule(ast, depth);
      case "at_rule":
        return this._emitAtRule(ast, depth);
      case "at_prelude":
        return this._emitAtPrelude(ast, depth);
      case "at_prelude_token":
        return this._emitDefault(ast, depth);
      case "at_prelude_tokens":
        return this._emitAtPreludeTokens(ast, depth);
      case "function_in_prelude":
        return this._emitFunctionInPrelude(ast, depth);
      case "paren_block":
        return this._emitParenBlock(ast, depth);
      case "selector_list":
        return this._emitSelectorList(ast, depth);
      case "complex_selector":
        return this._emitComplexSelector(ast, depth);
      case "combinator":
        return this._emitCombinator(ast, depth);
      case "compound_selector":
        return this._emitCompoundSelector(ast, depth);
      case "simple_selector":
        return this._emitSimpleSelector(ast, depth);
      case "subclass_selector":
        return this._emitSubclassSelector(ast, depth);
      case "class_selector":
        return this._emitClassSelector(ast, depth);
      case "id_selector":
        return this._emitIdSelector(ast, depth);
      case "attribute_selector":
        return this._emitAttributeSelector(ast, depth);
      case "attr_matcher":
        return this._emitAttrMatcher(ast, depth);
      case "attr_value":
        return this._emitAttrValue(ast, depth);
      case "pseudo_class":
        return this._emitPseudoClass(ast, depth);
      case "pseudo_class_args":
        return this._emitPseudoClassArgs(ast, depth);
      case "pseudo_class_arg":
        return this._emitDefault(ast, depth);
      case "pseudo_element":
        return this._emitPseudoElement(ast, depth);
      case "block":
        return this._emitBlock(ast, depth);
      case "block_contents":
        return this._emitBlockContents(ast, depth);
      case "block_item":
        return this._emitBlockItem(ast, depth);
      case "declaration_or_nested":
        return this._emitDeclarationOrNested(ast, depth);
      case "declaration":
        return this._emitDeclaration(ast, depth);
      case "property":
        return this._emitProperty(ast, depth);
      case "priority":
        return this._emitPriority(ast, depth);
      case "value_list":
        return this._emitValueList(ast, depth);
      case "value":
        return this._emitValue(ast, depth);
      case "function_call":
        return this._emitFunctionCall(ast, depth);
      case "function_args":
        return this._emitFunctionArgs(ast, depth);
      case "function_arg":
        return this._emitFunctionArg(ast, depth);
      default:
        return this._emitDefault(ast, depth);
    }
  }

  // ---------------------------------------------------------------------------
  // Top-Level Structure
  // ---------------------------------------------------------------------------

  /**
   * stylesheet = { rule } ;
   *
   * Join rules with blank lines (pretty) or nothing (minified).
   */
  private _emitStylesheet(node: ASTNode, depth: number): string {
    const parts: string[] = [];

    for (const child of node.children) {
      const text = this._emitNode(child, depth);
      if (text.trim()) {
        parts.push(text);
      }
    }

    if (this.minified) return parts.join("");
    return parts.join("\n\n");
  }

  /**
   * rule = lattice_rule | at_rule | qualified_rule ;
   *
   * A rule is a wrapper — just emit the single child.
   */
  private _emitRule(node: ASTNode, depth: number): string {
    const children = node.children;
    if (children.length > 0) {
      return this._emitNode(children[0], depth);
    }
    return "";
  }

  // ---------------------------------------------------------------------------
  // Qualified Rules (selector + block)
  // ---------------------------------------------------------------------------

  /**
   * qualified_rule = selector_list block ;
   *
   * Emits:
   *   selector_list {
   *     declarations...
   *   }
   */
  private _emitQualifiedRule(node: ASTNode, depth: number): string {
    let selector = "";
    let block = "";

    for (const child of node.children) {
      if (!isASTNode(child)) continue;
      const childAst = child as ASTNode;
      if (childAst.ruleName === "selector_list") {
        selector = this._emitNode(child, depth);
      } else if (childAst.ruleName === "block") {
        block = this._emitBlock(childAst, depth);
      } else {
        // Other children
        const text = this._emitNode(child, depth);
        if (text.trim()) selector += text;
      }
    }

    if (this.minified) {
      return `${selector}${block}`;
    }
    return selector ? `${selector} ${block}` : block;
  }

  // ---------------------------------------------------------------------------
  // At-Rules
  // ---------------------------------------------------------------------------

  /**
   * at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block ) ;
   *
   * Two forms:
   *   @import url("style.css");
   *   @media (max-width: 768px) { ... }
   */
  private _emitAtRule(node: ASTNode, depth: number): string {
    let keyword = "";
    let prelude = "";
    let blockText = "";
    let hasSemicolon = false;

    for (const child of node.children) {
      if (!isASTNode(child)) {
        const tok = child as Token;
        const type = tokenType(tok);
        if (type === "AT_KEYWORD") {
          keyword = tok.value;
        } else if (type === "SEMICOLON") {
          hasSemicolon = true;
        }
      } else {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "at_prelude") {
          prelude = this._emitAtPrelude(childAst, depth);
        } else if (childAst.ruleName === "block") {
          blockText = this._emitBlock(childAst, depth);
        }
      }
    }

    if (this.minified) {
      if (hasSemicolon) return `${keyword}${prelude};`;
      return `${keyword}${prelude}${blockText}`;
    }

    if (hasSemicolon) {
      const preludePart = prelude.trim() ? ` ${prelude.trim()}` : "";
      return `${keyword}${preludePart};`;
    }
    const preludePart = prelude.trim() ? ` ${prelude.trim()}` : "";
    return `${keyword}${preludePart} ${blockText}`;
  }

  /**
   * at_prelude = { at_prelude_token } ;
   *
   * Collect tokens and nodes, space-separate them.
   */
  private _emitAtPrelude(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      parts.push(this._emitNode(child, depth));
    }
    return parts.join(" ");
  }

  private _emitAtPreludeTokens(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      parts.push(this._emitNode(child, depth));
    }
    return parts.join(" ");
  }

  private _emitFunctionInPrelude(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      if (!isASTNode(child)) {
        const tok = child as Token;
        if (tokenType(tok) === "RPAREN") {
          parts.push(")");
        } else {
          parts.push(tok.value);
        }
      } else {
        parts.push(this._emitNode(child, depth));
      }
    }
    return parts.join("");
  }

  private _emitParenBlock(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      if (!isASTNode(child)) {
        const tok = child as Token;
        const type = tokenType(tok);
        if (type === "LPAREN") {
          parts.push("(");
        } else if (type === "RPAREN") {
          parts.push(")");
        } else {
          parts.push(tok.value);
        }
      } else {
        parts.push(this._emitNode(child, depth));
      }
    }
    return parts.join("");
  }

  // ---------------------------------------------------------------------------
  // Selectors
  // ---------------------------------------------------------------------------

  /**
   * selector_list = complex_selector { COMMA complex_selector } ;
   *
   * Comma-separate selectors.
   */
  private _emitSelectorList(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      if (!isASTNode(child)) {
        // Skip COMMA tokens — we add commas ourselves
        continue;
      }
      parts.push(this._emitNode(child, depth));
    }
    const sep = this.minified ? "," : ", ";
    return parts.join(sep);
  }

  /**
   * complex_selector = compound_selector { [ combinator ] compound_selector } ;
   */
  private _emitComplexSelector(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      parts.push(this._emitNode(child, depth));
    }
    return parts.join(" ");
  }

  /** combinator = GREATER | PLUS | TILDE ; */
  private _emitCombinator(node: ASTNode, _depth: number): string {
    if (node.children.length > 0) {
      return (node.children[0] as Token).value;
    }
    return "";
  }

  /**
   * compound_selector = simple_selector { subclass_selector }
   *                   | subclass_selector { subclass_selector } ;
   *
   * Concatenate without spaces: h1.classname#id
   */
  private _emitCompoundSelector(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      parts.push(this._emitNode(child, depth));
    }
    return parts.join("");
  }

  /** simple_selector = IDENT | STAR | AMPERSAND ; */
  private _emitSimpleSelector(node: ASTNode, _depth: number): string {
    if (node.children.length > 0) {
      return (node.children[0] as Token).value;
    }
    return "";
  }

  /** subclass_selector — dispatch to child. */
  private _emitSubclassSelector(node: ASTNode, depth: number): string {
    if (node.children.length > 0) {
      return this._emitNode(node.children[0], depth);
    }
    return "";
  }

  /** class_selector = DOT IDENT ; */
  private _emitClassSelector(node: ASTNode, _depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      if (!isASTNode(child)) {
        parts.push((child as Token).value);
      }
    }
    return parts.join("");
  }

  /** id_selector = HASH ; */
  private _emitIdSelector(node: ASTNode, _depth: number): string {
    if (node.children.length > 0) {
      return (node.children[0] as Token).value;
    }
    return "";
  }

  /**
   * attribute_selector = LBRACKET IDENT [ attr_matcher attr_value [ IDENT ] ] RBRACKET ;
   */
  private _emitAttributeSelector(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      if (!isASTNode(child)) {
        const tok = child as Token;
        const type = tokenType(tok);
        if (type === "LBRACKET") {
          parts.push("[");
        } else if (type === "RBRACKET") {
          parts.push("]");
        } else {
          parts.push(tok.value);
        }
      } else {
        parts.push(this._emitNode(child, depth));
      }
    }
    return parts.join("");
  }

  /** attr_matcher = EQUALS | TILDE_EQUALS | ... ; */
  private _emitAttrMatcher(node: ASTNode, _depth: number): string {
    if (node.children.length > 0) {
      return (node.children[0] as Token).value;
    }
    return "";
  }

  /** attr_value = IDENT | STRING ; */
  private _emitAttrValue(node: ASTNode, _depth: number): string {
    if (node.children.length > 0) {
      const child = node.children[0] as Token;
      if (tokenType(child) === "STRING") {
        return `"${child.value}"`;
      }
      return child.value;
    }
    return "";
  }

  /**
   * pseudo_class = COLON FUNCTION pseudo_class_args RPAREN | COLON IDENT ;
   */
  private _emitPseudoClass(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      if (!isASTNode(child)) {
        const tok = child as Token;
        const type = tokenType(tok);
        if (type === "COLON") {
          parts.push(":");
        } else if (type === "RPAREN") {
          parts.push(")");
        } else {
          parts.push(tok.value);
        }
      } else {
        parts.push(this._emitNode(child, depth));
      }
    }
    return parts.join("");
  }

  private _emitPseudoClassArgs(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      parts.push(this._emitNode(child, depth));
    }
    return parts.join("");
  }

  /** pseudo_element = COLON_COLON IDENT ; */
  private _emitPseudoElement(node: ASTNode, _depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      if (!isASTNode(child)) {
        const tok = child as Token;
        if (tokenType(tok) === "COLON_COLON") {
          parts.push("::");
        } else {
          parts.push(tok.value);
        }
      }
    }
    return parts.join("");
  }

  // ---------------------------------------------------------------------------
  // Blocks and Declarations
  // ---------------------------------------------------------------------------

  /**
   * block = LBRACE block_contents RBRACE ;
   *
   * Emits { declarations } with proper indentation.
   */
  private _emitBlock(node: ASTNode, depth: number): string {
    // Find block_contents node
    let contents: ASTNode | undefined;
    for (const child of node.children) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "block_contents") {
        contents = child as ASTNode;
        break;
      }
    }

    if (this.minified) {
      if (!contents) return "{}";
      const inner = this._emitBlockContents(contents, depth + 1);
      return "{" + inner + "}";
    }

    if (!contents) {
      return "{\n" + this.indent.repeat(depth) + "}";
    }

    const inner = this._emitBlockContents(contents, depth + 1);
    if (!inner.trim()) {
      return "{\n" + this.indent.repeat(depth) + "}";
    }
    return "{\n" + inner + "\n" + this.indent.repeat(depth) + "}";
  }

  /** block_contents = { block_item } ; */
  private _emitBlockContents(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      const text = this._emitNode(child, depth);
      if (text.trim()) {
        parts.push(text);
      }
    }

    if (this.minified) return parts.join("");

    const prefix = this.indent.repeat(depth);
    return parts.map((p) => `${prefix}${p}`).join("\n");
  }

  /** block_item = lattice_block_item | at_rule | declaration_or_nested ; */
  private _emitBlockItem(node: ASTNode, depth: number): string {
    if (node.children.length > 0) {
      return this._emitNode(node.children[0], depth);
    }
    return "";
  }

  /** declaration_or_nested = declaration | qualified_rule ; */
  private _emitDeclarationOrNested(node: ASTNode, depth: number): string {
    if (node.children.length > 0) {
      return this._emitNode(node.children[0], depth);
    }
    return "";
  }

  /**
   * declaration = property COLON value_list [ priority ] SEMICOLON ;
   *
   * Emits: property: value_list;
   * or:    property: value_list !important;
   */
  private _emitDeclaration(node: ASTNode, _depth: number): string {
    let prop = "";
    let value = "";
    let priority = "";

    for (const child of node.children) {
      if (!isASTNode(child)) continue; // Skip COLON and SEMICOLON tokens

      const childAst = child as ASTNode;
      if (childAst.ruleName === "property") {
        prop = this._emitProperty(childAst, 0);
      } else if (childAst.ruleName === "value_list") {
        value = this._emitValueList(childAst, 0);
      } else if (childAst.ruleName === "priority") {
        priority = " !important";
      }
    }

    if (this.minified) {
      return `${prop}:${value}${priority};`;
    }
    return `${prop}: ${value}${priority};`;
  }

  /** property = IDENT | CUSTOM_PROPERTY ; */
  private _emitProperty(node: ASTNode, _depth: number): string {
    if (node.children.length > 0) {
      return (node.children[0] as Token).value;
    }
    return "";
  }

  /** priority = BANG "important" ; */
  private _emitPriority(_node: ASTNode, _depth: number): string {
    return "!important";
  }

  // ---------------------------------------------------------------------------
  // Values
  // ---------------------------------------------------------------------------

  /**
   * value_list = value { value } ;
   *
   * Space-separate values, but commas don't need extra spaces.
   */
  private _emitValueList(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      const text = this._emitNode(child, depth);
      parts.push(text);
    }

    let result = parts.join(" ");
    // Collapse spaces around commas
    result = result.replace(/ , /g, ", ").replace(/ ,/g, ",");
    return result;
  }

  /**
   * value = DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT | HASH | ... ;
   */
  private _emitValue(node: ASTNode, depth: number): string {
    const children = node.children;
    if (children.length === 1) {
      const child = children[0];
      if (!isASTNode(child)) {
        const tok = child as Token;
        if (tokenType(tok) === "STRING") {
          return `"${tok.value}"`;
        }
        return tok.value;
      }
      return this._emitNode(child, depth);
    }
    return this._emitDefault(node, depth);
  }

  /**
   * function_call = FUNCTION function_args RPAREN | URL_TOKEN ;
   */
  private _emitFunctionCall(node: ASTNode, depth: number): string {
    const children = node.children;

    if (children.length === 1) {
      // URL_TOKEN
      return (children[0] as Token).value;
    }

    const parts: string[] = [];
    for (const child of children) {
      if (!isASTNode(child)) {
        const tok = child as Token;
        const type = tokenType(tok);
        if (type === "FUNCTION") {
          parts.push(tok.value); // Includes "("
        } else if (type === "RPAREN") {
          parts.push(")");
        } else {
          parts.push(tok.value);
        }
      } else {
        parts.push(this._emitNode(child, depth));
      }
    }
    return parts.join("");
  }

  /** function_args = { function_arg } ; */
  private _emitFunctionArgs(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      parts.push(this._emitNode(child, depth));
    }
    let result = parts.join(" ");
    result = result.replace(/ , /g, ", ").replace(/ ,/g, ",");
    return result;
  }

  /** Single argument in a function call. */
  private _emitFunctionArg(node: ASTNode, depth: number): string {
    const children = node.children;
    if (children.length === 1) {
      const child = children[0];
      if (!isASTNode(child)) {
        return (child as Token).value;
      }
      return this._emitNode(child, depth);
    }
    return this._emitDefault(node, depth);
  }

  // ---------------------------------------------------------------------------
  // Default and Utilities
  // ---------------------------------------------------------------------------

  /**
   * Default handler: concatenate children with spaces.
   *
   * Used for unknown rule names and as a fallback.
   */
  private _emitDefault(node: ASTNode, depth: number): string {
    const parts: string[] = [];
    for (const child of node.children) {
      parts.push(this._emitNode(child, depth));
    }
    return parts.join(" ");
  }
}
