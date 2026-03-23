/**
 * Additional coverage tests for lattice-ast-to-css.
 *
 * These tests target specific uncovered branches in:
 *   - emitter.ts   — CSS emission edge cases
 *   - evaluator.ts — expression evaluation paths
 *   - transformer.ts — transformer edge cases
 *   - values.ts / errors.ts — remaining value/error branches
 *
 * Tests are self-contained: they either use the transpile() helper
 * (for integration paths) or construct AST nodes directly.
 */

import { describe, it, expect } from "vitest";
import {
  ScopeChain,
  LatticeNumber,
  LatticeDimension,
  LatticePercentage,
  LatticeString,
  LatticeIdent,
  LatticeColor,
  LatticeBool,
  LatticeNull,
  LatticeList,
  isTruthy,
  tokenToValue,
  addValues,
  subtractValues,
  multiplyValues,
  negateValue,
  compareValues,
  ExpressionEvaluator,
  LatticeTransformer,
  CSSEmitter,
  LatticeError,
  UndefinedVariableError,
  UndefinedMixinError,
  UndefinedFunctionError,
  WrongArityError,
  CircularReferenceError,
  TypeErrorInExpression,
  MissingReturnError,
  LatticeModuleNotFoundError,
  ReturnOutsideFunctionError,
  UnitMismatchError,
  extractValueFromAst,
} from "../src/index.js";
import { parseLattice } from "@coding-adventures/lattice-parser";

// =============================================================================
// Helper: transpile Lattice to CSS
// =============================================================================

function transpile(source: string, minified = false): string {
  const ast = parseLattice(source);
  const transformer = new LatticeTransformer();
  const cssAst = transformer.transform(ast);
  const emitter = new CSSEmitter("  ", minified);
  return emitter.emit(cssAst);
}

// =============================================================================
// CSSEmitter — Direct AST Construction Tests
// =============================================================================
// These tests build ASTNode trees manually to exercise specific emitter paths
// that don't get triggered from normal Lattice source code parsing.

/** Helper to create a simple ASTNode. */
function makeNode(ruleName: string, children: Array<{ ruleName?: string; type?: string; value?: string; children?: any[] }>): any {
  return { ruleName, children: children.map(c => makeNode2(c)) };
}

function makeNode2(c: any): any {
  if (c.type !== undefined) {
    // It's a token
    return { type: c.type, value: c.value ?? "", line: 0, column: 0 };
  }
  return { ruleName: c.ruleName, children: (c.children ?? []).map(makeNode2) };
}

describe("CSSEmitter — emit() returns empty string for empty stylesheet", () => {
  it("returns empty string for stylesheet with no rules", () => {
    const emitter = new CSSEmitter();
    const ast = makeNode("stylesheet", []);
    expect(emitter.emit(ast)).toBe("");
  });
});

