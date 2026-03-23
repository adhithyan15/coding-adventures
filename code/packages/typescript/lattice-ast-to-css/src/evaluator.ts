/**
 * Expression Evaluator — Compile-time evaluation of Lattice expressions.
 *
 * Lattice expressions appear in three contexts:
 *
 * 1. @if conditions:  @if $theme == dark { ... }
 * 2. @for bounds:    @for $i from 1 through $count { ... }
 * 3. @return values: @return $n * 8px;
 *
 * The evaluator walks lattice_expression AST nodes and computes their values
 * at compile time. This is similar to constant folding in a compiler, but
 * Lattice evaluates ALL expressions at compile time (there is no runtime).
 *
 * Operator Precedence
 * -------------------
 *
 * From tightest to loosest binding (matching the grammar):
 *
 *   1. Unary minus:      -$x
 *   2. Multiplication:   $a * $b
 *   3. Addition/subtraction: $a + $b, $a - $b
 *   4. Comparison:       ==, !=, >, >=, <=
 *   5. Logical AND:      $a and $b
 *   6. Logical OR:       $a or $b
 *
 * The grammar encodes this precedence via nested rules (lattice_or_expr →
 * lattice_and_expr → lattice_comparison → lattice_additive → lattice_multiplicative
 * → lattice_unary → lattice_primary), so the evaluator just recursively
 * evaluates the AST — no precedence climbing needed.
 */

import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";
import type { ScopeChain } from "./scope.js";
import {
  LatticeValue,
  LatticeNull,
  LatticeIdent,
  isTruthy,
  tokenToValue,
  addValues,
  subtractValues,
  multiplyValues,
  negateValue,
  compareValues,
  extractValueFromAst,
} from "./values.js";

// =============================================================================
// Type Guards
// =============================================================================

/** Check if a child is an ASTNode (not a Token). */
function isASTNode(child: ASTNode | Token): child is ASTNode {
  return "ruleName" in child;
}

/** Get the token type as a string. */
function tokenTypeName(token: Token): string {
  return token.type as string;
}

// =============================================================================
// ExpressionEvaluator
// =============================================================================

/**
 * Evaluates Lattice expression AST nodes at compile time.
 *
 * The evaluator walks the AST produced by the grammar parser's expression
 * rules and computes a LatticeValue result.
 *
 * The grammar's nesting of rules already encodes operator precedence, so
 * the evaluator just recursively evaluates each node without needing its
 * own precedence table.
 *
 * Usage:
 *
 *     const evaluator = new ExpressionEvaluator(scope);
 *     const result = evaluator.evaluate(expressionNode);
 *     // result is a LatticeValue like LatticeNumber(42)
 */
export class ExpressionEvaluator {
  constructor(private readonly scope: ScopeChain) {}

  /**
   * Evaluate an expression AST node.
   *
   * Dispatches on ruleName to the appropriate handler. If the node is a
   * token (leaf), converts it directly to a value.
   *
   * @param node - An ASTNode or Token from the parser.
   * @returns The evaluated LatticeValue.
   */
  evaluate(node: ASTNode | Token): LatticeValue {
    // If it's a raw token (not an ASTNode), convert directly.
    if (!isASTNode(node)) {
      return tokenToValue(node as Token);
    }

    const rule = (node as ASTNode).ruleName;

    // Dispatch to handler based on rule name.
    switch (rule) {
      case "lattice_expression":
        return this._evalLatticeExpression(node as ASTNode);
      case "lattice_or_expr":
        return this._evalOrExpr(node as ASTNode);
      case "lattice_and_expr":
        return this._evalAndExpr(node as ASTNode);
      case "lattice_comparison":
        return this._evalComparison(node as ASTNode);
      case "lattice_additive":
        return this._evalAdditive(node as ASTNode);
      case "lattice_multiplicative":
        return this._evalMultiplicative(node as ASTNode);
      case "lattice_unary":
        return this._evalUnary(node as ASTNode);
      case "lattice_primary":
        return this._evalPrimary(node as ASTNode);
      case "comparison_op":
        return tokenToValue((node as ASTNode).children[0] as Token);
    }

    // For wrapper rules with a single child, unwrap.
    const children = (node as ASTNode).children;
    if (children.length === 1) {
      return this.evaluate(children[0]);
    }

    // Default: try to evaluate the first meaningful child.
    for (const child of children) {
      if (isASTNode(child) || (child as Token).type) {
        return this.evaluate(child);
      }
    }

    return new LatticeNull();
  }

  // ---------------------------------------------------------------------------
  // Rule handlers
  // ---------------------------------------------------------------------------

  /** lattice_expression = lattice_or_expr ; */
  private _evalLatticeExpression(node: ASTNode): LatticeValue {
    return this.evaluate(node.children[0]);
  }

  /**
   * lattice_or_expr = lattice_and_expr { "or" lattice_and_expr } ;
   *
   * Short-circuit evaluation: return first truthy operand, or the last one.
   * Mirrors JavaScript's || operator semantics.
   */
  private _evalOrExpr(node: ASTNode): LatticeValue {
    const children = node.children;
    let result = this.evaluate(children[0]);

    let i = 1;
    while (i < children.length) {
      const child = children[i];
      // Skip the "or" IDENT token
      if (!isASTNode(child) && (child as Token).value === "or") {
        i++;
        continue;
      }
      if (isTruthy(result)) {
        return result;
      }
      result = this.evaluate(child);
      i++;
    }
    return result;
  }

