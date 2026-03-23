/**
 * Lattice Value Types — Runtime values for the compile-time evaluator.
 *
 * Lattice expressions are evaluated at compile time (there is no runtime —
 * the output is static CSS). This module defines the TypeScript types that
 * represent those compile-time values.
 *
 * Why Not Just Use Strings?
 * --------------------------
 *
 * CSS values are text, so why not just use strings? Two reasons:
 *
 * 1. **Type safety for arithmetic**: 10px + 5px is valid (same unit);
 *    10px + 5s is invalid (different units). We need structured types
 *    to enforce this, not just strings.
 *
 * 2. **Truthiness for control flow**: @if conditions need to know if a
 *    value is "truthy" or "falsy". Numbers have numeric truthiness;
 *    booleans have boolean truthiness; null is always falsy. String
 *    comparison can't capture these semantics.
 *
 * Value Type Table
 * -----------------
 *
 *   TypeScript Class       CSS/Lattice                Examples
 *   --------------------   -----------------------    -------------------
 *   LatticeNumber          CSS NUMBER token           42, 3.14, -1
 *   LatticeDimension       CSS DIMENSION token        16px, 2em, 1.5rem
 *   LatticePercentage      CSS PERCENTAGE token       50%, 100%, 33.33%
 *   LatticeString          CSS STRING token           "hello", 'world'
 *   LatticeIdent           CSS IDENT token            red, bold, dark
 *   LatticeColor           CSS HASH token             #4a90d9, #fff
 *   LatticeBool            Lattice boolean literal    true, false
 *   LatticeNull            Lattice null literal       null
 *   LatticeList            Comma-separated list       red, green, blue
 *
 * Discriminated Unions
 * ---------------------
 *
 * The LatticeValue type is a TypeScript discriminated union — each class
 * has a readonly `kind` property that TypeScript's type narrowing can use
 * to distinguish between them in switch statements and if chains.
 *
 *     function process(v: LatticeValue): string {
 *       switch (v.kind) {
 *         case "number": return v.value.toString();
 *         case "dimension": return `${v.value}${v.unit}`;
 *         // ...
 *       }
 *     }
 */

import type { Token } from "@coding-adventures/lexer";
import type { ASTNode } from "@coding-adventures/parser";
import { TypeErrorInExpression } from "./errors.js";

// =============================================================================
// Value Classes
// =============================================================================

/**
 * A pure number without units. Maps to CSS NUMBER token.
 *
 * Examples: 42, 3.14, 0, -1
 *
 * Arithmetic: Number op Number → Number
 */
export class LatticeNumber {
  readonly kind = "number" as const;

  constructor(readonly value: number) {}

  toString(): string {
    // Emit integers without decimal point: 42 not 42.0
    if (this.value === Math.trunc(this.value) && isFinite(this.value)) {
      return String(Math.trunc(this.value));
    }
    return String(this.value);
  }
}

/**
 * A number with a CSS unit. Maps to CSS DIMENSION token.
 *
 * Examples: 16px, 2em, 1.5rem, 100vh, 300ms
 *
 * The unit is a string like "px", "em", "rem", etc. Arithmetic is only
 * valid between dimensions with the same unit (10px + 5px = 15px).
 * Cross-unit arithmetic (10px + 2em) raises TypeErrorInExpression.
 */
export class LatticeDimension {
  readonly kind = "dimension" as const;

  constructor(readonly value: number, readonly unit: string) {}

  toString(): string {
    if (this.value === Math.trunc(this.value) && isFinite(this.value)) {
      return `${Math.trunc(this.value)}${this.unit}`;
    }
    return `${this.value}${this.unit}`;
  }
}

/**
 * A percentage value. Maps to CSS PERCENTAGE token.
 *
 * Examples: 50%, 100%, 33.33%
 *
 * Percentages are a special case — they look like dimensions with unit "%"
 * but CSS treats them differently in many contexts (e.g., width: 50% is
 * relative to the parent, but font-size: 50% is relative to the inherited
 * font-size).
 */
export class LatticePercentage {
  readonly kind = "percentage" as const;

  constructor(readonly value: number) {}

  toString(): string {
    if (this.value === Math.trunc(this.value) && isFinite(this.value)) {
      return `${Math.trunc(this.value)}%`;
    }
    return `${this.value}%`;
  }
}