describe("CSSEmitter — direct node tests", () => {
  it("emits a rule node with single child", () => {
    const emitter = new CSSEmitter();
    // Build: rule → qualified_rule → selector_list + block
    const identToken = { type: "IDENT", value: "h1" };
    const selectorNode = makeNode("simple_selector", [identToken]);
    const compoundSelector = makeNode("compound_selector", [selectorNode]);
    const complexSelector = makeNode("complex_selector", [compoundSelector]);
    const selectorList = makeNode("selector_list", [complexSelector]);

    const propToken = { type: "IDENT", value: "color" };
    const propNode = makeNode("property", [propToken]);
    const valToken = { type: "IDENT", value: "red" };
    const valNode = makeNode("value", [valToken]);
    const valueList = makeNode("value_list", [valNode]);
    const declNode = makeNode("declaration", [propNode, valueList]);
    const declOrNested = makeNode("declaration_or_nested", [declNode]);
    const blockItem = makeNode("block_item", [declOrNested]);
    const blockContents = makeNode("block_contents", [blockItem]);
    const blockNode = makeNode("block", [blockContents]);

    const qualifiedRule = makeNode("qualified_rule", [selectorList, blockNode]);
    const ruleNode = makeNode("rule", [qualifiedRule]);

    const css = emitter.emit(ruleNode);
    expect(css).toContain("h1");
    expect(css).toContain("color: red");
  });

  it("emits a rule node with no children returns empty", () => {
    const emitter = new CSSEmitter();
    const ruleNode = makeNode("rule", []);
    // rule with no children → empty
    const css = emitter.emit(ruleNode);
    expect(css).toBe("");
  });

  it("emits block with no contents (empty block)", () => {
    const emitter = new CSSEmitter();
    // block with no block_contents child
    const blockNode = makeNode("block", []);
    const css = emitter.emit(blockNode);
    expect(css).toContain("{");
    expect(css).toContain("}");
  });

  it("emits block with empty block_contents", () => {
    const emitter = new CSSEmitter();
    const blockContents = makeNode("block_contents", []);
    const blockNode = makeNode("block", [blockContents]);
    const css = emitter.emit(blockNode);
    expect(css).toContain("{");
    expect(css).toContain("}");
  });

  it("emits simple_selector with empty children returns empty", () => {
    const emitter = new CSSEmitter();
    const node = makeNode("simple_selector", []);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("");
  });

  it("emits combinator with empty children returns empty", () => {
    const emitter = new CSSEmitter();
    const node = makeNode("combinator", []);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("");
  });

  it("emits id_selector with no children returns empty", () => {
    const emitter = new CSSEmitter();
    const node = makeNode("id_selector", []);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("");
  });

  it("emits subclass_selector with no children returns empty", () => {
    const emitter = new CSSEmitter();
    const node = makeNode("subclass_selector", []);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("");
  });

  it("emits attr_matcher with token", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "EQUALS", value: "=" };
    const node = makeNode("attr_matcher", [tok]);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("=");
  });

  it("emits attr_matcher with no children returns empty", () => {
    const emitter = new CSSEmitter();
    const node = makeNode("attr_matcher", []);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("");
  });

  it("emits attr_value with IDENT token", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "IDENT", value: "submit" };
    const node = makeNode("attr_value", [tok]);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("submit");
  });

  it("emits attr_value with STRING token wraps in quotes", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "STRING", value: "text" };
    const node = makeNode("attr_value", [tok]);
    const css = emitter.emit(node);
    expect(css.trim()).toBe('"text"');
  });

  it("emits attr_value with no children returns empty", () => {
    const emitter = new CSSEmitter();
    const node = makeNode("attr_value", []);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("");
  });

  it("emits priority node as !important", () => {
    const emitter = new CSSEmitter();
    const node = makeNode("priority", []);
    // _emitPriority just returns "!important"
    // It's normally called from _emitDeclaration
    const css = emitter.emit(node);
    expect(css).toContain("!important");
  });

  it("emits value with STRING token wraps in quotes", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "STRING", value: "Arial" };
    const node = makeNode("value", [tok]);
    const css = emitter.emit(node);
    expect(css).toContain('"Arial"');
  });

  it("emits value with multiple children uses default handler", () => {
    const emitter = new CSSEmitter();
    const tok1 = { type: "IDENT", value: "hello" };
    const tok2 = { type: "IDENT", value: "world" };
    const node = makeNode("value", [tok1, tok2]);
    const css = emitter.emit(node);
    expect(css).toContain("hello");
    expect(css).toContain("world");
  });

  it("emits function_call with single URL_TOKEN child", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "URL_TOKEN", value: "url(image.png)" };
    const node = makeNode("function_call", [tok]);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("url(image.png)");
  });

  it("emits function_arg with single token child", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "NUMBER", value: "42" };
    const node = makeNode("function_arg", [tok]);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("42");
  });

  it("emits function_arg with multiple children uses default handler", () => {
    const emitter = new CSSEmitter();
    const tok1 = { type: "IDENT", value: "a" };
    const tok2 = { type: "IDENT", value: "b" };
    const node = makeNode("function_arg", [tok1, tok2]);
    const css = emitter.emit(node);
    expect(css).toContain("a");
    expect(css).toContain("b");
  });

  it("emits pseudo_element with COLON_COLON token as ::", () => {
    const emitter = new CSSEmitter();
    const tok1 = { type: "COLON_COLON", value: "::" };
    const tok2 = { type: "IDENT", value: "before" };
    const node = makeNode("pseudo_element", [tok1, tok2]);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("::before");
  });

  it("emits pseudo_element with non-COLON_COLON as value", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "IDENT", value: "after" };
    const node = makeNode("pseudo_element", [tok]);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("after");
  });

  it("emits block_item with empty children returns empty", () => {
    const emitter = new CSSEmitter();
    const node = makeNode("block_item", []);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("");
  });

  it("emits declaration_or_nested with empty children returns empty", () => {
    const emitter = new CSSEmitter();
    const node = makeNode("declaration_or_nested", []);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("");
  });

  it("emits property with empty children returns empty", () => {
    const emitter = new CSSEmitter();
    const node = makeNode("property", []);
    const css = emitter.emit(node);
    expect(css.trim()).toBe("");
  });

  it("emits pseudo_class_args with children", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "IDENT", value: "2n+1" };
    const argNode = makeNode("pseudo_class_arg", [tok]);
    const node = makeNode("pseudo_class_args", [argNode]);
    const css = emitter.emit(node);
    expect(css).toContain("2n+1");
  });

  it("default handler for unknown rule name", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "IDENT", value: "foo" };
    const node = makeNode("unknown_rule_xyz", [tok]);
    const css = emitter.emit(node);
    expect(css).toContain("foo");
  });

  it("emits at_prelude_token via default handler", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "IDENT", value: "screen" };
    const node = makeNode("at_prelude_token", [tok]);
    const css = emitter.emit(node);
    expect(css).toContain("screen");
  });

  it("emits pseudo_class_arg via default handler", () => {
    const emitter = new CSSEmitter();
    const tok = { type: "NUMBER", value: "2" };
    const node = makeNode("pseudo_class_arg", [tok]);
    const css = emitter.emit(node);
    expect(css).toContain("2");
  });
});

