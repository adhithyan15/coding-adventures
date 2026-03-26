/**
 * Lattice AST Transformer — Expands Lattice constructs into pure CSS.
 *
 * This is the core of the Lattice-to-CSS compiler. It takes a Lattice AST
 * (containing both CSS and Lattice nodes) and produces a clean CSS AST
 * (containing only CSS nodes) by expanding all Lattice constructs.
 *
 * Three-Pass Architecture
 * -----------------------
 *
 * The transformation runs in three passes:
 *
 * Pass 1: Symbol Collection
 *   Walk the top-level AST and collect definitions:
 *     - Variable declarations → variable registry
 *     - Mixin definitions → mixin registry
 *     - Function definitions → function registry
 *   Remove definition nodes from the AST (they produce no CSS output).
 *
 * Pass 2: Expansion
 *   Recursively walk remaining AST nodes with a scope chain:
 *     - Replace VARIABLE tokens with their resolved values
 *     - Expand @include directives by cloning mixin bodies
 *     - Evaluate @if/@for/@each control flow
 *     - Evaluate Lattice function calls and replace with return values
 *
 * Pass 3: Cleanup
 *   Remove any empty blocks or nodes that resulted from transformation.
 *
 * Why Not a Single Pass?
 * Because mixins and functions can be defined after they're used:
 *
 *     .btn { @include button(red); }   ← used first
 *     @mixin button($bg) { ... }       ← defined later
 *
 * Pass 1 collects all definitions up front, so Pass 2 can resolve them
 * regardless of source order.
 *
 * AST Mutation vs Immutability
 * -----------------------------
 *
 * The transformer MUTATES the AST in place (modifying children arrays).
 * This matches the Python reference implementation and is more efficient
 * than creating new nodes at every step. The input AST should not be
 * reused after transformation.
 *
 * For immutable transformation (needed for incremental compilation),
 * deep-clone the AST before passing it to transform().
 */

import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";
import { ScopeChain } from "./scope.js";
import { ExpressionEvaluator } from "./evaluator.js";
import {
  LatticeValue,
  LatticeNumber,
  LatticeNull,
  LatticeIdent,
  LatticeMap,
  LatticeList,
  isTruthy,
  tokenToValue,
  valueToCss,
  extractValueFromAst,
  BUILTIN_FUNCTIONS,
} from "./values.js";
import {
  UndefinedVariableError,
  UndefinedMixinError,
  CircularReferenceError,
  WrongArityError,
  MissingReturnError,
  MaxIterationError,
} from "./errors.js";

// =============================================================================
// Type Guards
// =============================================================================

/** Check if a child is an ASTNode (not a Token). */
function isASTNode(child: ASTNode | Token): child is ASTNode {
  return "ruleName" in child;
}

/** Get the string token type. */
function tokenTypeName(token: Token): string {
  return token.type as string;
}

/** Get the .value of a token, or undefined if it's an ASTNode. */
function getTokenValue(child: ASTNode | Token): string | undefined {
  if (isASTNode(child)) return undefined;
  return (child as Token).value;
}

// =============================================================================
// CSS Built-in Functions
// =============================================================================

/**
 * CSS built-in functions that should NOT be resolved as Lattice functions.
 *
 * When a function_call node uses one of these names, it's passed through
 * unchanged. Only Lattice-defined @function names are expanded.
 */
const CSS_FUNCTIONS = new Set([
  "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklch", "oklab",
  "color", "color-mix",
  "calc", "min", "max", "clamp", "abs", "sign", "round", "mod", "rem",
  "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "pow", "sqrt",
  "hypot", "log", "exp",
  "var", "env",
  "url", "format", "local",
  "linear-gradient", "radial-gradient", "conic-gradient",
  "repeating-linear-gradient", "repeating-radial-gradient",
  "repeating-conic-gradient",
  "counter", "counters", "attr", "element",
  "translate", "translateX", "translateY", "translateZ",
  "rotate", "rotateX", "rotateY", "rotateZ",
  "scale", "scaleX", "scaleY", "scaleZ",
  "skew", "skewX", "skewY",
  "matrix", "matrix3d", "perspective",
  "cubic-bezier", "steps",
  "path", "polygon", "circle", "ellipse", "inset",
  "image-set", "cross-fade",
  "fit-content", "minmax", "repeat",
  "blur", "brightness", "contrast", "drop-shadow", "grayscale",
  "hue-rotate", "invert", "opacity", "saturate", "sepia",
]);