/**
 * A quoted string value. Maps to CSS STRING token.
 *
 * Examples: "hello", 'world'
 *
 * The quotes are not stored — they're added back during CSS emission.
 * The lexer strips quotes when tokenizing (escapeMode: none in lattice.tokens).
 *
 * String concatenation is supported: "hello" + "world" → "helloworld".
 */
export class LatticeString {
  readonly kind = "string" as const;

  constructor(readonly value: string) {}

  toString(): string {
    return `"${this.value}"`;
  }
}

/**
 * An unquoted identifier. Maps to CSS IDENT token.
 *
 * Examples: red, bold, dark, sans-serif, transparent
 *
 * CSS color keywords (red, blue, etc.) are idents, not a special type.
 * The evaluator treats them as opaque identifiers — no color arithmetic.
 * This matches Sass behavior: you can compare idents but not do math on them.
 */
export class LatticeIdent {
  readonly kind = "ident" as const;

  constructor(readonly value: string) {}

  toString(): string {
    return this.value;
  }
}

/**
 * A hex color value. Maps to CSS HASH token in color context.
 *
 * Examples: #4a90d9, #fff, #00000080
 *
 * Stored as the raw string including the # prefix.
 * No color arithmetic is supported — colors are opaque values.
 */
export class LatticeColor {
  readonly kind = "color" as const;

  constructor(readonly value: string) {}

  toString(): string {
    return this.value;
  }
}

/**
 * A boolean value — true or false.
 *
 * Lattice boolean literals are idents that the grammar matches via
 * literal text: "true" and "false".
 *
 * Truthiness: false is falsy, true is truthy.
 * Used in @if conditions: @if $theme == dark { ... }
 */
export class LatticeBool {
  readonly kind = "bool" as const;

  constructor(readonly value: boolean) {}

  toString(): string {
    return this.value ? "true" : "false";
  }
}

/**
 * The null value.
 *
 * null is falsy and stringifies to empty string (like Sass).
 * Used for optional parameters and missing values.
 *
 * Example: @mixin m($color: null) { @if $color { color: $color; } }
 */
export class LatticeNull {
  readonly kind = "null" as const;

  toString(): string {
    return "";
  }
}

/**
 * A comma-separated list of values.
 *
 * Used in @each directives and multi-value declarations.
 * Each item is a LatticeValue.
 *
 * Example: @each $color in red, green, blue { ... }
 * Here, (red, green, blue) is a LatticeList of LatticeIdents.
 */
export class LatticeList {
  readonly kind = "list" as const;

  constructor(readonly items: readonly LatticeValue[]) {}

  toString(): string {
    return this.items.map((i) => i.toString()).join(", ");
  }
}

// =============================================================================
// Discriminated Union
// =============================================================================

/**
 * The union of all Lattice value types.
 *
 * TypeScript uses the `kind` property to narrow the type in
 * switch/if chains. For example:
 *
 *     if (value.kind === "number") {
 *       // TypeScript knows value is LatticeNumber here
 *       console.log(value.value); // number
 *     }
 */
export type LatticeValue =
  | LatticeNumber
  | LatticeDimension
  | LatticePercentage
  | LatticeString
  | LatticeIdent
  | LatticeColor
  | LatticeBool
  | LatticeNull
  | LatticeList;

// =============================================================================
// Truthiness
// =============================================================================

/**
 * Determine whether a Lattice value is truthy.
 *
 * Truthiness rules (matching Sass conventions):
 *
 *   false  → falsy
 *   null   → falsy
 *   0      → falsy (LatticeNumber with value 0)
 *   everything else → truthy (including empty strings and empty lists)
 *
 * Note: Unlike JavaScript, empty strings are truthy in Lattice.
 * This matches Sass behavior.
 *
 * @param value - The value to test.
 * @returns true if the value is truthy, false otherwise.
 */
export function isTruthy(value: LatticeValue): boolean {
  if (value.kind === "bool") return value.value;
  if (value.kind === "null") return false;
  if (value.kind === "number" && value.value === 0) return false;
  return true;
}

// =============================================================================
// Token → Value Conversion
// =============================================================================

/**
 * Convert a parser Token to a LatticeValue.
 *
 * Maps token types to value types:
 *
 *   NUMBER     → LatticeNumber
 *   DIMENSION  → LatticeDimension
 *   PERCENTAGE → LatticePercentage
 *   STRING     → LatticeString
 *   IDENT      → LatticeIdent (or LatticeBool/LatticeNull for literals)
 *   HASH       → LatticeColor
 *
 * @param token - A Token from the parser.
 * @returns The corresponding LatticeValue.
 */