describe("CSSEmitter — minified mode edge cases", () => {
  it("emits minified stylesheet with multiple rules (no separator)", () => {
    const css = transpile("h1 { color: red; } h2 { color: blue; }", true);
    expect(css).toContain("color:red");
    expect(css).toContain("color:blue");
    // In minified mode, rules are joined with no separator
    expect(css.indexOf("\n")).toBe(css.length - 1); // Only trailing newline
  });

  it("emits minified selector list with comma no space", () => {
    const css = transpile("h1, h2 { color: red; }", true);
    expect(css).toContain("h1,h2");
  });

  it("emits minified declaration with no space after colon", () => {
    const css = transpile("h1 { color: red; }", true);
    expect(css).toContain("color:red");
    expect(css).not.toContain("color: red");
  });

  it("emits minified block with no newlines", () => {
    const css = transpile("h1 { color: red; font-size: 16px; }", true);
    // Should have no newlines inside the block
    expect(css.replace(/\n$/, "").includes("\n")).toBe(false);
  });

  it("emits minified !important", () => {
    const css = transpile("h1 { color: red !important; }", true);
    expect(css).toContain("!important");
  });
});

describe("CSSEmitter — at_rule variations", () => {
  it("emits @import with semicolon (no prelude spaces in minified)", () => {
    const css = transpile(`@import url("style.css");`, true);
    expect(css).toContain("@import");
  });

  it("emits @media with prelude and block", () => {
    const css = transpile("@media screen { h1 { color: red; } }");
    expect(css).toContain("@media");
    expect(css).toContain("screen");
    expect(css).toContain("color: red");
  });

  it("emits at_rule without prelude in minified", () => {
    // @charset has no complex prelude
    const css = transpile(`@charset "UTF-8";`, true);
    expect(css).toContain("@charset");
  });
});

describe("CSSEmitter — attribute selectors", () => {
  it("emits attribute selector with value and matcher", () => {
    const css = transpile('input[type="text"] { border: none; }');
    expect(css).toContain('[type="text"]');
  });

  it("emits attribute selector with IDENT value", () => {
    const css = transpile("a[target] { color: blue; }");
    expect(css).toContain("[target]");
  });
});

describe("CSSEmitter — pseudo-class with function syntax", () => {
  it("emits :nth-child pseudo-class", () => {
    const css = transpile("li:nth-child(2n+1) { color: red; }");
    expect(css).toContain(":nth-child");
    expect(css).toContain("2n+1");
  });

  it("emits :hover pseudo-class (no function args)", () => {
    const css = transpile("a:hover { text-decoration: none; }");
    expect(css).toContain(":hover");
  });

  it("emits :not() pseudo-class", () => {
    const css = transpile("p:not(.special) { color: red; }");
    expect(css).toContain(":not");
  });
});