/** Check if a function name is a CSS built-in (not a Lattice function). */
function isCssFunction(name: string): boolean {
  // FUNCTION token includes "(" at the end: "rgb(" → "rgb"
  const cleanName = name.replace(/\($/, "");
  return CSS_FUNCTIONS.has(cleanName);
}

// =============================================================================
// Mixin and Function Definition Records
// =============================================================================

/** Stored definition of a @mixin. */
interface MixinDef {
  readonly name: string;
  readonly params: string[];
  readonly defaults: Map<string, ASTNode | Token>;
  readonly body: ASTNode | Token;
}

/** Stored definition of a @function. */
interface FunctionDef {
  readonly name: string;
  readonly params: string[];
  readonly defaults: Map<string, ASTNode | Token>;
  readonly body: ASTNode | Token;
}

// =============================================================================
// Return Signal (internal exception for @return)
// =============================================================================

/**
 * Internal signal for @return inside function evaluation.
 *
 * Not a real error — used to unwind the function body evaluation
 * when a @return is hit. The value is the LatticeValue to return.
 *
 * Design note: using an exception for control flow is unusual in TypeScript,
 * but it mirrors the Python reference implementation and avoids threading a
 * "return value" through every recursive call. The alternative would be
 * returning a Result<T> type, which adds significant complexity.
 */
class ReturnSignal {
  constructor(readonly value: LatticeValue) {}
}

// =============================================================================
// Simple AST Node (for synthetic nodes created during transformation)
// =============================================================================

/** A minimal ASTNode for synthetic nodes. */
class SimpleASTNode implements ASTNode {
  constructor(
    readonly ruleName: string,
    readonly children: Array<ASTNode | Token>
  ) {}
}

/** A minimal Token for synthetic tokens. */
interface SyntheticToken extends Token {
  type: string;
  value: string;
  line: number;
  column: number;
}

/** Create a synthetic token based on a CSS value string. */
function makeSyntheticToken(value: string, template: ASTNode | Token): SyntheticToken {
  const line = isASTNode(template) ? 0 : (template as Token).line ?? 0;
  const column = isASTNode(template) ? 0 : (template as Token).column ?? 0;

  // Determine the token type based on the value's shape
  let type = "IDENT";
  if (value.startsWith("#")) {
    type = "HASH";
  } else if (value.startsWith('"') || value.startsWith("'")) {
    type = "STRING";
  } else if (/^-?[0-9]*\.?[0-9]+%$/.test(value)) {
    type = "PERCENTAGE";
  } else if (/^-?[0-9]*\.?[0-9]+[a-zA-Z]/.test(value)) {
    type = "DIMENSION";
  } else if (/^-?[0-9]*\.?[0-9]+$/.test(value)) {
    type = "NUMBER";
  }

  return { type, value, line, column };
}

/** Create a value node (ASTNode) wrapping a synthetic token. */
function makeValueNode(cssText: string, template: ASTNode | Token): ASTNode {
  const token = makeSyntheticToken(cssText, template);
  return new SimpleASTNode("value", [token]);
}

/** Deep-clone an ASTNode or Token tree. */
function deepClone<T extends ASTNode | Token>(node: T): T {
  if (!isASTNode(node)) {
    // Token — clone it as a plain object
    return { ...(node as Token) } as T;
  }
  const ast = node as ASTNode;
  return new SimpleASTNode(
    ast.ruleName,
    ast.children.map((c) => deepClone(c))
  ) as unknown as T;
}

// =============================================================================
// Mutable AST node (for mutation during transformation)
// =============================================================================

/** Get a mutable children array from an ASTNode. */
function getChildren(node: ASTNode): Array<ASTNode | Token> {
  return node.children as Array<ASTNode | Token>;
}

/** Set children on an ASTNode (mutation). */
function setChildren(node: ASTNode, children: Array<ASTNode | Token>): void {
  (node as { children: Array<ASTNode | Token> }).children = children;
}

// =============================================================================
// LatticeTransformer
// =============================================================================

/**
 * Transforms a Lattice AST into a clean CSS AST.
 *
 * Usage:
 *
 *     const transformer = new LatticeTransformer();
 *     const cssAst = transformer.transform(latticeAst);
 *
 * The returned AST contains only CSS nodes (no variables, mixins,
 * control flow, or function definitions). It can be passed directly
 * to CSSEmitter.emit() to produce CSS text.
 */
export class LatticeTransformer {
  /** Global scope for variable bindings. */
  private readonly variables: ScopeChain = new ScopeChain();

  /** Mixin definitions collected in Pass 1. */
  private readonly mixins: Map<string, MixinDef> = new Map();

  /** Function definitions collected in Pass 1. */
  private readonly functions: Map<string, FunctionDef> = new Map();

  /** Call stack for cycle detection in mixin expansion. */
  private readonly mixinStack: string[] = [];

  /** Call stack for cycle detection in function evaluation. */
  private readonly functionStack: string[] = [];

  /** Lattice v2: Maximum iterations for @while loops. */
  private readonly maxWhileIterations: number;

  /** Lattice v2: @extend tracking. Maps target selector -> extending selectors. */
  private readonly extendMap: Map<string, string[]> = new Map();

  /** Lattice v2: @at-root hoisted rules. Spliced into root in Pass 3. */
  private readonly atRootRules: Array<ASTNode | Token> = [];

  /** Lattice v2: @content block stack. Each entry is a block AST or null. */
  private readonly contentBlockStack: Array<ASTNode | null> = [];

  /** Lattice v2: Scope stack for @content evaluation (caller's scope). */
  private readonly contentScopeStack: ScopeChain[] = [];

  constructor(maxWhileIterations: number = 1000) {
    this.maxWhileIterations = maxWhileIterations;
  }

  /**
   * Transform a Lattice AST into a clean CSS AST.
   *
   * Runs the three-pass pipeline:
   * 1. Collect symbols (variables, mixins, functions)
   * 2. Expand all Lattice constructs
   * 3. Clean up empty nodes, apply @extend, splice @at-root
   *
   * @param ast - The root "stylesheet" ASTNode from the parser.
   * @returns A clean CSS AST with no Lattice nodes.
   */
  transform(ast: ASTNode): ASTNode {
    // Pass 1: Collect symbols
    this._collectSymbols(ast);

    // Pass 2: Expand
    const result = this._expandNode(ast, this.variables);

    // Pass 3: Cleanup + @extend selector merging + @at-root hoisting
    const cleaned = this._cleanup(result as ASTNode) as ASTNode;
    if (this.extendMap.size > 0) {
      this._applyExtends(cleaned);
    }
    if (this.atRootRules.length > 0) {
      this._spliceAtRootRules(cleaned);
    }
    return cleaned;
  }

  // ===========================================================================
  // Pass 1: Symbol Collection
  // ===========================================================================

  /**
   * Walk top-level rules and collect variable/mixin/function definitions.
   *
   * Definitions are removed from the AST children list since they
   * produce no CSS output.
   */
  private _collectSymbols(ast: ASTNode): void {
    if (!isASTNode(ast)) return;

    const newChildren: Array<ASTNode | Token> = [];

    for (const child of getChildren(ast)) {
      if (!isASTNode(child)) {
        newChildren.push(child);
        continue;
      }

      const childAst = child as ASTNode;

      if (childAst.ruleName === "rule") {
        const ruleChildren = getChildren(childAst);
        if (ruleChildren.length === 0) {
          newChildren.push(child);
          continue;
        }

        const inner = ruleChildren[0];
        if (!isASTNode(inner)) {
          newChildren.push(child);
          continue;
        }

        const innerAst = inner as ASTNode;

        if (innerAst.ruleName === "lattice_rule") {
          const latticeChildren = getChildren(innerAst);
          if (latticeChildren.length === 0) {
            newChildren.push(child);
            continue;
          }

          const latticeInner = latticeChildren[0];
          if (!isASTNode(latticeInner)) {
            newChildren.push(child);
            continue;
          }

          const latticeRule = (latticeInner as ASTNode).ruleName;

          if (latticeRule === "variable_declaration") {
            this._collectVariable(latticeInner as ASTNode);
            continue; // Don't add to output — variable declarations produce no CSS
          } else if (latticeRule === "mixin_definition") {
            this._collectMixin(latticeInner as ASTNode);
            continue;
          } else if (latticeRule === "function_definition") {
            this._collectFunction(latticeInner as ASTNode);
            continue;
          } else if (latticeRule === "use_directive") {
            continue; // Skip @use for now (module resolution not implemented)
          }
        }

        newChildren.push(child);
      } else {
        newChildren.push(child);
      }
    }

    setChildren(ast, newChildren);
  }

  /**
   * Extract variable name and value from a variable_declaration node.
   *
   * Lattice v2 adds !default and !global flags:
   * - !default: only set the variable if it is not already defined.
   * - !global: set the variable in the root (global) scope.
   * - Both can appear together.
   */
  private _collectVariable(node: ASTNode): void {
    let name: string | undefined;
    let valueNode: ASTNode | undefined;
    let isDefault = false;
    let isGlobal = false;

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) {
        const typeName = tokenTypeName(child as Token);
        if (typeName === "VARIABLE") {
          name = (child as Token).value;
        } else if (typeName === "BANG_DEFAULT") {
          isDefault = true;
        } else if (typeName === "BANG_GLOBAL") {
          isGlobal = true;
        }
      } else {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "value_list") {
          valueNode = childAst;
        } else if (childAst.ruleName === "variable_flag") {
          for (const fc of getChildren(childAst)) {
            if (!isASTNode(fc)) {
              const ft = tokenTypeName(fc as Token);
              if (ft === "BANG_DEFAULT") isDefault = true;
              else if (ft === "BANG_GLOBAL") isGlobal = true;
            }
          }
        }
      }
    }

    if (name && valueNode) {
      if (isDefault && isGlobal) {
        // Check global scope only -- if not defined there, set globally
        let root: ScopeChain = this.variables;
        while (root.parent !== null) root = root.parent;
        if (root.get(name) === undefined) {
          this.variables.setGlobal(name, valueNode);
        }
      } else if (isDefault) {
        if (this.variables.get(name) === undefined) {
          this.variables.set(name, valueNode);
        }
      } else if (isGlobal) {
        this.variables.setGlobal(name, valueNode);
      } else {
        this.variables.set(name, valueNode);
      }
    }
  }

  /** Extract mixin name, params, and body from a mixin_definition node. */
  private _collectMixin(node: ASTNode): void {
    let name: string | undefined;
    let params: string[] = [];
    let defaults: Map<string, ASTNode | Token> = new Map();
    let body: ASTNode | undefined;

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) {
        if (tokenTypeName(child as Token) === "FUNCTION") {
          // FUNCTION token is "name(" — strip the paren
          name = (child as Token).value.replace(/\($/, "");
        }
      } else {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "mixin_params") {
          const extracted = this._extractParams(childAst);
          params = extracted.params;
          defaults = extracted.defaults;
        } else if (childAst.ruleName === "block") {
          body = childAst;
        }
      }
    }

    if (name && body) {
      this.mixins.set(name, { name, params, defaults, body });
    }
  }

  /** Extract function name, params, and body from a function_definition node. */
  private _collectFunction(node: ASTNode): void {
    let name: string | undefined;
    let params: string[] = [];
    let defaults: Map<string, ASTNode | Token> = new Map();
    let body: ASTNode | undefined;

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) {
        if (tokenTypeName(child as Token) === "FUNCTION") {
          name = (child as Token).value.replace(/\($/, "");
        }
      } else {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "mixin_params") {
          const extracted = this._extractParams(childAst);
          params = extracted.params;
          defaults = extracted.defaults;
        } else if (childAst.ruleName === "function_body") {
          body = childAst;
        }
      }
    }

    if (name && body) {
      this.functions.set(name, { name, params, defaults, body });
    }
  }

  /**
   * Extract parameter names and defaults from mixin_params.
   *
   * mixin_params = mixin_param { COMMA mixin_param } ;
   * mixin_param = VARIABLE [ COLON mixin_value_list ] ;
   */
  private _extractParams(node: ASTNode): {
    params: string[];
    defaults: Map<string, ASTNode | Token>;
  } {
    const params: string[] = [];
    const defaults: Map<string, ASTNode | Token> = new Map();

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) continue;
      const childAst = child as ASTNode;
      if (childAst.ruleName === "mixin_param") {
        let paramName: string | undefined;
        let defaultValue: ASTNode | undefined;

        for (const pc of getChildren(childAst)) {
          if (!isASTNode(pc)) {
            if (tokenTypeName(pc as Token) === "VARIABLE") {
              paramName = (pc as Token).value;
            }
          } else if ((pc as ASTNode).ruleName === "value_list" || (pc as ASTNode).ruleName === "mixin_value_list") {
            defaultValue = pc as ASTNode;
          }
        }

        if (paramName) {
          params.push(paramName);
          if (defaultValue !== undefined) {
            defaults.set(paramName, defaultValue);
          }
        }
      }
    }

    return { params, defaults };
  }

  // ===========================================================================
  // Pass 2: Expansion
  // ===========================================================================

  /**
   * Recursively expand a single AST node.
   *
   * Dispatches on ruleName to handle Lattice-specific constructs.
   * CSS nodes are passed through with their children expanded.
   */
  private _expandNode(
    node: ASTNode | Token,
    scope: ScopeChain
  ): ASTNode | Token | Array<ASTNode | Token> | null {
    if (!isASTNode(node)) {
      // Token — check for variable substitution
      const token = node as Token;
      if (tokenTypeName(token) === "VARIABLE") {
        return this._substituteVariable(token, scope);
      }
      return token;
    }

    const ast = node as ASTNode;
    const rule = ast.ruleName;

    switch (rule) {
      case "stylesheet":
        return this._expandStylesheet(ast, scope);
      case "rule":
        return this._expandTopLevelRule(ast, scope);
      case "lattice_rule":
        return this._expandTopLevelLatticeRule(ast, scope);
      case "lattice_control":
        return this._expandControl(ast, scope);
      case "block":
        return this._expandBlock(ast, scope);
      case "block_contents":
        return this._expandBlockContents(ast, scope);
      case "block_item":
        return this._expandBlockItem(ast, scope);
      case "value_list":
        return this._expandValueList(ast, scope);
      case "value":
        return this._expandValue(ast, scope);
      case "function_call":
        return this._expandFunctionCall(ast, scope);
      case "function_arg":
        return this._expandChildren(ast, scope);
      case "function_args":
        return this._expandChildren(ast, scope);
      // Lattice v2: resolve variables in selector positions
      case "compound_selector":
      case "simple_selector":
      case "class_selector":
        return this._expandSelectorWithVars(ast, scope);
      default:
        return this._expandChildren(ast, scope);
    }
  }

  /**
   * Expand a top-level rule node.
   *
   * rule = lattice_rule | at_rule | qualified_rule ;
   *
   * Most rules are expanded by `_expandChildren`. The special case is
   * lattice_rule wrapping lattice_control — which can produce multiple
   * output nodes (one for each loop iteration, or the chosen branch of @if).
   *
   * For those, we need to return the expanded nodes so the stylesheet handler
   * can splice them in correctly.
   */
  private _expandTopLevelRule(
    node: ASTNode,
    scope: ScopeChain
  ): ASTNode | Token | Array<ASTNode | Token> | null {
    const children = getChildren(node);
    if (children.length === 0) return node;

    const inner = children[0];
    if (!isASTNode(inner)) {
      return this._expandChildren(node, scope);
    }

    const innerAst = inner as ASTNode;
    if (innerAst.ruleName === "lattice_rule") {
      // Expand the lattice_rule — may return multiple nodes (control flow)
      const expanded = this._expandTopLevelLatticeRule(innerAst, scope);
      if (expanded === null) return null;
      if (Array.isArray(expanded)) {
        // Wrap each expanded item in a rule node for consistent structure
        return expanded;
      }
      setChildren(node, [expanded as ASTNode | Token]);
      return node;
    }

    return this._expandChildren(node, scope);
  }

  /**
   * Expand a top-level lattice_rule node.
   *
   * lattice_rule = variable_declaration | mixin_definition | function_definition
   *              | use_directive | lattice_control ;
   *
   * Definitions (variable, mixin, function, use) have already been processed in
   * Pass 1 and should produce no output — but they may still be in the tree
   * if `_collectSymbols` didn't fully clean them up.
   *
   * lattice_control (@if, @for, @each) must be expanded here.
   */
  private _expandTopLevelLatticeRule(
    node: ASTNode,
    scope: ScopeChain
  ): ASTNode | Token | Array<ASTNode | Token> | null {
    const children = getChildren(node);
    if (children.length === 0) return null;

    const inner = children[0];
    if (!isASTNode(inner)) return null;

    const innerAst = inner as ASTNode;
    const rule = innerAst.ruleName;

    if (rule === "lattice_control") {
      // Expand and return all produced nodes
      return this._expandControl(innerAst, scope);
    }

    // variable_declaration, mixin_definition, function_definition, use_directive:
    // Already handled in Pass 1. Produce no CSS output.
    if (
      rule === "variable_declaration" ||
      rule === "mixin_definition" ||
      rule === "function_definition" ||
      rule === "use_directive"
    ) {
      return null;
    }

    return this._expandChildren(node, scope);
  }

  /**
   * Expand the top-level stylesheet node.
   *
   * stylesheet = { rule } ;
   *
   * Most rule expansion is straightforward. But when a top-level `rule`
   * wraps a `lattice_control` (@for, @each), expansion produces multiple
   * output nodes. This handler splices them in correctly.
   */
  private _expandStylesheet(node: ASTNode, scope: ScopeChain): ASTNode {
    const newChildren: Array<ASTNode | Token> = [];

    for (const child of getChildren(node)) {
      const expanded = this._expandNode(child, scope);
      if (expanded === null) {
        // Removed (variable, mixin, function definitions)
      } else if (Array.isArray(expanded)) {
        // Splice in all items (e.g., @for loop iterations)
        newChildren.push(...expanded);
      } else {
        newChildren.push(expanded as ASTNode | Token);
      }
    }

    setChildren(node, newChildren);
    return node;
  }

  /** Expand all children of a node (generic handler). */
  private _expandChildren(
    node: ASTNode,
    scope: ScopeChain
  ): ASTNode {
    const newChildren: Array<ASTNode | Token> = [];

    for (const child of getChildren(node)) {
      const expanded = this._expandNode(child, scope);
      if (expanded !== null) {
        if (Array.isArray(expanded)) {
          newChildren.push(...expanded);
        } else {
          newChildren.push(expanded as ASTNode | Token);
        }
      }
    }

    setChildren(node, newChildren);
    return node;
  }

  /**
   * Replace a VARIABLE token with its resolved value.
   *
   * If the variable is bound to a value_list node, we return the node.
   * If bound to a LatticeValue, we create a synthetic token.
   */
  private _substituteVariable(
    token: Token,
    scope: ScopeChain
  ): ASTNode | Token {
    const name = token.value;
    const value = scope.get(name);

    if (value === undefined) {
      throw new UndefinedVariableError(
        name,
        token.line ?? 0,
        token.column ?? 0
      );
    }

    // If the value is an AST node (value_list), deep-copy and expand it
    if (value !== null && typeof value === "object" && "ruleName" in value) {
      const cloned = deepClone(value as ASTNode);
      const expanded = this._expandNode(cloned, scope);
      if (expanded === null) return makeSyntheticToken("", token);
      if (Array.isArray(expanded)) return expanded[0] as ASTNode | Token;
      return expanded as ASTNode | Token;
    }

    // If it's a LatticeValue, convert to a synthetic token
    if (value !== null && typeof value === "object" && "kind" in value) {
      const cssText = valueToCss(value as LatticeValue);
      return makeSyntheticToken(cssText, token);
    }

    return token;
  }

  /** Expand a block, creating a child scope. */
  private _expandBlock(node: ASTNode, scope: ScopeChain): ASTNode {
    const childScope = scope.child();
    return this._expandChildren(node, childScope);
  }

  /**
   * Expand block_contents, handling Lattice block items.
   *
   * Block items can include variable_declarations, @include directives,
   * and control flow (@if, @for, @each). These are expanded and their
   * results spliced into the children list.
   */
  private _expandBlockContents(node: ASTNode, scope: ScopeChain): ASTNode {
    const newChildren: Array<ASTNode | Token> = [];

    for (const child of getChildren(node)) {
      const expanded = this._expandBlockItemInner(child, scope);
      if (expanded === null) {
        // Removed (e.g., variable declaration)
      } else if (Array.isArray(expanded)) {
        newChildren.push(...expanded);
      } else {
        newChildren.push(expanded);
      }
    }

    setChildren(node, newChildren);
    return node;
  }

  /** Process a single child of block_contents during expansion. */
  private _expandBlockItemInner(
    child: ASTNode | Token,
    scope: ScopeChain
  ): ASTNode | Token | Array<ASTNode | Token> | null {
    if (!isASTNode(child)) return child;

    const ast = child as ASTNode;

    if (ast.ruleName === "block_item") {
      const innerChildren = getChildren(ast);
      if (innerChildren.length > 0 && isASTNode(innerChildren[0])) {
        const innerAst = innerChildren[0] as ASTNode;

        if (innerAst.ruleName === "lattice_block_item") {
          const result = this._expandLatticeBlockItem(innerAst, scope);
          if (result === null) return null;
          if (Array.isArray(result)) return result;
          setChildren(ast, [innerAst]);
          setChildren(innerAst, [result as ASTNode | Token]);
          return ast;
        }

        // Lattice v2: handle property_nesting inside declaration_or_nested
        if (innerAst.ruleName === "declaration_or_nested") {
          const donChildren = getChildren(innerAst);
          if (donChildren.length > 0 && isASTNode(donChildren[0])) {
            if ((donChildren[0] as ASTNode).ruleName === "property_nesting") {
              const result = this._expandPropertyNesting(donChildren[0] as ASTNode, scope);
              return result.length > 0 ? result : null;
            }
          }
        }
      }
      return this._expandChildren(ast, scope);
    }

    return this._expandChildren(ast, scope);
  }

  /** Expand a block_item node. */
  private _expandBlockItem(
    node: ASTNode,
    scope: ScopeChain
  ): ASTNode | Token | Array<ASTNode | Token> | null {
    const children = getChildren(node);
    if (children.length === 0) return node;

    const inner = children[0];
    if (!isASTNode(inner)) return this._expandChildren(node, scope);

    const innerAst = inner as ASTNode;

    if (innerAst.ruleName === "lattice_block_item") {
      const result = this._expandLatticeBlockItem(innerAst, scope);
      if (result === null) return null;
      if (Array.isArray(result)) return result;
      setChildren(node, [result as ASTNode | Token]);
      return node;
    }

    return this._expandChildren(node, scope);
  }

  /**
   * Expand a lattice_block_item.
   *
   * lattice_block_item = variable_declaration | include_directive | lattice_control
   *                    | content_directive | at_root_directive | extend_directive ;
   *
   * Lattice v2 adds: content_directive, at_root_directive, extend_directive.
   */
  private _expandLatticeBlockItem(
    node: ASTNode,
    scope: ScopeChain
  ): ASTNode | Token | Array<ASTNode | Token> | null {
    const children = getChildren(node);
    if (children.length === 0) return node;

    const inner = children[0];
    if (!isASTNode(inner)) return node;

    const innerAst = inner as ASTNode;
    const rule = innerAst.ruleName;

    if (rule === "variable_declaration") {
      this._expandVariableDeclaration(innerAst, scope);
      return null; // Remove from output
    } else if (rule === "include_directive") {
      return this._expandInclude(innerAst, scope);
    } else if (rule === "lattice_control") {
      return this._expandControl(innerAst, scope);
    } else if (rule === "content_directive") {
      return this._expandContent(scope);
    } else if (rule === "at_root_directive") {
      return this._expandAtRoot(innerAst, scope);
    } else if (rule === "extend_directive") {
      this._collectExtend(innerAst);
      return null; // Remove from output
    }

    return this._expandChildren(node, scope);
  }

  /**
   * Process a variable_declaration inside a block.
   *
   * Sets the variable in the current scope. The node is removed from output.
   *
   * Lattice v2: handles !default and !global flags.
   */
  private _expandVariableDeclaration(node: ASTNode, scope: ScopeChain): void {
    let name: string | undefined;
    let valueNode: ASTNode | undefined;
    let isDefault = false;
    let isGlobal = false;

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) {
        const typeName = tokenTypeName(child as Token);
        if (typeName === "VARIABLE") {
          name = (child as Token).value;
        } else if (typeName === "BANG_DEFAULT") {
          isDefault = true;
        } else if (typeName === "BANG_GLOBAL") {
          isGlobal = true;
        }
      } else {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "value_list") {
          valueNode = childAst;
        } else if (childAst.ruleName === "variable_flag") {
          for (const fc of getChildren(childAst)) {
            if (!isASTNode(fc)) {
              const ft = tokenTypeName(fc as Token);
              if (ft === "BANG_DEFAULT") isDefault = true;
              else if (ft === "BANG_GLOBAL") isGlobal = true;
            }
          }
        }
      }
    }

    if (name && valueNode) {
      let expandedValue = this._expandNode(deepClone(valueNode), scope);
      let value: unknown = expandedValue ?? valueNode;

      // Try to evaluate as an expression (e.g. $i + 1 → LatticeNumber(2)).
      // This is critical for @while loops: without it, $i: $i + 1
      // stores unevaluated tokens instead of the computed number, causing
      // the loop condition to never change and looping forever.
      try {
        const evaluator = new ExpressionEvaluator(scope);
        const evaluated = evaluator.evaluate(
          deepClone((expandedValue ?? valueNode) as ASTNode)
        );
        if (evaluated !== null && evaluated !== undefined) {
          // Store the LatticeValue directly so _substituteVariable can
          // convert it via the "kind" in value check.
          value = evaluated;
        }
      } catch {
        // Not a pure expression (e.g. Helvetica, sans-serif) — keep AST
      }

      if (isDefault && isGlobal) {
        let root: ScopeChain = scope;
        while (root.parent !== null) root = root.parent;
        if (root.get(name) === undefined) {
          scope.setGlobal(name, value);
        }
      } else if (isDefault) {
        if (scope.get(name) === undefined) {
          scope.set(name, value);
        }
      } else if (isGlobal) {
        scope.setGlobal(name, value);
      } else {
        scope.set(name, value);
      }
    }
  }

  /** Expand variables within a value_list. */
  private _expandValueList(node: ASTNode, scope: ScopeChain): ASTNode {
    const newChildren: Array<ASTNode | Token> = [];

    for (const child of getChildren(node)) {
      const expanded = this._expandNode(child, scope);
      if (expanded !== null) {
        if (Array.isArray(expanded)) {
          newChildren.push(...expanded);
        } else {
          // If expansion returns a value_list, splice its children
          const exp = expanded as ASTNode | Token;
          if (isASTNode(exp) && (exp as ASTNode).ruleName === "value_list") {
            newChildren.push(...getChildren(exp as ASTNode));
          } else {
            newChildren.push(exp);
          }
        }
      }
    }

    setChildren(node, newChildren);
    return node;
  }

  /** Expand a single value node. */
  private _expandValue(
    node: ASTNode,
    scope: ScopeChain
  ): ASTNode | Token {
    const children = getChildren(node);
    if (children.length === 0) return node;

    // Check if it's a VARIABLE token
    if (children.length === 1 && !isASTNode(children[0])) {
      const token = children[0] as Token;
      if (tokenTypeName(token) === "VARIABLE") {
        const result = this._substituteVariable(token, scope);
        if (isASTNode(result) && (result as ASTNode).ruleName === "value_list") {
          return result as ASTNode; // Return the value_list directly
        }
        setChildren(node, [result]);
        return node;
      }
    }

    return this._expandChildren(node, scope);
  }

  /**
   * Expand a function_call node.
   *
   * If the function is a CSS built-in (rgb, calc, etc.), pass through.
   * If it's a Lattice function, evaluate it and replace with the return value.
   */
  private _expandFunctionCall(
    node: ASTNode,
    scope: ScopeChain
  ): ASTNode | Token {
    const children = getChildren(node);

    // Find the FUNCTION token to get the name
    let funcName: string | undefined;
    for (const child of children) {
      if (!isASTNode(child) && tokenTypeName(child as Token) === "FUNCTION") {
        funcName = (child as Token).value.replace(/\($/, "");
        break;
      }
    }

    // URL_TOKEN or no function name — pass through
    if (funcName === undefined) {
      return this._expandChildren(node, scope);
    }

    // User-defined function ALWAYS takes priority — even over CSS built-ins
    if (this.functions.has(funcName)) {
      return this._evaluateFunctionCall(funcName, node, scope);
    }

    // CSS built-in that is NOT also a Lattice built-in — pass through
    if (isCssFunction(funcName) && !BUILTIN_FUNCTIONS.has(funcName)) {
      return this._expandChildren(node, scope);
    }

    // Lattice v2 built-in function
    if (BUILTIN_FUNCTIONS.has(funcName)) {
      return this._evaluateBuiltinFunction(funcName, node, scope);
    }

    // CSS built-in that overlaps with Lattice built-in names
    if (isCssFunction(funcName)) {
      return this._expandChildren(node, scope);
    }


    // Unknown function — pass through (might be a CSS function we don't know)
    return this._expandChildren(node, scope);
  }

  // ===========================================================================
  // @include Expansion
  // ===========================================================================

  /**
   * Expand an @include directive by cloning the mixin body.
   *
   * include_directive = "@include" FUNCTION include_args RPAREN ( SEMICOLON | block )
   *                   | "@include" IDENT ( SEMICOLON | block ) ;
   *
   * Lattice v2: if @include has a trailing block (not SEMICOLON), that block
   * is the content block -- it replaces @content; in the mixin body.
   * The content block is evaluated in the caller's scope.
   */
  private _expandInclude(
    node: ASTNode,
    scope: ScopeChain
  ): Array<ASTNode | Token> {
    const children = getChildren(node);
    let mixinName: string | undefined;
    let argsNode: ASTNode | undefined;
    let contentBlock: ASTNode | null = null;

    for (const child of children) {
      if (!isASTNode(child)) {
        const token = child as Token;
        const type = tokenTypeName(token);
        if (type === "FUNCTION") {
          mixinName = token.value.replace(/\($/, "");
        } else if (type === "IDENT") {
          mixinName = token.value;
        }
      } else {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "include_args") {
          argsNode = childAst;
        } else if (childAst.ruleName === "block") {
          contentBlock = childAst;
        }
      }
    }

    if (mixinName === undefined) return [];

    if (!this.mixins.has(mixinName)) {
      throw new UndefinedMixinError(mixinName);
    }

    // Cycle detection
    if (this.mixinStack.includes(mixinName)) {
      throw new CircularReferenceError(
        "mixin",
        [...this.mixinStack, mixinName]
      );
    }

    const mixinDef = this.mixins.get(mixinName)!;

    // Parse arguments — returns positional list and named map
    const { positional, named } = argsNode
      ? this._parseIncludeArgs(argsNode)
      : { positional: [] as Array<ASTNode | Token>, named: new Map<string, ASTNode | Token>() };

    // Check arity (named args count toward total)
    const totalArgs = positional.length + named.size;
    const required = mixinDef.params.length - mixinDef.defaults.size;
    if (totalArgs < required || totalArgs > mixinDef.params.length) {
      throw new WrongArityError(
        "Mixin",
        mixinName,
        mixinDef.params.length,
        totalArgs
      );
    }

    // Evaluate all arguments in the CALLER'S scope before binding them.
    // This prevents infinite recursion when a positional arg is a variable
    // with the same name as the mixin parameter (e.g. @include btn($color)
    // where the outer scope has $color bound: without pre-evaluation,
    // mixinScope.$color = value_list{VARIABLE($color)}, and expanding that
    // calls substituteVariable($color, mixinScope) → same value_list → loop).
    const evaluateArg = (argNode: ASTNode | Token): ASTNode | Token => {
      const cloned = deepClone(argNode as ASTNode);
      const exp = this._expandNode(cloned, scope);
      if (exp === null) return argNode;
      if (Array.isArray(exp)) return exp[0] ?? argNode;
      return exp;
    };

    // Create child scope with params bound.
    // Named args take priority; remaining params are filled from positional args.
    const mixinScope = scope.child();
    let posIdx = 0;
    for (let i = 0; i < mixinDef.params.length; i++) {
      const paramName = mixinDef.params[i];
      if (named.has(paramName)) {
        mixinScope.set(paramName, evaluateArg(named.get(paramName)!));
      } else if (posIdx < positional.length) {
        mixinScope.set(paramName, evaluateArg(positional[posIdx++]));
      } else if (mixinDef.defaults.has(paramName)) {
        mixinScope.set(paramName, deepClone(mixinDef.defaults.get(paramName)! as ASTNode));
      }
    }

    // Lattice v2: push content block and caller scope for @content
    this.contentBlockStack.push(contentBlock);
    this.contentScopeStack.push(scope);

    // Clone and expand the mixin body
    this.mixinStack.push(mixinName);
    try {
      const bodyClone = deepClone(mixinDef.body as ASTNode);
      const expanded = this._expandNode(bodyClone, mixinScope);

      // Extract block_contents children from the expanded block
      const expandedAst = (Array.isArray(expanded) ? expanded[0] : expanded) as ASTNode;
      if (expandedAst && isASTNode(expandedAst)) {
        for (const child of getChildren(expandedAst)) {
          if (isASTNode(child) && (child as ASTNode).ruleName === "block_contents") {
            return getChildren(child as ASTNode).filter((c) => c !== null);
          }
        }
      }

      return [];
    } finally {
      this.mixinStack.pop();
      this.contentBlockStack.pop();
      this.contentScopeStack.pop();
    }
  }

  /**
   * Parse include_args into positional and named argument collections.
   *
   * include_args = include_arg { COMMA include_arg } ;
   * include_arg  = VARIABLE COLON value_list | value_list ;
   *
   * Named args ($param: value) are collected into the `named` map keyed by
   * the full variable name (e.g. "$gap"). Unnamed args go into `positional`.
   *
   * Legacy: if include_args still contains bare value_list children (grammar
   * before named-arg support), those are treated as positional. A single
   * value_list that contains commas is split on those commas.
   */
  private _parseIncludeArgs(
    node: ASTNode
  ): { positional: Array<ASTNode | Token>; named: Map<string, ASTNode | Token> } {
    const positional: Array<ASTNode | Token> = [];
    const named = new Map<string, ASTNode | Token>();

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) continue;
      const childAst = child as ASTNode;

      if (childAst.ruleName === "include_arg") {
        // Named arg: first child is VARIABLE, second is COLON, third is value_list
        const argChildren = getChildren(childAst);
        if (
          argChildren.length >= 3 &&
          !isASTNode(argChildren[0]) &&
          tokenTypeName(argChildren[0] as Token) === "VARIABLE" &&
          !isASTNode(argChildren[1]) &&
          tokenTypeName(argChildren[1] as Token) === "COLON"
        ) {
          const varName = (argChildren[0] as Token).value; // e.g. "$gap"
          // The value is the value_list node (third child)
          const valueNode = argChildren[2];
          named.set(varName, valueNode as ASTNode | Token);
        } else {
          // Positional: the include_arg wraps a value_list
          const valueList = argChildren.find(
            (c) => isASTNode(c) && (c as ASTNode).ruleName === "value_list"
          ) as ASTNode | undefined;
          if (valueList) positional.push(valueList);
        }
      } else if (childAst.ruleName === "value_list") {
        // Legacy: bare value_list (old grammar without include_arg wrapper)
        positional.push(childAst);
      }
    }

    // Legacy: if we collected only one positional value_list and it contains
    // comma tokens, split it into multiple positional args.
    if (positional.length === 1 && named.size === 0) {
      const split = this._splitValueListOnCommas(positional[0] as ASTNode);
      if (split.length > 1) {
        return { positional: split, named };
      }
    }

    return { positional, named };
  }

  /**
   * Split a value_list into multiple value_lists at COMMA boundaries.
   *
   * If value_list contains value nodes with COMMA tokens, split them:
   *   [red, COMMA, white] → [[red], [white]]
   */
  private _splitValueListOnCommas(node: ASTNode): Array<ASTNode | Token> {
    const children = getChildren(node);

    // Check if any child value node contains a COMMA
    let hasComma = false;
    for (const child of children) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "value") {
        for (const vc of getChildren(child as ASTNode)) {
          if (!isASTNode(vc) && tokenTypeName(vc as Token) === "COMMA") {
            hasComma = true;
            break;
          }
        }
      }
    }

    if (!hasComma) return [node];

    // Split on comma value nodes
    const groups: Array<Array<ASTNode | Token>> = [[]];
    for (const child of children) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "value") {
        const inner = getChildren(child as ASTNode);
        if (
          inner.length === 1 &&
          !isASTNode(inner[0]) &&
          tokenTypeName(inner[0] as Token) === "COMMA"
        ) {
          groups.push([]);
          continue;
        }
      }
      groups[groups.length - 1].push(child);
    }

    // Create new value_list nodes for each group
    return groups
      .filter((g) => g.length > 0)
      .map((g) => new SimpleASTNode("value_list", g));
  }

  // ===========================================================================
  // Control Flow
  // ===========================================================================

  /**
   * Expand a lattice_control node.
   *
   * lattice_control = if_directive | for_directive | each_directive
   *                 | while_directive ;
   *
   * Lattice v2 adds while_directive.
   */
  private _expandControl(
    node: ASTNode,
    scope: ScopeChain
  ): Array<ASTNode | Token> | null {
    const children = getChildren(node);
    if (children.length === 0) return null;

    const inner = children[0];
    if (!isASTNode(inner)) return null;

    const innerAst = inner as ASTNode;

    switch (innerAst.ruleName) {
      case "if_directive":
        return this._expandIf(innerAst, scope);
      case "for_directive":
        return this._expandFor(innerAst, scope);
      case "each_directive":
        return this._expandEach(innerAst, scope);
      case "while_directive":
        return this._expandWhile(innerAst, scope);
    }

    return null;
  }

  /**
   * Expand an @if / @else if / @else directive.
   *
   * if_directive = "@if" lattice_expression block
   *                { "@else" "if" lattice_expression block }
   *                [ "@else" block ] ;
   *
   * Evaluates expressions and expands the matching branch.
   */
  private _expandIf(
    node: ASTNode,
    scope: ScopeChain
  ): Array<ASTNode | Token> {
    const children = getChildren(node);

    // Parse the if/else-if/else structure
    type Branch = { condition: ASTNode | Token | null; block: ASTNode };
    const branches: Branch[] = [];

    let i = 0;
    while (i < children.length) {
      const child = children[i];
      const val = getTokenValue(child);

      if (val === "@if") {
        const expr = children[i + 1];
        const block = children[i + 2];
        if (block && isASTNode(block)) {
          branches.push({ condition: expr, block: block as ASTNode });
        }
        i += 3;
      } else if (val === "@else") {
        // Check if next is "if"
        if (i + 1 < children.length && getTokenValue(children[i + 1]) === "if") {
          const expr = children[i + 2];
          const block = children[i + 3];
          if (block && isASTNode(block)) {
            branches.push({ condition: expr, block: block as ASTNode });
          }
          i += 4;
        } else {
          const block = children[i + 1];
          if (block && isASTNode(block)) {
            branches.push({ condition: null, block: block as ASTNode });
          }
          i += 2;
        }
      } else {
        i++;
      }
    }

    // Evaluate branches
    const evaluator = new ExpressionEvaluator(scope);
    for (const { condition, block } of branches) {
      if (condition === null) {
        // @else — always matches
        return this._expandBlockToItems(block, scope);
      } else {
        const result = evaluator.evaluate(condition);
        if (isTruthy(result)) {
          return this._expandBlockToItems(block, scope);
        }
      }
    }

    return [];
  }

  /**
   * Expand a @for loop.
   *
   * for_directive = "@for" VARIABLE "from" lattice_expression
   *                 ( "through" | "to" ) lattice_expression block ;
   */
  private _expandFor(
    node: ASTNode,
    scope: ScopeChain
  ): Array<ASTNode | Token> {
    const children = getChildren(node);

    let varName: string | undefined;
    let fromExpr: ASTNode | Token | undefined;
    let toExpr: ASTNode | Token | undefined;
    let isThrough = false;
    let block: ASTNode | undefined;

    let i = 0;
    while (i < children.length) {
      const child = children[i];
      const val = getTokenValue(child);

      if (val !== undefined && !isASTNode(child) && tokenTypeName(child as Token) === "VARIABLE") {
        varName = val;
      } else if (val === "from") {
        fromExpr = children[i + 1];
        i++;
      } else if (val === "through") {
        isThrough = true;
        toExpr = children[i + 1];
        i++;
      } else if (val === "to") {
        isThrough = false;
        toExpr = children[i + 1];
        i++;
      } else if (isASTNode(child) && (child as ASTNode).ruleName === "block") {
        block = child as ASTNode;
      }

      i++;
    }

    if (!varName || !fromExpr || !toExpr || !block) return [];

    const evaluator = new ExpressionEvaluator(scope);
    const fromVal = evaluator.evaluate(fromExpr);
    const toVal = evaluator.evaluate(toExpr);

    // Extract numeric values
    const fromNum = fromVal.kind === "number" ? Math.trunc(fromVal.value) : 0;
    const toNum = toVal.kind === "number" ? Math.trunc(toVal.value) : 0;

    const end = isThrough ? toNum + 1 : toNum; // through is inclusive, to is exclusive

    const result: Array<ASTNode | Token> = [];
    for (let iVal = fromNum; iVal < end; iVal++) {
      const loopScope = scope.child();
      loopScope.set(varName, new LatticeNumber(iVal));

      const expanded = this._expandBlockToItems(deepClone(block), loopScope);
      result.push(...expanded);
    }

    return result;
  }

  /**
   * Expand a @each loop.
   *
   * each_directive = "@each" VARIABLE { COMMA VARIABLE } "in"
   *                  each_list block ;
   *
   * Lattice v2: when iterating over a map with two variables
   * (@each $key, $value in $map), the first variable gets the key
   * and the second gets the value for each entry.
   */
  private _expandEach(
    node: ASTNode,
    scope: ScopeChain
  ): Array<ASTNode | Token> {
    const children = getChildren(node);

    const varNames: string[] = [];
    let eachList: ASTNode | undefined;
    let block: ASTNode | undefined;

    for (const child of children) {
      if (!isASTNode(child)) {
        const token = child as Token;
        if (tokenTypeName(token) === "VARIABLE") {
          varNames.push(token.value);
        }
      } else {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "each_list") {
          eachList = childAst;
        } else if (childAst.ruleName === "block") {
          block = childAst;
        }
      }
    }

    if (varNames.length === 0 || !eachList || !block) return [];

    // Lattice v2: check if the each_list references a map or list variable
    const resolved = this._resolveEachList(eachList, scope);
    if (resolved !== null) {
      return this._expandEachOverResolved(varNames, resolved, block, scope);
    }

    // Extract list items from each_list
    // each_list = value { COMMA value } ;
    const items: Array<ASTNode | Token> = [];
    for (const child of getChildren(eachList)) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "value") {
        items.push(child as ASTNode);
      }
    }

    const result: Array<ASTNode | Token> = [];
    for (const item of items) {
      const loopScope = scope.child();
      // Bind the first variable to the item's value
      if (varNames.length > 0) {
        const itemValue = this._extractValueToken(item);
        loopScope.set(varNames[0], itemValue);
      }

      const expanded = this._expandBlockToItems(deepClone(block), loopScope);
      result.push(...expanded);
    }

    return result;
  }

  /**
   * Try to resolve an each_list to a LatticeValue (map or list).
   *
   * If the each_list contains a single VARIABLE that resolves to a
   * LatticeMap or LatticeList, return it.
   *
   * If the variable is stored as an AST node (e.g. value_list containing a
   * map_literal), convert it on-the-fly to a LatticeMap. This handles the
   * common pattern:
   *
   *   $colors: (red: #f00, blue: #00f);
   *   @each $name, $color in $colors { ... }
   *
   * where $colors is stored as the raw value_list AST from Pass 1.
   */
  private _resolveEachList(eachList: ASTNode, scope: ScopeChain): LatticeValue | null {
    const varTokens: Token[] = [];
    for (const child of getChildren(eachList)) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "value") {
        for (const vc of getChildren(child as ASTNode)) {
          if (!isASTNode(vc) && tokenTypeName(vc as Token) === "VARIABLE") {
            varTokens.push(vc as Token);
          }
        }
      }
    }
    if (varTokens.length === 1) {
      const val = scope.get(varTokens[0].value);
      if (val !== undefined && val !== null && typeof val === "object" && "kind" in val) {
        const lv = val as LatticeValue;
        if (lv.kind === "map" || lv.kind === "list") return lv;
      }
      // Variable stored as an AST node — check if it wraps a map_literal
      if (val !== undefined && val !== null && typeof val === "object" && "ruleName" in val) {
        const mapLit = this._findMapLiteralInAst(val as ASTNode);
        if (mapLit) {
          return this._convertMapLiteralToLatticeMap(mapLit, scope);
        }
      }
    }
    return null;
  }

  /**
   * Recursively search for a map_literal node within an AST subtree.
   * Used to unwrap value_list → value → map_literal nesting.
   */
  private _findMapLiteralInAst(node: ASTNode): ASTNode | null {
    if (node.ruleName === "map_literal") return node;
    for (const child of getChildren(node)) {
      if (isASTNode(child)) {
        const found = this._findMapLiteralInAst(child as ASTNode);
        if (found) return found;
      }
    }
    return null;
  }

  /**
   * Convert a map_literal AST node to a LatticeMap.
   *
   * map_literal = LPAREN map_entry COMMA map_entry { COMMA map_entry } RPAREN ;
   * map_entry   = ( IDENT | STRING ) COLON lattice_expression ;
   *
   * Each entry's value expression is evaluated in the given scope so that
   * variable references resolve correctly:
   *   $colors: (primary: $accent, secondary: $panel-bg);
   */
  private _convertMapLiteralToLatticeMap(node: ASTNode, scope: ScopeChain): LatticeMap {
    const items: Array<readonly [string, LatticeValue]> = [];
    const evaluator = new ExpressionEvaluator(scope);

    for (const child of getChildren(node)) {
      if (!isASTNode(child) || (child as ASTNode).ruleName !== "map_entry") continue;

      let key: string | undefined;
      let valueExpr: ASTNode | undefined;

      for (const ec of getChildren(child as ASTNode)) {
        if (!isASTNode(ec)) {
          const t = ec as Token;
          const tn = tokenTypeName(t);
          if ((tn === "IDENT" || tn === "STRING") && key === undefined) {
            // Strip surrounding quotes from string keys
            key = t.value.replace(/^"|"$/g, "").replace(/^'|'$/g, "");
          }
        } else {
          const ecAst = ec as ASTNode;
          if (ecAst.ruleName === "lattice_expression" && valueExpr === undefined) {
            valueExpr = ecAst;
          }
        }
      }

      if (key !== undefined && valueExpr !== undefined) {
        const value = evaluator.evaluate(valueExpr);
        items.push([key, value] as const);
      }
    }

    return new LatticeMap(items);
  }

  /** Expand @each over a resolved LatticeMap or LatticeList. */
  private _expandEachOverResolved(
    varNames: string[],
    collection: LatticeValue,
    block: ASTNode,
    scope: ScopeChain
  ): Array<ASTNode | Token> {
    const result: Array<ASTNode | Token> = [];

    if (collection.kind === "map") {
      for (const [key, value] of collection.items) {
        const loopScope = scope.child();
        loopScope.set(varNames[0], new LatticeIdent(key));
        if (varNames.length >= 2) {
          loopScope.set(varNames[1], value);
        }
        result.push(...this._expandBlockToItems(deepClone(block), loopScope));
      }
    } else if (collection.kind === "list") {
      for (const item of collection.items) {
        const loopScope = scope.child();
        loopScope.set(varNames[0], item);
        result.push(...this._expandBlockToItems(deepClone(block), loopScope));
      }
    }

    return result;
  }

  /** Extract the meaningful content from a value node. */
  private _extractValueToken(node: ASTNode | Token): LatticeValue | ASTNode | Token {
    if (isASTNode(node)) {
      const children = getChildren(node as ASTNode);
      if (children.length === 1 && !isASTNode(children[0])) {
        return tokenToValue(children[0] as Token);
      }
    }
    return node;
  }

  /** Expand a block and return its block_contents children. */
  private _expandBlockToItems(
    block: ASTNode,
    scope: ScopeChain
  ): Array<ASTNode | Token> {
    const expanded = this._expandNode(block, scope);
    const expandedAst = (Array.isArray(expanded) ? expanded[0] : expanded) as ASTNode;
    if (expandedAst && isASTNode(expandedAst)) {
      for (const child of getChildren(expandedAst)) {
        if (isASTNode(child) && (child as ASTNode).ruleName === "block_contents") {
          return getChildren(child as ASTNode).filter((c) => c !== null);
        }
      }
    }
    return [];
  }

  // ===========================================================================
  // Function Evaluation
  // ===========================================================================

  /**
   * Evaluate a Lattice function call and return the result.
   *
   * The function body is evaluated in an isolated scope (parent = globals).
   * @return signals the return value via ReturnSignal exception.
   */
  private _evaluateFunctionCall(
    funcName: string,
    node: ASTNode,
    scope: ScopeChain
  ): ASTNode | Token {
    const funcDef = this.functions.get(funcName)!;
    const children = getChildren(node);

    // Parse arguments from function_args
    let args: Array<ASTNode | Token | LatticeValue> = [];
    for (const child of children) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "function_args") {
        args = this._parseFunctionCallArgs(child as ASTNode);
        break;
      }
    }

    // Check arity
    const required = funcDef.params.length - funcDef.defaults.size;
    if (args.length < required || args.length > funcDef.params.length) {
      throw new WrongArityError(
        "Function",
        funcName,
        funcDef.params.length,
        args.length
      );
    }

    // Cycle detection
    if (this.functionStack.includes(funcName)) {
      throw new CircularReferenceError(
        "function",
        [...this.functionStack, funcName]
      );
    }

    // Create isolated scope (parent = global scope only)
    const funcScope = this.variables.child();
    for (let i = 0; i < funcDef.params.length; i++) {
      const paramName = funcDef.params[i];
      if (i < args.length) {
        funcScope.set(paramName, args[i]);
      } else if (funcDef.defaults.has(paramName)) {
        funcScope.set(paramName, deepClone(funcDef.defaults.get(paramName)! as ASTNode));
      }
    }

    // Evaluate the function body
    this.functionStack.push(funcName);
    try {
      const bodyClone = deepClone(funcDef.body as ASTNode);
      try {
        this._evaluateFunctionBody(bodyClone, funcScope);
      } catch (e) {
        if (e instanceof ReturnSignal) {
          const cssText = valueToCss(e.value);
          return makeValueNode(cssText, node);
        }
        throw e;
      }

      throw new MissingReturnError(funcName);
    } finally {
      this.functionStack.pop();
    }
  }

  /**
   * Evaluate function body statements.
   *
   * function_body = LBRACE { function_body_item } RBRACE ;
   * function_body_item = variable_declaration | return_directive | lattice_control ;
   */
  private _evaluateFunctionBody(body: ASTNode, scope: ScopeChain): void {
    if (!isASTNode(body)) return;

    for (const child of getChildren(body)) {
      if (!isASTNode(child)) continue;

      const childAst = child as ASTNode;

      if (childAst.ruleName === "function_body_item") {
        const innerChildren = getChildren(childAst);
        if (innerChildren.length === 0) continue;

        const inner = innerChildren[0];
        if (!isASTNode(inner)) continue;

        const innerAst = inner as ASTNode;

        if (innerAst.ruleName === "variable_declaration") {
          this._expandVariableDeclaration(innerAst, scope);
        } else if (innerAst.ruleName === "return_directive") {
          this._evaluateReturn(innerAst, scope);
        } else if (innerAst.ruleName === "lattice_control") {
          this._evaluateControlInFunction(innerAst, scope);
        }
      } else {
        // Recurse for nested structures
        this._evaluateFunctionBody(childAst, scope);
      }
    }
  }

  /**
   * Evaluate a @return directive.
   *
   * return_directive = "@return" lattice_expression SEMICOLON ;
   */
  private _evaluateReturn(node: ASTNode, scope: ScopeChain): never {
    for (const child of getChildren(node)) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "lattice_expression") {
        const evaluator = new ExpressionEvaluator(scope);
        const result = evaluator.evaluate(child as ASTNode);
        throw new ReturnSignal(result);
      }
    }
    throw new ReturnSignal(new LatticeNull());
  }

  /**
   * Evaluate control flow inside a function body.
   *
   * Similar to _expandControl but operates within function evaluation context,
   * where @return can signal a function return.
   */
  private _evaluateControlInFunction(node: ASTNode, scope: ScopeChain): void {
    const children = getChildren(node);
    if (children.length === 0) return;

    const inner = children[0];
    if (!isASTNode(inner)) return;

    const innerAst = inner as ASTNode;

    if (innerAst.ruleName === "if_directive") {
      this._evaluateIfInFunction(innerAst, scope);
    }
  }

  /** Evaluate @if inside a function body. */
  private _evaluateIfInFunction(node: ASTNode, scope: ScopeChain): void {
    const children = getChildren(node);

    type Branch = { condition: ASTNode | Token | null; block: ASTNode };
    const branches: Branch[] = [];

    let i = 0;
    while (i < children.length) {
      const child = children[i];
      const val = getTokenValue(child);

      if (val === "@if") {
        const expr = children[i + 1];
        const block = children[i + 2];
        if (block && isASTNode(block)) {
          branches.push({ condition: expr, block: block as ASTNode });
        }
        i += 3;
      } else if (val === "@else") {
        if (i + 1 < children.length && getTokenValue(children[i + 1]) === "if") {
          const expr = children[i + 2];
          const block = children[i + 3];
          if (block && isASTNode(block)) {
            branches.push({ condition: expr, block: block as ASTNode });
          }
          i += 4;
        } else {
          const block = children[i + 1];
          if (block && isASTNode(block)) {
            branches.push({ condition: null, block: block as ASTNode });
          }
          i += 2;
        }
      } else {
        i++;
      }
    }

    const evaluator = new ExpressionEvaluator(scope);
    for (const { condition, block } of branches) {
      if (condition === null || isTruthy(evaluator.evaluate(condition))) {
        // Evaluate the block — look for @return
        this._evaluateBlockInFunction(block, scope);
        return;
      }
    }
  }

  /**
   * Evaluate a block inside a function, handling @return at-rules.
   *
   * When @return appears inside @if blocks within a function, the grammar
   * parses it as an at_rule (not return_directive), because return_directive
   * is only valid in function_body_item. We detect @return at-rules here.
   */
  private _evaluateBlockInFunction(block: ASTNode, scope: ScopeChain): void {
    if (!isASTNode(block)) return;

    for (const child of getChildren(block)) {
      if (!isASTNode(child)) continue;

      const childAst = child as ASTNode;

      if (childAst.ruleName === "block_contents") {
        this._evaluateBlockInFunction(childAst, scope);
      } else if (childAst.ruleName === "block_item") {
        const innerChildren = getChildren(childAst);
        if (innerChildren.length > 0 && isASTNode(innerChildren[0])) {
          const inner = innerChildren[0] as ASTNode;
          if (inner.ruleName === "at_rule") {
            this._maybeEvaluateReturnAtRule(inner, scope);
          } else if (inner.ruleName === "lattice_block_item") {
            for (const lbc of getChildren(inner)) {
              if (isASTNode(lbc) && (lbc as ASTNode).ruleName === "variable_declaration") {
                this._expandVariableDeclaration(lbc as ASTNode, scope);
              }
            }
          }
        }
      }
    }
  }

  /**
   * Check if an at_rule is actually @return, and evaluate it if so.
   *
   * at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block ) ;
   *
   * If AT_KEYWORD value is "@return", extract the expression from
   * at_prelude and evaluate it.
   */
  private _maybeEvaluateReturnAtRule(node: ASTNode, scope: ScopeChain): void {
    let keyword: string | undefined;
    let prelude: ASTNode | undefined;

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) {
        if (tokenTypeName(child as Token) === "AT_KEYWORD") {
          keyword = (child as Token).value;
        }
      } else if ((child as ASTNode).ruleName === "at_prelude") {
        prelude = child as ASTNode;
      }
    }

    if (keyword !== "@return" || !prelude) return;

    // Extract tokens from at_prelude to build an expression
    const tokens: Token[] = [];
    this._collectTokens(prelude, tokens);

    if (tokens.length === 0) {
      throw new ReturnSignal(new LatticeNull());
    }

    // For simple cases (single token), convert directly
    if (tokens.length === 1) {
      const token = tokens[0];
      if (tokenTypeName(token) === "VARIABLE") {
        const varVal = scope.get(token.value);
        if (varVal !== undefined) {
          if (varVal !== null && typeof varVal === "object" && "kind" in varVal) {
            throw new ReturnSignal(varVal as LatticeValue);
          }
          if (varVal !== null && typeof varVal === "object" && "ruleName" in varVal) {
            const extracted = extractValueFromAst(varVal as ASTNode);
            throw new ReturnSignal(extracted);
          }
        }
      }
      throw new ReturnSignal(tokenToValue(token));
    }

    // Fallback: use the first token
    throw new ReturnSignal(tokenToValue(tokens[0]));
  }

  /** Recursively collect all raw tokens from an AST node. */
  private _collectTokens(node: ASTNode | Token, tokens: Token[]): void {
    if (!isASTNode(node)) {
      tokens.push(node as Token);
      return;
    }
    for (const child of getChildren(node as ASTNode)) {
      this._collectTokens(child, tokens);
    }
  }

  /**
   * Parse function_args into individual argument values.
   *
   * function_args = { function_arg } ;
   * function_arg = ... | COMMA | ...
   *
   * Arguments are separated by COMMA tokens.
   */
  private _parseFunctionCallArgs(
    node: ASTNode
  ): Array<LatticeValue | ASTNode | Token> {
    const argGroups: Array<Array<LatticeValue | ASTNode | Token>> = [[]];

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) {
        if (tokenTypeName(child as Token) === "COMMA") {
          argGroups.push([]);
          continue;
        }
      }

      if (isASTNode(child) && (child as ASTNode).ruleName === "function_arg") {
        for (const ic of getChildren(child as ASTNode)) {
          if (!isASTNode(ic)) {
            if (tokenTypeName(ic as Token) === "COMMA") {
              argGroups.push([]);
              continue;
            }
            argGroups[argGroups.length - 1].push(tokenToValue(ic as Token));
          } else {
            argGroups[argGroups.length - 1].push(ic as ASTNode);
          }
        }
      }
    }

    // Convert each arg group to a single value
    const result: Array<LatticeValue | ASTNode | Token> = [];
    for (const group of argGroups) {
      if (group.length === 1) {
        result.push(group[0]);
      } else if (group.length > 1) {
        result.push(group[0]); // Take first for simplicity
      }
    }

    return result;
  }

  // ===========================================================================
  // Lattice v2: @while Loops
  // ===========================================================================

  /**
   * Expand a @while loop.
   *
   * while_directive = "@while" lattice_expression block ;
   *
   * Evaluates the condition; if truthy, expands the block body. Repeats
   * until the condition is falsy or max iterations exceeded.
   *
   * Variable scoping: @while uses the enclosing scope directly (not a child
   * scope). Variable mutations inside the body persist across iterations.
   */
  private _expandWhile(
    node: ASTNode,
    scope: ScopeChain
  ): Array<ASTNode | Token> {
    const children = getChildren(node);
    let condition: ASTNode | undefined;
    let block: ASTNode | undefined;

    for (const child of children) {
      if (isASTNode(child)) {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "lattice_expression") {
          condition = childAst;
        } else if (childAst.ruleName === "block") {
          block = childAst;
        }
      }
    }

    if (!condition || !block) return [];

    const result: Array<ASTNode | Token> = [];
    let iteration = 0;

    while (true) {
      const evaluator = new ExpressionEvaluator(scope);
      const condValue = evaluator.evaluate(deepClone(condition));

      if (!isTruthy(condValue)) break;

      iteration++;
      if (iteration > this.maxWhileIterations) {
        throw new MaxIterationError(this.maxWhileIterations);
      }

      const expanded = this._expandBlockToItems(deepClone(block), scope);
      result.push(...expanded);
    }

    return result;
  }

  // ===========================================================================
  // Lattice v2: Variables in Selectors
  // ===========================================================================

  /**
   * Resolve VARIABLE tokens in selector positions.
   *
   * When a VARIABLE token appears in a compound_selector, simple_selector,
   * or class_selector, resolve it to its string value and create a
   * synthetic IDENT token.
   */
  private _expandSelectorWithVars(
    node: ASTNode,
    scope: ScopeChain
  ): ASTNode {
    const newChildren: Array<ASTNode | Token> = [];

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) {
        const token = child as Token;
        if (tokenTypeName(token) === "VARIABLE") {
          const varName = token.value;
          const value = scope.get(varName);
          if (value === undefined) {
            throw new UndefinedVariableError(
              varName,
              token.line ?? 0,
              token.column ?? 0
            );
          }
          let cssText: string;
          if (value !== null && typeof value === "object" && "kind" in value) {
            cssText = valueToCss(value as LatticeValue);
          } else if (value !== null && typeof value === "object" && "ruleName" in value) {
            const v = extractValueFromAst(value as ASTNode);
            cssText = valueToCss(v);
          } else {
            cssText = String(value);
          }
          // Strip quotes from strings in selector context
          cssText = cssText.replace(/^"|"$/g, "").replace(/^'|'$/g, "");
          newChildren.push(makeSyntheticToken(cssText, token));
        } else {
          newChildren.push(child);
        }
      } else {
        // Recurse into child AST nodes
        const expanded = this._expandNode(child, scope);
        if (expanded !== null) {
          if (Array.isArray(expanded)) {
            newChildren.push(...expanded);
          } else {
            newChildren.push(expanded as ASTNode | Token);
          }
        }
      }
    }

    setChildren(node, newChildren);
    return node;
  }

  // ===========================================================================
  // Lattice v2: @content Blocks
  // ===========================================================================

  /**
   * Expand a @content directive inside a mixin body.
   *
   * Replaces @content; with the content block from the current @include call.
   * The content block is evaluated in the caller's scope, not the mixin's.
   * If no content block was passed, produces an empty list.
   */
  private _expandContent(
    scope: ScopeChain
  ): Array<ASTNode | Token> {
    if (this.contentBlockStack.length === 0) return [];

    const contentBlock = this.contentBlockStack[this.contentBlockStack.length - 1];
    if (contentBlock === null) return [];

    const callerScope = this.contentScopeStack.length > 0
      ? this.contentScopeStack[this.contentScopeStack.length - 1]
      : scope;

    return this._expandBlockToItems(deepClone(contentBlock), callerScope);
  }

  // ===========================================================================
  // Lattice v2: @at-root
  // ===========================================================================

  /**
   * Expand an @at-root directive.
   *
   * Rules inside @at-root are collected and hoisted to the stylesheet
   * root level during Pass 3. They are removed from the current
   * nesting context.
   */
  private _expandAtRoot(
    node: ASTNode,
    scope: ScopeChain
  ): null {
    const children = getChildren(node);
    let block: ASTNode | undefined;
    let selectorList: ASTNode | undefined;

    for (const child of children) {
      if (isASTNode(child)) {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "block") {
          block = childAst;
        } else if (childAst.ruleName === "selector_list") {
          selectorList = childAst;
        }
      }
    }

    if (!block) return null;

    if (selectorList) {
      // Inline form: @at-root .selector { ... }
      const expandedSel = this._expandNode(deepClone(selectorList), scope);
      const expandedBlock = this._expandNode(deepClone(block), scope);
      const qr = new SimpleASTNode("qualified_rule", [
        expandedSel as ASTNode | Token,
        expandedBlock as ASTNode | Token,
      ]);
      this.atRootRules.push(qr);
    } else {
      // Block form: @at-root { ... multiple rules ... }
      const expanded = this._expandBlockToItems(deepClone(block), scope);
      this.atRootRules.push(...expanded);
    }

    return null;
  }

  // ===========================================================================
  // Lattice v2: @extend and %placeholder
  // ===========================================================================

  /**
   * Collect an @extend directive for later selector merging.
   *
   * Records the extend relationship for Pass 3 processing.
   */
  private _collectExtend(node: ASTNode): void {
    let target = "";

    for (const child of getChildren(node)) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "extend_target") {
        const parts: string[] = [];
        for (const tc of getChildren(child as ASTNode)) {
          if (!isASTNode(tc)) parts.push((tc as Token).value);
        }
        target = parts.join("");
      }
    }

    if (target) {
      if (!this.extendMap.has(target)) {
        this.extendMap.set(target, []);
      }
    }
  }

  // ===========================================================================
  // Lattice v2: Property Nesting
  // ===========================================================================

  /**
   * Expand a property_nesting node.
   *
   * property_nesting = property COLON block ;
   *
   * Flattens nested property declarations by prepending the parent
   * property name with a hyphen to each child declaration's property.
   *
   * Example:
   *   font: { size: 14px; weight: bold; }
   *   => font-size: 14px; font-weight: bold;
   */
  private _expandPropertyNesting(
    node: ASTNode,
    scope: ScopeChain
  ): Array<ASTNode | Token> {
    let parentProp = "";
    let block: ASTNode | undefined;

    for (const child of getChildren(node)) {
      if (isASTNode(child)) {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "property") {
          for (const pc of getChildren(childAst)) {
            if (!isASTNode(pc)) parentProp = (pc as Token).value;
          }
        } else if (childAst.ruleName === "block") {
          block = childAst;
        }
      }
    }

    if (!parentProp || !block) return [];

    const expanded = this._expandNode(deepClone(block), scope);
    const result: Array<ASTNode | Token> = [];
    this._flattenNestedProps(expanded as ASTNode, parentProp, result);
    return result;
  }

  /** Recursively flatten nested property declarations. */
  private _flattenNestedProps(
    node: ASTNode | Token,
    prefix: string,
    result: Array<ASTNode | Token>
  ): void {
    if (!isASTNode(node)) return;
    for (const child of getChildren(node as ASTNode)) {
      if (!isASTNode(child)) continue;
      const childAst = child as ASTNode;
      if (childAst.ruleName === "block_contents") {
        this._flattenNestedProps(childAst, prefix, result);
      } else if (childAst.ruleName === "block_item") {
        this._flattenNestedBlockItem(childAst, prefix, result);
      } else if (childAst.ruleName === "declaration") {
        this._rewriteDeclarationPrefix(childAst, prefix, result);
      }
    }
  }

  /** Process a block_item inside a property nesting block. */
  private _flattenNestedBlockItem(
    node: ASTNode,
    prefix: string,
    result: Array<ASTNode | Token>
  ): void {
    const children = getChildren(node);
    if (children.length === 0) return;
    const inner = children[0];
    if (!isASTNode(inner)) return;
    const innerAst = inner as ASTNode;
    if (innerAst.ruleName === "declaration_or_nested") {
      for (const dc of getChildren(innerAst)) {
        if (isASTNode(dc)) {
          const dcAst = dc as ASTNode;
          if (dcAst.ruleName === "declaration") {
            this._rewriteDeclarationPrefix(dcAst, prefix, result);
          } else if (dcAst.ruleName === "property_nesting") {
            // Recursive nesting: accumulate prefix
            const subResults = this._expandPropertyNestingWithPrefix(dcAst, prefix);
            result.push(...subResults);
          }
        }
      }
    }
  }

  /** Rewrite a declaration's property name to include the prefix. */
  private _rewriteDeclarationPrefix(
    decl: ASTNode,
    prefix: string,
    result: Array<ASTNode | Token>
  ): void {
    for (const child of getChildren(decl)) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "property") {
        for (const pc of getChildren(child as ASTNode)) {
          if (!isASTNode(pc)) {
            const token = pc as Token;
            (token as { value: string }).value = `${prefix}-${token.value}`;
          }
        }
      }
    }
    result.push(decl);
  }

  /** Expand nested property_nesting with an accumulated prefix. */
  private _expandPropertyNestingWithPrefix(
    node: ASTNode,
    prefix: string
  ): Array<ASTNode | Token> {
    let subProp = "";
    let block: ASTNode | undefined;

    for (const child of getChildren(node)) {
      if (isASTNode(child)) {
        const childAst = child as ASTNode;
        if (childAst.ruleName === "property") {
          for (const pc of getChildren(childAst)) {
            if (!isASTNode(pc)) subProp = (pc as Token).value;
          }
        } else if (childAst.ruleName === "block") {
          block = childAst;
        }
      }
    }

    const newPrefix = `${prefix}-${subProp}`;
    const result: Array<ASTNode | Token> = [];
    if (block) {
      this._flattenNestedProps(block, newPrefix, result);
    }
    return result;
  }

  // ===========================================================================
  // Lattice v2: Built-in Function Evaluation
  // ===========================================================================

  /**
   * Evaluate a Lattice v2 built-in function call.
   *
   * Uses ExpressionEvaluator to resolve arguments, then calls the registered
   * built-in function handler. The result is converted back to an AST node.
   */
  private _evaluateBuiltinFunction(
    funcName: string,
    node: ASTNode,
    scope: ScopeChain
  ): ASTNode | Token {
    const children = getChildren(node);
    let args: LatticeValue[] = [];

    // Collect and evaluate arguments
    for (const child of children) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "function_args") {
        const evaluator = new ExpressionEvaluator(scope);
        args = this._collectBuiltinFunctionArgs(child as ASTNode, evaluator);
        break;
      }
    }

    const handler = BUILTIN_FUNCTIONS.get(funcName)!;
    const result = handler(args);

    if (result.kind === "null") {
      // Null result -- pass through as CSS function
      return this._expandChildren(node, scope);
    }

    const cssText = valueToCss(result);
    return makeValueNode(cssText, node);
  }

  /** Collect evaluated arguments from function_args for built-in functions. */
  private _collectBuiltinFunctionArgs(
    node: ASTNode,
    evaluator: ExpressionEvaluator
  ): LatticeValue[] {
    const args: LatticeValue[] = [];
    const currentTokens: Token[] = [];

    const flushTokens = () => {
      if (currentTokens.length > 0) {
        if (currentTokens.length === 1) {
          const t = currentTokens[0];
          if (tokenTypeName(t) === "VARIABLE") {
            const val = evaluator["scope"].get(t.value);
            if (val !== undefined && val !== null && typeof val === "object" && "kind" in val) {
              args.push(val as LatticeValue);
            } else if (val !== undefined && val !== null && typeof val === "object" && "ruleName" in val) {
              args.push(extractValueFromAst(val as ASTNode));
            } else {
              args.push(tokenToValue(t));
            }
          } else {
            args.push(tokenToValue(t));
          }
        } else {
          args.push(tokenToValue(currentTokens[0]));
        }
        currentTokens.length = 0;
      }
    };

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) {
        if (tokenTypeName(child as Token) === "COMMA") {
          flushTokens();
          continue;
        }
      }
      if (isASTNode(child) && (child as ASTNode).ruleName === "function_arg") {
        for (const ic of getChildren(child as ASTNode)) {
          if (!isASTNode(ic)) {
            if (tokenTypeName(ic as Token) === "COMMA") {
              flushTokens();
              continue;
            }
            currentTokens.push(ic as Token);
          } else {
            // AST node (expression) -- evaluate directly
            args.push(evaluator.evaluate(ic as ASTNode));
            currentTokens.length = 0;
          }
        }
      }
    }
    flushTokens();
    return args;
  }

  // ===========================================================================
  // Lattice v2: @extend Selector Merging (Pass 3)
  // ===========================================================================

  /**
   * Apply @extend selector merging to the entire AST.
   *
   * This is a post-processing pass. Currently implements the simplified
   * approach: remove placeholder-only rules from the output.
   */
  private _applyExtends(root: ASTNode): void {
    this._removePlaceholderRules(root);
  }

  /**
   * Remove rules whose selectors are exclusively placeholder selectors.
   *
   * Placeholder selectors (%name) exist only to be extended. Rules with
   * only placeholder selectors should not appear in the CSS output.
   */
  private _removePlaceholderRules(node: ASTNode | Token): void {
    if (!isASTNode(node)) return;

    const newChildren: Array<ASTNode | Token> = [];
    for (const child of getChildren(node as ASTNode)) {
      if (child === null) continue;
      if (this._isPlaceholderOnlyRule(child)) continue;
      this._removePlaceholderRules(child);
      newChildren.push(child);
    }
    setChildren(node as ASTNode, newChildren);
  }

  /** Check if a node is a qualified_rule with only placeholder selectors. */
  private _isPlaceholderOnlyRule(node: ASTNode | Token): boolean {
    if (!isASTNode(node)) return false;
    const ast = node as ASTNode;
    if (ast.ruleName === "qualified_rule") {
      const selectorText = this._extractSelectorText(ast);
      const selectors = selectorText.split(",").map(s => s.trim()).filter(s => s);
      return selectors.length > 0 && selectors.every(s => s.startsWith("%"));
    }
    if (ast.ruleName === "rule") {
      const children = getChildren(ast);
      if (children.length > 0 && isASTNode(children[0])) {
        return this._isPlaceholderOnlyRule(children[0]);
      }
    }
    return false;
  }

  /** Extract selector text from a qualified_rule. */
  private _extractSelectorText(node: ASTNode): string {
    for (const child of getChildren(node)) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "selector_list") {
        return this._collectText(child as ASTNode);
      }
    }
    return "";
  }

  /** Recursively collect all token text from an AST node. */
  private _collectText(node: ASTNode | Token): string {
    if (!isASTNode(node)) return (node as Token).value;
    const parts: string[] = [];
    for (const child of getChildren(node as ASTNode)) {
      parts.push(this._collectText(child));
    }
    return parts.join(" ");
  }

  // ===========================================================================
  // Lattice v2: @at-root Hoisting (Pass 3)
  // ===========================================================================

  /**
   * Splice @at-root hoisted rules into the root stylesheet.
   *
   * Rules collected during expansion via @at-root directives are
   * appended to the root stylesheet's children list.
   */
  private _spliceAtRootRules(root: ASTNode): void {
    for (const rule of this.atRootRules) {
      if (rule !== null) {
        getChildren(root).push(rule);
      }
    }
  }

  // ===========================================================================
  // Pass 3: Cleanup
  // ===========================================================================

  /**
   * Remove empty blocks and null children from the AST.
   *
   * After expansion, some nodes may be empty (e.g., a @for loop that
   * generated nothing, or a mixin body that was fully inlined). We
   * remove these empty shells so the emitter doesn't produce blank CSS.
   */
  private _cleanup(node: ASTNode | Token): ASTNode | Token | null {
    if (!isASTNode(node)) return node;

    const ast = node as ASTNode;
    const newChildren: Array<ASTNode | Token> = [];

    for (const child of getChildren(ast)) {
      if (child === null) continue;
      const cleaned = this._cleanup(child);
      if (cleaned !== null) {
        newChildren.push(cleaned);
      }
    }

    setChildren(ast, newChildren);
    return ast;
  }
}