  /**
   * lattice_and_expr = lattice_comparison { "and" lattice_comparison } ;
   *
   * Short-circuit evaluation: return first falsy operand, or the last one.
   * Mirrors JavaScript's && operator semantics.
   */
  private _evalAndExpr(node: ASTNode): LatticeValue {
    const children = node.children;
    let result = this.evaluate(children[0]);

    let i = 1;
    while (i < children.length) {
      const child = children[i];
      // Skip the "and" IDENT token
      if (!isASTNode(child) && (child as Token).value === "and") {
        i++;
        continue;
      }
      if (!isTruthy(result)) {
        return result;
      }
      result = this.evaluate(child);
      i++;
    }
    return result;
  }

  /**
   * lattice_comparison = lattice_additive [ comparison_op lattice_additive ] ;
   *
   * If only one child (no comparison_op), return the additive value.
   * If three children, perform the comparison.
   */
  private _evalComparison(node: ASTNode): LatticeValue {
    const children = node.children;
    const left = this.evaluate(children[0]);

    if (children.length === 1) {
      return left;
    }

    // Find the comparison_op node and right operand.
    let opNode: ASTNode | null = null;
    let rightNode: ASTNode | Token | null = null;

    for (let i = 1; i < children.length; i++) {
      const child = children[i];
      if (isASTNode(child) && (child as ASTNode).ruleName === "comparison_op") {
        opNode = child as ASTNode;
      } else if (opNode !== null) {
        rightNode = child;
        break;
      }
    }

    if (opNode === null || rightNode === null) {
      return left;
    }

    const right = this.evaluate(rightNode);
    // The comparison_op child is the operator token itself
    const opToken = opNode.children[0] as Token;
    const opType = tokenTypeName(opToken);

    return compareValues(left, right, opType);
  }

  /**
   * lattice_additive = lattice_multiplicative
   *                    { ( PLUS | MINUS ) lattice_multiplicative } ;
   */
  private _evalAdditive(node: ASTNode): LatticeValue {
    const children = node.children;
    let result = this.evaluate(children[0]);

    let i = 1;
    while (i < children.length) {
      const child = children[i];
      if (!isASTNode(child)) {
        const token = child as Token;
        const op = token.value;
        if (op === "+" || op === "-") {
          i++;
          if (i < children.length) {
            const right = this.evaluate(children[i]);
            if (op === "+") {
              result = addValues(result, right);
            } else {
              result = subtractValues(result, right);
            }
          }
        }
      }
      i++;
    }
    return result;
  }

  /**
   * lattice_multiplicative = lattice_unary { STAR lattice_unary } ;
   */
  private _evalMultiplicative(node: ASTNode): LatticeValue {
    const children = node.children;
    let result = this.evaluate(children[0]);

    let i = 1;
    while (i < children.length) {
      const child = children[i];
      if (!isASTNode(child) && (child as Token).value === "*") {
        i++;
        if (i < children.length) {
          const right = this.evaluate(children[i]);
          result = multiplyValues(result, right);
        }
      }
      i++;
    }
    return result;
  }

  /**
   * lattice_unary = MINUS lattice_unary | lattice_primary ;
   */
  private _evalUnary(node: ASTNode): LatticeValue {
    const children = node.children;

    // Check if first child is a MINUS token
    if (
      children.length >= 2 &&
      !isASTNode(children[0]) &&
      (children[0] as Token).value === "-"
    ) {
      const operand = this.evaluate(children[1]);
      return negateValue(operand);
    }

    return this.evaluate(children[0]);
  }

  /**
   * lattice_primary = VARIABLE | NUMBER | DIMENSION | PERCENTAGE
   *                 | STRING | IDENT | HASH
   *                 | "true" | "false" | "null"
   *                 | function_call
   *                 | LPAREN lattice_expression RPAREN ;
   */
  private _evalPrimary(node: ASTNode): LatticeValue {
    const children = node.children;

    for (const child of children) {
      if (!isASTNode(child)) {
        // It's a token
        const token = child as Token;
        const typeName = tokenTypeName(token);

        // Skip parentheses tokens
        if (typeName === "LPAREN" || typeName === "RPAREN") {
          continue;
        }

        if (typeName === "VARIABLE") {
          // Look up the variable in scope
          const result = this.scope.get(token.value);
          if (result === undefined) {
            // Return the ident for now; transformer handles undefined errors
            return new LatticeIdent(token.value);
          }
          // If already a LatticeValue, return it
          if (result !== null && typeof result === "object" && "kind" in result) {
            return result as LatticeValue;
          }
          // If it's an AST node (e.g., value_list), extract its first value
          if (result !== null && typeof result === "object" && "ruleName" in result) {
            return extractValueFromAst(result as ASTNode);
          }
          // If it's a raw token, convert it
          if (result !== null && typeof result === "object" && "type" in result) {
            return tokenToValue(result as Token);
          }
          return new LatticeNull();
        }

        return tokenToValue(token);
      }

      // It's an ASTNode — recurse (handles function_call and parenthesized expressions)
      return this.evaluate(child);
    }

    return new LatticeNull();
  }
}