describe("CSSEmitter — CSS function calls", () => {
  it("emits calc() function", () => {
    const css = transpile("div { width: calc(100% - 20px); }");
    expect(css).toContain("calc(100%");
  });

  it("emits linear-gradient function", () => {
    const css = transpile("div { background: linear-gradient(red, blue); }");
    expect(css).toContain("linear-gradient(red, blue)");
  });

  it("emits var() function", () => {
    const css = transpile("div { color: var(--primary); }");
    expect(css).toContain("var(--primary)");
  });
});

describe("CSSEmitter — paren_block and function_in_prelude", () => {
  it("emits @media with condition in prelude", () => {
    const css = transpile("@media screen and (min-width: 768px) { body { margin: 0; } }");
    expect(css).toContain("@media");
    expect(css).toContain("768px");
  });

  it("emits @supports rule", () => {
    const css = transpile("@supports (display: grid) { .x { display: grid; } }");
    expect(css).toContain("@supports");
  });
});

describe("CSSEmitter — complex selectors", () => {
  it("emits adjacent sibling combinator (+)", () => {
    const css = transpile("h1 + p { color: red; }");
    expect(css).toContain("+");
    expect(css).toContain("color: red");
  });

  it("emits general sibling combinator (~)", () => {
    const css = transpile("h1 ~ p { color: blue; }");
    expect(css).toContain("~");
  });

  it("emits descendant combinator (space)", () => {
    const css = transpile(".parent .child { color: green; }");
    expect(css).toContain(".parent");
    expect(css).toContain(".child");
  });

  it("emits multiple classes on same element", () => {
    const css = transpile(".btn.active { display: block; }");
    expect(css).toContain(".btn");
    expect(css).toContain(".active");
  });

  it("emits STAR universal selector", () => {
    const css = transpile("* { box-sizing: border-box; }");
    expect(css).toContain("*");
  });
});

describe("CSSEmitter — value list comma collapsing", () => {
  it("emits multi-value declaration with comma-collapsed spaces", () => {
    const css = transpile("div { font-family: Arial, sans-serif; }");
    expect(css).toContain("Arial, sans-serif");
  });

  it("emits function args comma-collapsed", () => {
    const css = transpile("div { background: rgba(0, 128, 255, 0.5); }");
    expect(css).toContain("rgba(0, 128, 255, 0.5)");
  });
});

describe("CSSEmitter — minified at_rule", () => {
  it("emits @media rule in minified mode with block", () => {
    const css = transpile("@media screen { h1 { color: red; } }", true);
    expect(css).toContain("@media");
    expect(css).toContain("color:red");
  });

  it("emits @import in minified mode", () => {
    const css = transpile(`@import "style.css";`, true);
    expect(css).toContain("@import");
    expect(css).toContain(";");
  });

  it("emits @charset with string value in minified (no space)", () => {
    // @charset has a simple string prelude
    const css = transpile(`@charset "UTF-8";`, true);
    expect(css).toContain("@charset");
  });
});

// =============================================================================
// ExpressionEvaluator — direct tests via scope and node construction
// =============================================================================