export function tokenToValue(token: Token): LatticeValue {
  const { type: tokenType, value } = token;

  if (tokenType === "NUMBER") {
    return new LatticeNumber(parseFloat(value));
  }

  if (tokenType === "DIMENSION") {
    // Split "16px" into number (16) and unit (px).
    // Find where the numeric part ends and the unit begins.
    // The numeric part is: optional leading minus, digits, optional dot, more digits.
    const match = value.match(/^(-?[0-9]*\.?[0-9]+(?:[eE][+-]?[0-9]+)?)([a-zA-Z]+)$/);
    if (match) {
      return new LatticeDimension(parseFloat(match[1]), match[2]);
    }
    // Fallback: try to parse the number up to the first letter
    let i = 0;
    if (value[i] === "-") i++;
    while (i < value.length && (value[i] === "." || (value[i] >= "0" && value[i] <= "9"))) i++;
    const num = parseFloat(value.slice(0, i));
    const unit = value.slice(i);
    return new LatticeDimension(num, unit);
  }

  if (tokenType === "PERCENTAGE") {
    // "50%" → LatticePercentage(50)
    return new LatticePercentage(parseFloat(value.replace("%", "")));
  }

  if (tokenType === "STRING") {
    return new LatticeString(value);
  }

  if (tokenType === "HASH") {
    return new LatticeColor(value);
  }

  if (tokenType === "IDENT") {
    if (value === "true") return new LatticeBool(true);
    if (value === "false") return new LatticeBool(false);
    if (value === "null") return new LatticeNull();
    return new LatticeIdent(value);
  }

  // Fallback for unexpected token types — treat as ident.
  return new LatticeIdent(String(value));
}

/**
 * Extract the first LatticeValue from an AST node.
 *
 * When a variable is bound to a value_list node (from the parser),
 * we need to extract the actual value. A value_list like "dark"
 * contains a single value node wrapping an IDENT token.
 *
 * For multi-token value_lists, we take the first token's value.
 *
 * @param node - An ASTNode from the parser.
 * @returns The extracted LatticeValue.
 */
export function extractValueFromAst(node: ASTNode | Token): LatticeValue {
  if (!("ruleName" in node)) {
    // It's a token — convert directly
    return tokenToValue(node as Token);
  }

  const astNode = node as ASTNode;
  for (const child of astNode.children) {
    if (!("ruleName" in child)) {
      // It's a token — convert it
      return tokenToValue(child as Token);
    } else {
      // Recurse into child nodes
      const result = extractValueFromAst(child as ASTNode);
      if (result.kind !== "null") return result;
    }
  }
  return new LatticeNull();
}

/**
 * Convert a LatticeValue to its CSS text representation.
 *
 * This is used when substituting evaluated values back into CSS output.
 * Each value type's toString() already produces valid CSS text.
 *
 * @param value - The value to convert.
 * @returns CSS text representation of the value.
 */
export function valueToCss(value: LatticeValue): string {
  return value.toString();
}

// =============================================================================
// Arithmetic Helpers
// =============================================================================

/**
 * Add two LatticeValues.
 *
 * Rules:
 *   Number + Number → Number
 *   Dimension + Dimension (same unit) → Dimension
 *   Percentage + Percentage → Percentage
 *   String + String → String (concatenation)
 *   anything else → TypeErrorInExpression
 */
export function addValues(left: LatticeValue, right: LatticeValue): LatticeValue {
  if (left.kind === "number" && right.kind === "number") {
    return new LatticeNumber(left.value + right.value);
  }
  if (left.kind === "dimension" && right.kind === "dimension") {
    if (left.unit === right.unit) {
      return new LatticeDimension(left.value + right.value, left.unit);
    }
    throw new TypeErrorInExpression("add", left.toString(), right.toString());
  }
  if (left.kind === "percentage" && right.kind === "percentage") {
    return new LatticePercentage(left.value + right.value);
  }
  if (left.kind === "string" && right.kind === "string") {
    return new LatticeString(left.value + right.value);
  }
  throw new TypeErrorInExpression("add", left.toString(), right.toString());
}

/**
 * Subtract two LatticeValues.
 *
 * Rules mirror addition, but subtracts.
 */
