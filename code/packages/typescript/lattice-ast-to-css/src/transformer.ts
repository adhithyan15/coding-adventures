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
  isTruthy,
  tokenToValue,
  valueToCss,
  extractValueFromAst,
} from "./values.js";
import {
  UndefinedVariableError,
  UndefinedMixinError,
  CircularReferenceError,
  WrongArityError,
  MissingReturnError,
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

  /**
   * Transform a Lattice AST into a clean CSS AST.
   *
   * Runs the three-pass pipeline:
   * 1. Collect symbols (variables, mixins, functions)
   * 2. Expand all Lattice constructs
   * 3. Clean up empty nodes
   *
   * @param ast - The root "stylesheet" ASTNode from the parser.
   * @returns A clean CSS AST with no Lattice nodes.
   */
  transform(ast: ASTNode): ASTNode {
    // Pass 1: Collect symbols
    this._collectSymbols(ast);

    // Pass 2: Expand
    const result = this._expandNode(ast, this.variables);

    // Pass 3: Cleanup
    return this._cleanup(result as ASTNode) as ASTNode;
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

  /** Extract variable name and value from a variable_declaration node. */
  private _collectVariable(node: ASTNode): void {
    let name: string | undefined;
    let valueNode: ASTNode | undefined;

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) {
        if (tokenTypeName(child as Token) === "VARIABLE") {
          name = (child as Token).value;
        }
      } else if ((child as ASTNode).ruleName === "value_list") {
        valueNode = child as ASTNode;
      }
    }

    if (name && valueNode) {
      this.variables.set(name, valueNode);
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
   * lattice_block_item = variable_declaration | include_directive | lattice_control ;
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
    }

    return this._expandChildren(node, scope);
  }

  /**
   * Process a variable_declaration inside a block.
   *
   * Sets the variable in the current scope. The node is removed from output.
   */
  private _expandVariableDeclaration(node: ASTNode, scope: ScopeChain): void {
    let name: string | undefined;
    let valueNode: ASTNode | undefined;

    for (const child of getChildren(node)) {
      if (!isASTNode(child)) {
        if (tokenTypeName(child as Token) === "VARIABLE") {
          name = (child as Token).value;
        }
      } else if ((child as ASTNode).ruleName === "value_list") {
        valueNode = child as ASTNode;
      }
    }

    if (name && valueNode) {
      // Expand the value first (it might contain variables)
      const expandedValue = this._expandNode(deepClone(valueNode), scope);
      scope.set(name, expandedValue ?? valueNode);
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

    // Lattice function — evaluate (user functions shadow CSS built-ins)
    if (this.functions.has(funcName)) {
      return this._evaluateFunctionCall(funcName, node, scope);
    }

    // CSS built-in — expand args but keep structure
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
   */
  private _expandInclude(
    node: ASTNode,
    scope: ScopeChain
  ): Array<ASTNode | Token> {
    const children = getChildren(node);
    let mixinName: string | undefined;
    let argsNode: ASTNode | undefined;

    for (const child of children) {
      if (!isASTNode(child)) {
        const token = child as Token;
        const type = tokenTypeName(token);
        if (type === "FUNCTION") {
          mixinName = token.value.replace(/\($/, "");
        } else if (type === "IDENT") {
          mixinName = token.value;
        }
      } else if ((child as ASTNode).ruleName === "include_args") {
        argsNode = child as ASTNode;
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

    // Parse arguments
    const args = argsNode ? this._parseIncludeArgs(argsNode) : [];

    // Check arity
    const required = mixinDef.params.length - mixinDef.defaults.size;
    if (args.length < required || args.length > mixinDef.params.length) {
      throw new WrongArityError(
        "Mixin",
        mixinName,
        mixinDef.params.length,
        args.length
      );
    }

    // Create child scope with params bound
    const mixinScope = scope.child();
    for (let i = 0; i < mixinDef.params.length; i++) {
      const paramName = mixinDef.params[i];
      if (i < args.length) {
        mixinScope.set(paramName, args[i]);
      } else if (mixinDef.defaults.has(paramName)) {
        mixinScope.set(paramName, deepClone(mixinDef.defaults.get(paramName)! as ASTNode));
      }
    }

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
    }
  }

  /**
   * Parse include_args into a list of value nodes.
   *
   * include_args = value_list { COMMA value_list } ;
   *
   * Due to grammar design, commas may be absorbed into a single value_list.
   * So if we get only one value_list but it contains COMMA values,
   * we split it on commas to produce multiple args.
   */
  private _parseIncludeArgs(node: ASTNode): Array<ASTNode | Token> {
    const valueLists: ASTNode[] = [];

    for (const child of getChildren(node)) {
      if (isASTNode(child) && (child as ASTNode).ruleName === "value_list") {
        valueLists.push(child as ASTNode);
      }
    }

    // If there's only one value_list, check if it contains commas and split
    if (valueLists.length === 1) {
      return this._splitValueListOnCommas(valueLists[0]);
    }

    return valueLists;
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
   * lattice_control = if_directive | for_directive | each_directive ;
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