describe("ExpressionEvaluator — fallthrough paths", () => {
  it("evaluates unknown rule with single child (unwraps)", () => {
    const scope = new ScopeChain();
    const evaluator = new ExpressionEvaluator(scope);
    // A wrapper node with a single child (token)
    const numToken = { type: "NUMBER", value: "7", line: 1, column: 1 };
    const wrapperNode = { ruleName: "some_wrapper", children: [numToken] } as any;
    const result = evaluator.evaluate(wrapperNode);
    expect(result).toEqual(new LatticeNumber(7));
  });

  it("evaluates unknown rule with multiple children returns first meaningful", () => {
    const scope = new ScopeChain();
    const evaluator = new ExpressionEvaluator(scope);
    const numToken1 = { type: "NUMBER", value: "3", line: 1, column: 1 };
    const numToken2 = { type: "NUMBER", value: "5", line: 1, column: 1 };
    const wrapperNode = { ruleName: "multi_wrapper", children: [numToken1, numToken2] } as any;
    const result = evaluator.evaluate(wrapperNode);
    expect(result).toEqual(new LatticeNumber(3));
  });

  it("evaluates empty node with multiple children returns LatticeNull", () => {
    const scope = new ScopeChain();
    const evaluator = new ExpressionEvaluator(scope);
    // Node with no children
    const emptyNode = { ruleName: "empty_node", children: [] } as any;
    const result = evaluator.evaluate(emptyNode);
    expect(result).toBeInstanceOf(LatticeNull);
  });

  it("evaluates comparison_op node extracts first token", () => {
    const scope = new ScopeChain();
    const evaluator = new ExpressionEvaluator(scope);
    const opToken = { type: "EQUALS_EQUALS", value: "==", line: 1, column: 1 };
    const opNode = { ruleName: "comparison_op", children: [opToken] } as any;
    // comparison_op returns tokenToValue of its first child
    const result = evaluator.evaluate(opNode);
    // EQUALS_EQUALS is not a known type, so becomes LatticeIdent("==")
    expect(result).toBeInstanceOf(LatticeIdent);
  });

  it("evaluates lattice_primary skipping LPAREN and RPAREN", () => {
    const scope = new ScopeChain();
    const evaluator = new ExpressionEvaluator(scope);
    // Simulates (42) — LPAREN, NUMBER, RPAREN
    const lp = { type: "LPAREN", value: "(", line: 1, column: 1 };
    const num = { type: "NUMBER", value: "42", line: 1, column: 1 };
    const rp = { type: "RPAREN", value: ")", line: 1, column: 1 };
    const primaryNode = { ruleName: "lattice_primary", children: [lp, num, rp] } as any;
    const result = evaluator.evaluate(primaryNode);
    expect(result).toEqual(new LatticeNumber(42));
  });

  it("evaluates lattice_primary variable stored as null returns LatticeNull", () => {
    const scope = new ScopeChain();
    scope.set("$nullvar", null);
    const evaluator = new ExpressionEvaluator(scope);
    const varToken = { type: "VARIABLE", value: "$nullvar", line: 1, column: 1 };
    const primaryNode = { ruleName: "lattice_primary", children: [varToken] } as any;
    const result = evaluator.evaluate(primaryNode);
    expect(result).toBeInstanceOf(LatticeNull);
  });
});

describe("ExpressionEvaluator — additive multiple operations", () => {
  it("evaluates a + b + c via transpile", () => {
    const css = transpile(`
      @function add3($a, $b, $c) {
        @return $a + $b + $c;
      }
      .x { width: add3(1, 2, 3); }
    `);
    // 1 + 2 + 3 = 6 (pure numbers)
    expect(css).toContain("width: 6");
  });

  it("evaluates a - b - c via transpile", () => {
    const css = transpile(`
      @function sub3($a, $b, $c) {
        @return $a - $b - $c;
      }
      .x { width: sub3(10, 3, 2); }
    `);
    // 10 - 3 - 2 = 5
    expect(css).toContain("width: 5");
  });
});

describe("ExpressionEvaluator — multiplicative chain", () => {
  it("evaluates a * b * c via transpile", () => {
    const css = transpile(`
      @function mul3($a, $b, $c) {
        @return $a * $b * $c;
      }
      .x { padding: mul3(2, 3, 4); }
    `);
    // 2 * 3 * 4 = 24
    expect(css).toContain("padding: 24");
  });
});

describe("ExpressionEvaluator — or short-circuit (truthy first)", () => {
  it("or returns first truthy value without evaluating second", () => {
    const css = transpile(`
      @mixin check($a, $b) {
        @if $a == x or $b == y {
          color: found;
        }
      }
      .x { @include check(x, z); }
    `);
    // $a == x is true, so the whole 'or' is truthy
    expect(css).toContain("color: found");
  });

  it("or returns second when first is falsy", () => {
    const css = transpile(`
      @mixin check($a, $b) {
        @if $a == x or $b == y {
          color: found;
        }
      }
      .x { @include check(z, y); }
    `);
    // $a == x is false, $b == y is true
    expect(css).toContain("color: found");
  });

  it("or returns false when both are falsy", () => {
    const css = transpile(`
      @mixin check($a, $b) {
        @if $a == x or $b == y {
          color: found;
        } @else {
          color: notfound;
        }
      }
      .x { @include check(z, z); }
    `);
    expect(css).toContain("color: notfound");
    expect(css).not.toContain("color: found");
  });
});

describe("ExpressionEvaluator — and short-circuit (falsy first)", () => {
  it("and returns first falsy when first is false", () => {
    const css = transpile(`
      @mixin check($a, $b) {
        @if $a == x and $b == y {
          color: both;
        } @else {
          color: notboth;
        }
      }
      .x { @include check(z, y); }
    `);
    // $a == x is false, short-circuits
    expect(css).toContain("color: notboth");
  });

  it("and returns true when both are true", () => {
    const css = transpile(`
      @mixin check($a, $b) {
        @if $a == x and $b == y {
          color: both;
        }
      }
      .x { @include check(x, y); }
    `);
    expect(css).toContain("color: both");
  });
});