export function subtractValues(left: LatticeValue, right: LatticeValue): LatticeValue {
  if (left.kind === "number" && right.kind === "number") {
    return new LatticeNumber(left.value - right.value);
  }
  if (left.kind === "dimension" && right.kind === "dimension") {
    if (left.unit === right.unit) {
      return new LatticeDimension(left.value - right.value, left.unit);
    }
    throw new TypeErrorInExpression("subtract", left.toString(), right.toString());
  }
  if (left.kind === "percentage" && right.kind === "percentage") {
    return new LatticePercentage(left.value - right.value);
  }
  throw new TypeErrorInExpression("subtract", left.toString(), right.toString());
}

/**
 * Multiply two LatticeValues.
 *
 * Rules:
 *   Number × Number → Number
 *   Number × Dimension → Dimension (scales the value)
 *   Dimension × Number → Dimension (commutative)
 *   Number × Percentage → Percentage
 *   Percentage × Number → Percentage
 *   anything else → TypeErrorInExpression
 */
export function multiplyValues(left: LatticeValue, right: LatticeValue): LatticeValue {
  if (left.kind === "number" && right.kind === "number") {
    return new LatticeNumber(left.value * right.value);
  }
  if (left.kind === "number" && right.kind === "dimension") {
    return new LatticeDimension(left.value * right.value, right.unit);
  }
  if (left.kind === "dimension" && right.kind === "number") {
    return new LatticeDimension(left.value * right.value, left.unit);
  }
  if (left.kind === "number" && right.kind === "percentage") {
    return new LatticePercentage(left.value * right.value);
  }
  if (left.kind === "percentage" && right.kind === "number") {
    return new LatticePercentage(left.value * right.value);
  }
  throw new TypeErrorInExpression("multiply", left.toString(), right.toString());
}

/**
 * Negate a LatticeValue.
 *
 * Only numeric values (Number, Dimension, Percentage) can be negated.
 */
export function negateValue(value: LatticeValue): LatticeValue {
  if (value.kind === "number") return new LatticeNumber(-value.value);
  if (value.kind === "dimension") return new LatticeDimension(-value.value, value.unit);
  if (value.kind === "percentage") return new LatticePercentage(-value.value);
  throw new TypeErrorInExpression("negate", value.toString(), "");
}

/**
 * Compare two LatticeValues.
 *
 * Supports ==, !=, >, >=, <= comparisons.
 * Returns a LatticeBool.
 *
 * For numeric types (same type + unit), does numeric comparison.
 * For everything else, falls back to string equality.
 *
 * @param left - Left operand.
 * @param right - Right operand.
 * @param op - Operator token type string: "EQUALS_EQUALS", "NOT_EQUALS",
 *             "GREATER", "GREATER_EQUALS", "LESS_EQUALS".
 * @returns LatticeBool result.
 */
export function compareValues(
  left: LatticeValue,
  right: LatticeValue,
  op: string
): LatticeBool {
  // Numeric comparison for same-type values
  const isNumeric = (v: LatticeValue): v is LatticeNumber | LatticeDimension | LatticePercentage =>
    v.kind === "number" || v.kind === "dimension" || v.kind === "percentage";

  if (isNumeric(left) && left.kind === right.kind) {
    const lv = left.value;
    const rv = (right as LatticeNumber | LatticeDimension | LatticePercentage).value;

    // For dimensions, units must match for ordering comparisons
    if (
      left.kind === "dimension" &&
      right.kind === "dimension" &&
      left.unit !== right.unit &&
      op !== "EQUALS_EQUALS" &&
      op !== "NOT_EQUALS"
    ) {
      return new LatticeBool(false);
    }

    switch (op) {
      case "EQUALS_EQUALS":
        if (left.kind === "dimension" && right.kind === "dimension") {
          return new LatticeBool(lv === rv && left.unit === (right as LatticeDimension).unit);
        }
        return new LatticeBool(lv === rv);
      case "NOT_EQUALS":
        if (left.kind === "dimension" && right.kind === "dimension") {
          return new LatticeBool(lv !== rv || left.unit !== (right as LatticeDimension).unit);
        }
        return new LatticeBool(lv !== rv);
      case "GREATER":
        return new LatticeBool(lv > rv);
      case "GREATER_EQUALS":
        return new LatticeBool(lv >= rv);
      case "LESS_EQUALS":
        return new LatticeBool(lv <= rv);
    }
  }

  // Equality comparison via string representation for mixed/non-numeric types
  const leftStr = left.toString();
  const rightStr = right.toString();
  if (op === "EQUALS_EQUALS") return new LatticeBool(leftStr === rightStr);
  if (op === "NOT_EQUALS") return new LatticeBool(leftStr !== rightStr);

  // Can't order non-numeric types
  return new LatticeBool(false);
}