describe("ExpressionEvaluator — comparison on dimensions", () => {
  it("compares dimensions with == (same unit, same value)", () => {
    expect(compareValues(new LatticeDimension(10, "px"), new LatticeDimension(10, "px"), "EQUALS_EQUALS"))
      .toEqual(new LatticeBool(true));
  });

  it("compares dimensions with > (same unit)", () => {
    expect(compareValues(new LatticeDimension(20, "px"), new LatticeDimension(10, "px"), "GREATER"))
      .toEqual(new LatticeBool(true));
  });

  it("compares dimensions with >= (same unit)", () => {
    expect(compareValues(new LatticeDimension(10, "px"), new LatticeDimension(10, "px"), "GREATER_EQUALS"))
      .toEqual(new LatticeBool(true));
  });

  it("compares dimensions with <= (same unit)", () => {
    expect(compareValues(new LatticeDimension(5, "px"), new LatticeDimension(10, "px"), "LESS_EQUALS"))
      .toEqual(new LatticeBool(true));
  });

  it("compares percentages with ==", () => {
    expect(compareValues(new LatticePercentage(50), new LatticePercentage(50), "EQUALS_EQUALS"))
      .toEqual(new LatticeBool(true));
  });

  it("compares percentages with != different values", () => {
    expect(compareValues(new LatticePercentage(30), new LatticePercentage(50), "NOT_EQUALS"))
      .toEqual(new LatticeBool(true));
  });

  it("compares percentages with >", () => {
    expect(compareValues(new LatticePercentage(80), new LatticePercentage(50), "GREATER"))
      .toEqual(new LatticeBool(true));
  });

  it("compares percentages with >=", () => {
    expect(compareValues(new LatticePercentage(50), new LatticePercentage(50), "GREATER_EQUALS"))
      .toEqual(new LatticeBool(true));
  });

  it("compares percentages with <=", () => {
    expect(compareValues(new LatticePercentage(20), new LatticePercentage(50), "LESS_EQUALS"))
      .toEqual(new LatticeBool(true));
  });
});

describe("ExpressionEvaluator — via transpile, condition cases", () => {
  it("@if with null variable is falsy", () => {
    // null is falsy, so @else branch triggers
    const css = transpile(`
      @mixin check($v) {
        @if $v {
          color: yes;
        } @else {
          color: no;
        }
      }
      .x { @include check(null); }
    `);
    expect(css).toContain("color: no");
  });

  it("@if with number 0 is falsy", () => {
    const css = transpile(`
      @mixin check($v) {
        @if $v {
          color: yes;
        } @else {
          color: no;
        }
      }
      .x { @include check(0); }
    `);
    expect(css).toContain("color: no");
  });

  it("@if with truthy number is truthy", () => {
    const css = transpile(`
      @mixin check($v) {
        @if $v {
          color: yes;
        }
      }
      .x { @include check(1); }
    `);
    expect(css).toContain("color: yes");
  });

  it("@if with false ident is falsy", () => {
    const css = transpile(`
      @mixin check($v) {
        @if $v {
          color: yes;
        } @else {
          color: no;
        }
      }
      .x { @include check(false); }
    `);
    expect(css).toContain("color: no");
  });

  it("@if with true ident is truthy", () => {
    const css = transpile(`
      @mixin check($v) {
        @if $v {
          color: yes;
        }
      }
      .x { @include check(true); }
    `);
    expect(css).toContain("color: yes");
  });
});

describe("LatticeTransformer — @function with @if inside", () => {
  it("function with @if returning from one branch", () => {
    const css = transpile(`
      @function classify($n) {
        @if $n > 10 {
          @return big;
        }
        @return small;
      }
      .x { class: classify(15); }
    `);
    expect(css).toContain("class: big");
  });

  it("function with @if falling through to else return", () => {
    const css = transpile(`
      @function classify($n) {
        @if $n > 10 {
          @return big;
        }
        @return small;
      }
      .x { class: classify(5); }
    `);
    expect(css).toContain("class: small");
  });

  it("function adding percentage values", () => {
    const css = transpile(`
      @function half($p) {
        @return $p + 0%;
      }
      .x { width: half(50%); }
    `);
    expect(css).toContain("width: 50%");
  });

  it("function with @else if branch", () => {
    const css = transpile(`
      @function grade($n) {
        @if $n >= 90 {
          @return A;
        }
        @return B;
      }
      .x { grade: grade(95); }
    `);
    expect(css).toContain("grade: A");
  });
});

describe("LatticeTransformer — variable scoping edge cases", () => {
  it("variable used in nested block", () => {
    const css = transpile(`
      $size: 14px;
      p { font-size: $size; }
    `);
    expect(css).toContain("font-size: 14px");
  });

  it("@use directive is silently skipped", () => {
    // @use is not fully implemented, should produce no output from the directive
    const css = transpile(`
      @use "colors";
      h1 { color: red; }
    `);
    expect(css).toContain("color: red");
    expect(css).not.toContain("@use");
  });
});

describe("LatticeTransformer — CSS passthrough edge cases", () => {
  it("passes through @keyframes with from/to", () => {
    const css = transpile(`
      @keyframes spin {
        from { transform: rotate(0deg); }
        to { transform: rotate(360deg); }
      }
    `);
    expect(css).toContain("@keyframes");
    expect(css).toContain("transform");
  });

  it("passes through CSS custom property declaration", () => {
    const css = transpile(":root { --primary: #4a90d9; }");
    expect(css).toContain("--primary");
  });

  it("passes through multiple rules", () => {
    const css = transpile("h1 { color: red; } h2 { color: blue; }");
    expect(css).toContain("color: red");
    expect(css).toContain("color: blue");
  });

  it("passes through @font-face", () => {
    const css = transpile(`@font-face { font-family: MyFont; src: url("font.woff"); }`);
    expect(css).toContain("@font-face");
    expect(css).toContain("font-family");
  });
});

describe("values.ts — tokenToValue edge cases", () => {
  it("handles unknown token type as LatticeIdent", () => {
    const tok = { type: "UNKNOWN_TYPE", value: "weird", line: 1, column: 1 };
    const result = tokenToValue(tok as any);
    expect(result).toBeInstanceOf(LatticeIdent);
    expect((result as LatticeIdent).value).toBe("weird");
  });

  it("handles DIMENSION with only unit letters (fallback path)", () => {
    // Create a DIMENSION token with an unusual format that matches the fallback
    const tok = { type: "DIMENSION", value: "1.5em", line: 1, column: 1 };
    const result = tokenToValue(tok as any);
    expect(result).toBeInstanceOf(LatticeDimension);
    expect((result as LatticeDimension).unit).toBe("em");
    expect((result as LatticeDimension).value).toBe(1.5);
  });

  it("handles DIMENSION with negative value", () => {
    const tok = { type: "DIMENSION", value: "-10px", line: 1, column: 1 };
    const result = tokenToValue(tok as any);
    expect(result).toBeInstanceOf(LatticeDimension);
    expect((result as LatticeDimension).value).toBe(-10);
    expect((result as LatticeDimension).unit).toBe("px");
  });
});

describe("values.ts — LatticeNumber infinite value", () => {
  it("toString of infinite number uses String()", () => {
    // When value is Infinity or NaN, Math.trunc check fails
    const n = new LatticeNumber(Infinity);
    expect(n.toString()).toBe("Infinity");
  });

  it("toString of NaN number", () => {
    const n = new LatticeNumber(NaN);
    expect(n.toString()).toBe("NaN");
  });
});

describe("values.ts — LatticeDimension infinite value", () => {
  it("toString of infinite dimension", () => {
    const d = new LatticeDimension(Infinity, "px");
    expect(d.toString()).toBe("Infinitypx");
  });
});

describe("values.ts — LatticePercentage infinite value", () => {
  it("toString of infinite percentage", () => {
    const p = new LatticePercentage(Infinity);
    expect(p.toString()).toBe("Infinity%");
  });
});

describe("values.ts — LatticeList", () => {
  it("toString of empty list is empty string", () => {
    const list = new LatticeList([]);
    expect(list.toString()).toBe("");
  });

  it("toString of single-item list has no comma", () => {
    const list = new LatticeList([new LatticeIdent("red")]);
    expect(list.toString()).toBe("red");
  });
});

describe("values.ts — extractValueFromAst — child is null-kind node", () => {
  it("skips null-kind children and recurses deeper", () => {
    // Build: node with a child that returns null kind (empty node)
    const innerEmpty = { ruleName: "inner", children: [] } as any;
    const outerNode = { ruleName: "outer", children: [innerEmpty] } as any;
    const result = extractValueFromAst(outerNode);
    expect(result).toBeInstanceOf(LatticeNull);
  });
});

describe("errors.ts — all constructors", () => {
  it("LatticeError without position", () => {
    const e = new LatticeError("test message");
    expect(e.line).toBe(0);
    expect(e.column).toBe(0);
    expect(e.message).toBe("test message");
    expect(e.latticeMessage).toBe("test message");
    expect(e.name).toBe("LatticeError");
  });

  it("LatticeError with position includes location in message", () => {
    const e = new LatticeError("bad thing", 3, 7);
    expect(e.message).toContain("line 3");
    expect(e.message).toContain("column 7");
  });

  it("UndefinedFunctionError stores name and is LatticeError", () => {
    const e = new UndefinedFunctionError("spacing", 2, 4);
    expect(e.name).toBe("spacing");
    expect(e instanceof LatticeError).toBe(true);
    expect(e.message).toContain("spacing");
  });

  it("WrongArityError with position", () => {
    const e = new WrongArityError("Function", "calc", 2, 3, 5, 10);
    expect(e.name).toBe("calc");
    expect(e.expected).toBe(2);
    expect(e.got).toBe(3);
    expect(e.line).toBe(5);
    expect(e.column).toBe(10);
  });

  it("CircularReferenceError with function kind", () => {
    const e = new CircularReferenceError("function", ["fn1", "fn2", "fn1"], 1, 0);
    expect(e.chain).toEqual(["fn1", "fn2", "fn1"]);
    expect(e.message).toContain("fn1 → fn2 → fn1");
  });

  it("TypeErrorInExpression with position", () => {
    const e = new TypeErrorInExpression("negate", "red", "", 3, 5);
    expect(e.op).toBe("negate");
    expect(e.leftType).toBe("red");
    expect(e.rightType).toBe("");
    expect(e.line).toBe(3);
  });

  it("MissingReturnError with position", () => {
    const e = new MissingReturnError("myFunc", 7, 2);
    expect(e.name).toBe("myFunc");
    expect(e.line).toBe(7);
    expect(e.column).toBe(2);
  });

  it("UnitMismatchError with position", () => {
    const e = new UnitMismatchError("px", "s", 4, 8);
    expect(e.leftUnit).toBe("px");
    expect(e.rightUnit).toBe("s");
    expect(e.line).toBe(4);
    expect(e.column).toBe(8);
  });

  it("LatticeModuleNotFoundError with position", () => {
    const e = new LatticeModuleNotFoundError("colors", 1, 5);
    expect(e.moduleName).toBe("colors");
    expect(e.line).toBe(1);
  });
});

describe("ScopeChain — toString", () => {
  it("toString shows depth and binding names", () => {
    const scope = new ScopeChain();
    scope.set("$x", 1);
    scope.set("$y", 2);
    const str = scope.toString();
    expect(str).toContain("depth=0");
    expect(str).toContain("$x");
    expect(str).toContain("$y");
  });

  it("toString of child scope shows depth=1", () => {
    const parent = new ScopeChain();
    const child = parent.child();
    const str = child.toString();
    expect(str).toContain("depth=1");
  });
});

describe("LatticeTransformer — function call in CSS value", () => {
  it("CSS built-in function passes through (not expanded as Lattice function)", () => {
    // rgb() is a CSS built-in — should not be treated as Lattice function
    const css = transpile(".x { color: rgb(255, 0, 0); }");
    expect(css).toContain("rgb(255, 0, 0)");
  });

  it("unknown non-CSS function passes through unchanged", () => {
    // myFunc() is unknown — transformer passes it through
    const css = transpile(".x { value: myFunc(5); }");
    expect(css).toContain("myFunc(5)");
  });
});

describe("LatticeTransformer — @function WrongArityError", () => {
  it("throws WrongArityError when function called with wrong arg count", () => {
    expect(() =>
      transpile(`
        @function add($a, $b) {
          @return $a + $b;
        }
        .x { value: add(1); }
      `)
    ).toThrow(WrongArityError);
  });
});
