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
import { TypeErrorInExpression, LatticeRangeError, ZeroDivisionInExpressionError } from "./errors.js";

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
 * Stored as the raw string including the # prefix. Provides
 * conversion helpers for RGB and HSL color spaces, needed by
 * Lattice v2 built-in color functions (lighten, darken, mix, etc.).
 *
 * Hex parsing:
 *
 *   Format      Example     Alpha
 *   #RGB        #f00        1.0
 *   #RRGGBB     #ff0000     1.0
 *   #RRGGBBAA   #ff000080   ~0.5
 *
 * HSL conversion uses the standard algorithm from CSS Color Level 4.
 * Hue is in degrees (0-360), saturation and lightness are percentages (0-100).
 */
export class LatticeColor {
  readonly kind = "color" as const;

  constructor(readonly value: string) {}

  /**
   * Parse hex string to [r, g, b, a] where r/g/b are 0-255, a is 0-1.
   *
   * Handles #RGB (3-char shorthand), #RRGGBB (6-char), and
   * #RRGGBBAA (8-char with alpha) formats.
   */
  toRgb(): [number, number, number, number] {
    const h = this.value.replace(/^#/, "");
    if (h.length === 3) {
      const r = parseInt(h[0] + h[0], 16);
      const g = parseInt(h[1] + h[1], 16);
      const b = parseInt(h[2] + h[2], 16);
      return [r, g, b, 1.0];
    } else if (h.length === 6) {
      const r = parseInt(h.slice(0, 2), 16);
      const g = parseInt(h.slice(2, 4), 16);
      const b = parseInt(h.slice(4, 6), 16);
      return [r, g, b, 1.0];
    } else if (h.length === 8) {
      const r = parseInt(h.slice(0, 2), 16);
      const g = parseInt(h.slice(2, 4), 16);
      const b = parseInt(h.slice(4, 6), 16);
      const a = parseInt(h.slice(6, 8), 16) / 255.0;
      return [r, g, b, a];
    }
    return [0, 0, 0, 1.0];
  }

  /**
   * Convert to [h, s, l, a] where h is 0-360, s/l are 0-100, a is 0-1.
   *
   * Uses the standard RGB-to-HSL algorithm from CSS Color Level 4.
   */
  toHsl(): [number, number, number, number] {
    const [r, g, b, a] = this.toRgb();
    const rf = r / 255.0;
    const gf = g / 255.0;
    const bf = b / 255.0;
    const mx = Math.max(rf, gf, bf);
    const mn = Math.min(rf, gf, bf);
    const light = (mx + mn) / 2.0;

    if (mx === mn) {
      return [0.0, 0.0, light * 100.0, a];
    }

    const d = mx - mn;
    const sat = light > 0.5 ? d / (2.0 - mx - mn) : d / (mx + mn);

    let hue: number;
    if (mx === rf) {
      hue = (gf - bf) / d + (gf < bf ? 6.0 : 0.0);
    } else if (mx === gf) {
      hue = (bf - rf) / d + 2.0;
    } else {
      hue = (rf - gf) / d + 4.0;
    }
    hue *= 60.0;

    return [hue, sat * 100.0, light * 100.0, a];
  }

  /**
   * Create a LatticeColor from RGB(A) components.
   *
   * Clamps each channel to its valid range before encoding as hex.
   * If alpha is 1.0, emits #RRGGBB; otherwise emits rgba() notation.
   */
  static fromRgb(r: number, g: number, b: number, a: number = 1.0): LatticeColor {
    r = Math.max(0, Math.min(255, Math.round(r)));
    g = Math.max(0, Math.min(255, Math.round(g)));
    b = Math.max(0, Math.min(255, Math.round(b)));
    a = Math.max(0.0, Math.min(1.0, a));
    if (a >= 1.0) {
      const hex = `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
      return new LatticeColor(hex);
    }
    return new LatticeColor(`rgba(${r}, ${g}, ${b}, ${a})`);
  }

  /**
   * Create a LatticeColor from HSL(A) components.
   *
   * Uses the standard HSL-to-RGB algorithm.
   * h is in degrees (0-360), s/l are percentages (0-100).
   */
  static fromHsl(h: number, s: number, l: number, a: number = 1.0): LatticeColor {
    h = ((h % 360.0) + 360.0) % 360.0;
    s = Math.max(0.0, Math.min(100.0, s)) / 100.0;
    l = Math.max(0.0, Math.min(100.0, l)) / 100.0;

    if (s === 0.0) {
      const v = Math.round(l * 255);
      return LatticeColor.fromRgb(v, v, v, a);
    }

    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;

    function hueToRgb(pp: number, qq: number, t: number): number {
      if (t < 0) t += 1;
      if (t > 1) t -= 1;
      if (t < 1 / 6) return pp + (qq - pp) * 6 * t;
      if (t < 1 / 2) return qq;
      if (t < 2 / 3) return pp + (qq - pp) * (2 / 3 - t) * 6;
      return pp;
    }

    const hNorm = h / 360.0;
    const rr = Math.round(hueToRgb(p, q, hNorm + 1 / 3) * 255);
    const gg = Math.round(hueToRgb(p, q, hNorm) * 255);
    const bb = Math.round(hueToRgb(p, q, hNorm - 1 / 3) * 255);

    return LatticeColor.fromRgb(rr, gg, bb, a);
  }

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

/**
 * An ordered key-value map -- Lattice v2 value type.
 *
 * Maps are written as parenthesized key-value pairs:
 *
 *     $theme: (
 *         primary: #4a90d9,
 *         secondary: #7b68ee,
 *         background: #ffffff,
 *     );
 *
 * Internally, a map is stored as an array of [key, value] tuples to
 * maintain insertion order.
 *
 * Lookup semantics:
 *   - Keys are strings (identifiers are treated as strings for lookup).
 *   - Duplicate keys: the last value wins (no error).
 *   - Maps are always truthy, even when empty.
 *   - Maps cannot be directly used as CSS values.
 *
 * Access is exclusively through built-in functions:
 *   - map-get($map, $key)
 *   - map-keys($map)
 *   - map-values($map)
 *   - map-has-key($map, $key)
 *   - map-merge($map1, $map2)
 *   - map-remove($map, $keys...)
 */
export class LatticeMap {
  readonly kind = "map" as const;

  constructor(readonly items: ReadonlyArray<readonly [string, LatticeValue]>) {}

  /** Look up a value by key. Returns undefined if not found. */
  get(key: string): LatticeValue | undefined {
    for (const [k, v] of this.items) {
      if (k === key) return v;
    }
    return undefined;
  }

  /** Return all keys in insertion order. */
  keys(): string[] {
    return this.items.map(([k]) => k);
  }

  /** Return all values in insertion order. */
  values(): LatticeValue[] {
    return this.items.map(([, v]) => v);
  }

  /** Check if a key exists in the map. */
  hasKey(key: string): boolean {
    return this.items.some(([k]) => k === key);
  }

  toString(): string {
    const entries = this.items.map(([k, v]) => `${k}: ${v}`).join(", ");
    return `(${entries})`;
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
  | LatticeList
  | LatticeMap;

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

// =============================================================================
// Lattice v2: Type Introspection Helpers
// =============================================================================

/**
 * Return the Lattice type name for a value.
 *
 * Maps internal types to user-facing type strings used by type-of().
 */
export function typeNameOf(value: LatticeValue): string {
  switch (value.kind) {
    case "number": case "dimension": case "percentage": return "number";
    case "string": case "ident": return "string";
    case "color": return "color";
    case "bool": return "bool";
    case "null": return "null";
    case "list": return "list";
    case "map": return "map";
    default: return "unknown";
  }
}

/**
 * Extract the numeric value from a number-like LatticeValue.
 *
 * Throws TypeErrorInExpression if the value is not numeric.
 */
export function getNumericValue(v: LatticeValue): number {
  if (v.kind === "number") return v.value;
  if (v.kind === "dimension") return v.value;
  if (v.kind === "percentage") return v.value;
  throw new TypeErrorInExpression("use", `Expected a number, got ${typeNameOf(v)}`, "");
}

// =============================================================================
// Lattice v2: Built-in Function Registry
// =============================================================================
//
// Built-in functions are registered in a Map keyed by function name.
// Each function takes an array of LatticeValue arguments and a ScopeChain
// reference (for context), and returns a LatticeValue.
//
// Functions are organized by category:
//   1. Map functions: map-get, map-keys, map-values, map-has-key, map-merge, map-remove
//   2. Color functions: lighten, darken, saturate, desaturate, adjust-hue,
//      complement, mix, rgba, red, green, blue, hue, saturation, lightness
//   3. List functions: nth, length, join, append, index
//   4. Type functions: type-of, unit, unitless, comparable
//   5. Math functions: math.div, math.floor, math.ceil, math.round,
//      math.abs, math.min, math.max
// =============================================================================

/** Type for built-in function handlers. */
type BuiltinFn = (args: LatticeValue[]) => LatticeValue;

function ensureColor(v: LatticeValue): LatticeColor {
  if (v.kind !== "color") {
    throw new TypeErrorInExpression("use", `Expected a color, got ${typeNameOf(v)}`, "");
  }
  return v;
}

function ensureAmount(v: LatticeValue): number {
  const val = getNumericValue(v);
  if (val < 0 || val > 100) {
    throw new LatticeRangeError("Amount must be between 0% and 100%");
  }
  return val;
}

function ensureMap(v: LatticeValue): LatticeMap {
  if (v.kind !== "map") {
    throw new TypeErrorInExpression("use", `Expected a map, got ${typeNameOf(v)}`, "");
  }
  return v;
}

// -- Map functions --

const builtinMapGet: BuiltinFn = (args) => {
  if (args.length < 2) throw new TypeErrorInExpression("call", "map-get requires 2 arguments", "");
  const m = ensureMap(args[0]);
  const key = args[1].toString().replace(/^"|"$/g, "");
  const result = m.get(key);
  return result ?? new LatticeNull();
};

const builtinMapKeys: BuiltinFn = (args) => {
  if (!args.length) throw new TypeErrorInExpression("call", "map-keys requires 1 argument", "");
  const m = ensureMap(args[0]);
  return new LatticeList(m.keys().map(k => new LatticeIdent(k)));
};

const builtinMapValues: BuiltinFn = (args) => {
  if (!args.length) throw new TypeErrorInExpression("call", "map-values requires 1 argument", "");
  const m = ensureMap(args[0]);
  return new LatticeList(m.values());
};

const builtinMapHasKey: BuiltinFn = (args) => {
  if (args.length < 2) throw new TypeErrorInExpression("call", "map-has-key requires 2 arguments", "");
  const m = ensureMap(args[0]);
  const key = args[1].toString().replace(/^"|"$/g, "");
  return new LatticeBool(m.hasKey(key));
};

const builtinMapMerge: BuiltinFn = (args) => {
  if (args.length < 2) throw new TypeErrorInExpression("call", "map-merge requires 2 arguments", "");
  const m1 = ensureMap(args[0]);
  const m2 = ensureMap(args[1]);
  const merged = new Map<string, LatticeValue>();
  for (const [k, v] of m1.items) merged.set(k, v);
  for (const [k, v] of m2.items) merged.set(k, v);
  return new LatticeMap(Array.from(merged.entries()));
};

const builtinMapRemove: BuiltinFn = (args) => {
  if (!args.length) throw new TypeErrorInExpression("call", "map-remove requires at least 1 argument", "");
  const m = ensureMap(args[0]);
  const keysToRemove = new Set(args.slice(1).map(a => a.toString().replace(/^"|"$/g, "")));
  return new LatticeMap(m.items.filter(([k]) => !keysToRemove.has(k)));
};

// -- Color functions --

const builtinLighten: BuiltinFn = (args) => {
  const color = ensureColor(args[0]);
  const amount = ensureAmount(args[1]);
  const [h, s, l, a] = color.toHsl();
  return LatticeColor.fromHsl(h, s, Math.min(100.0, l + amount), a);
};

const builtinDarken: BuiltinFn = (args) => {
  const color = ensureColor(args[0]);
  const amount = ensureAmount(args[1]);
  const [h, s, l, a] = color.toHsl();
  return LatticeColor.fromHsl(h, s, Math.max(0.0, l - amount), a);
};

const builtinSaturate: BuiltinFn = (args) => {
  const color = ensureColor(args[0]);
  const amount = ensureAmount(args[1]);
  const [h, s, l, a] = color.toHsl();
  return LatticeColor.fromHsl(h, Math.min(100.0, s + amount), l, a);
};

const builtinDesaturate: BuiltinFn = (args) => {
  const color = ensureColor(args[0]);
  const amount = ensureAmount(args[1]);
  const [h, s, l, a] = color.toHsl();
  return LatticeColor.fromHsl(h, Math.max(0.0, s - amount), l, a);
};

const builtinAdjustHue: BuiltinFn = (args) => {
  const color = ensureColor(args[0]);
  const degrees = getNumericValue(args[1]);
  const [h, s, l, a] = color.toHsl();
  return LatticeColor.fromHsl((h + degrees) % 360.0, s, l, a);
};

const builtinComplement: BuiltinFn = (args) => {
  const color = ensureColor(args[0]);
  const [h, s, l, a] = color.toHsl();
  return LatticeColor.fromHsl((h + 180.0) % 360.0, s, l, a);
};

const builtinMix: BuiltinFn = (args) => {
  const c1 = ensureColor(args[0]);
  const c2 = ensureColor(args[1]);
  const weight = args.length >= 3 ? getNumericValue(args[2]) : 50.0;
  const w = weight / 100.0;
  const [r1, g1, b1, a1] = c1.toRgb();
  const [r2, g2, b2, a2] = c2.toRgb();
  return LatticeColor.fromRgb(
    Math.round(r1 * w + r2 * (1 - w)),
    Math.round(g1 * w + g2 * (1 - w)),
    Math.round(b1 * w + b2 * (1 - w)),
    a1 * w + a2 * (1 - w)
  );
};

const builtinRgba: BuiltinFn = (args) => {
  if (args.length === 2 && args[0].kind === "color") {
    const color = args[0] as LatticeColor;
    const alpha = getNumericValue(args[1]);
    const [r, g, b] = color.toRgb();
    return LatticeColor.fromRgb(r, g, b, alpha);
  }
  if (args.length === 4) {
    return LatticeColor.fromRgb(
      Math.round(getNumericValue(args[0])),
      Math.round(getNumericValue(args[1])),
      Math.round(getNumericValue(args[2])),
      getNumericValue(args[3])
    );
  }
  return new LatticeNull();
};

const builtinRed: BuiltinFn = (args) => {
  const [r] = ensureColor(args[0]).toRgb();
  return new LatticeNumber(r);
};

const builtinGreen: BuiltinFn = (args) => {
  const [, g] = ensureColor(args[0]).toRgb();
  return new LatticeNumber(g);
};

const builtinBlue: BuiltinFn = (args) => {
  const [, , b] = ensureColor(args[0]).toRgb();
  return new LatticeNumber(b);
};

const builtinHue: BuiltinFn = (args) => {
  const [h] = ensureColor(args[0]).toHsl();
  return new LatticeDimension(Math.round(h), "deg");
};

const builtinSaturation: BuiltinFn = (args) => {
  const [, s] = ensureColor(args[0]).toHsl();
  return new LatticePercentage(Math.round(s));
};

const builtinLightness: BuiltinFn = (args) => {
  const [, , l] = ensureColor(args[0]).toHsl();
  return new LatticePercentage(Math.round(l));
};

// -- List functions --

const builtinNth: BuiltinFn = (args) => {
  if (args.length < 2) throw new TypeErrorInExpression("call", "nth requires 2 arguments", "");
  const lst = args[0];
  const n = Math.trunc(getNumericValue(args[1]));
  if (n < 1) throw new LatticeRangeError("List index must be 1 or greater");
  if (lst.kind === "list") {
    if (n > lst.items.length) throw new LatticeRangeError(`Index ${n} out of bounds for list of length ${lst.items.length}`);
    return lst.items[n - 1];
  }
  if (n === 1) return lst;
  throw new LatticeRangeError(`Index ${n} out of bounds for list of length 1`);
};

const builtinLength: BuiltinFn = (args) => {
  if (!args.length) throw new TypeErrorInExpression("call", "length requires 1 argument", "");
  const v = args[0];
  if (v.kind === "list") return new LatticeNumber(v.items.length);
  if (v.kind === "map") return new LatticeNumber(v.items.length);
  return new LatticeNumber(1);
};

const builtinJoin: BuiltinFn = (args) => {
  if (args.length < 2) throw new TypeErrorInExpression("call", "join requires at least 2 arguments", "");
  const items1 = args[0].kind === "list" ? args[0].items : [args[0]];
  const items2 = args[1].kind === "list" ? args[1].items : [args[1]];
  return new LatticeList([...items1, ...items2]);
};

const builtinAppend: BuiltinFn = (args) => {
  if (args.length < 2) throw new TypeErrorInExpression("call", "append requires at least 2 arguments", "");
  const items = args[0].kind === "list" ? [...args[0].items] : [args[0]];
  items.push(args[1]);
  return new LatticeList(items);
};

const builtinIndex: BuiltinFn = (args) => {
  if (args.length < 2) throw new TypeErrorInExpression("call", "index requires 2 arguments", "");
  const items = args[0].kind === "list" ? args[0].items : [args[0]];
  const targetStr = args[1].toString();
  for (let i = 0; i < items.length; i++) {
    if (items[i].toString() === targetStr) return new LatticeNumber(i + 1);
  }
  return new LatticeNull();
};

// -- Type functions --

const builtinTypeOf: BuiltinFn = (args) => {
  if (!args.length) throw new TypeErrorInExpression("call", "type-of requires 1 argument", "");
  return new LatticeString(typeNameOf(args[0]));
};

const builtinUnit: BuiltinFn = (args) => {
  if (!args.length) throw new TypeErrorInExpression("call", "unit requires 1 argument", "");
  const v = args[0];
  if (v.kind === "dimension") return new LatticeString(v.unit);
  if (v.kind === "percentage") return new LatticeString("%");
  if (v.kind === "number") return new LatticeString("");
  throw new TypeErrorInExpression("use", `Expected a number, got ${typeNameOf(v)}`, "");
};

const builtinUnitless: BuiltinFn = (args) => {
  if (!args.length) throw new TypeErrorInExpression("call", "unitless requires 1 argument", "");
  return new LatticeBool(args[0].kind === "number");
};

const builtinComparable: BuiltinFn = (args) => {
  if (args.length < 2) throw new TypeErrorInExpression("call", "comparable requires 2 arguments", "");
  const a = args[0], b = args[1];
  if (a.kind === b.kind) {
    if (a.kind === "dimension" && b.kind === "dimension") return new LatticeBool(a.unit === b.unit);
    return new LatticeBool(true);
  }
  const isNumKind = (k: string) => k === "number" || k === "dimension" || k === "percentage";
  if (isNumKind(a.kind) && isNumKind(b.kind)) {
    if (a.kind === "number" || b.kind === "number") return new LatticeBool(true);
    return new LatticeBool(false);
  }
  return new LatticeBool(false);
};

// -- Math functions --

const builtinMathDiv: BuiltinFn = (args) => {
  if (args.length < 2) throw new TypeErrorInExpression("call", "math.div requires 2 arguments", "");
  const a = args[0], b = args[1];
  const bVal = getNumericValue(b);
  if (bVal === 0) throw new ZeroDivisionInExpressionError();
  const aVal = getNumericValue(a);
  if (a.kind === "dimension" && b.kind === "number") return new LatticeDimension(aVal / bVal, a.unit);
  if (a.kind === "dimension" && b.kind === "dimension" && a.unit === b.unit) return new LatticeNumber(aVal / bVal);
  if (a.kind === "percentage" && b.kind === "number") return new LatticePercentage(aVal / bVal);
  return new LatticeNumber(aVal / bVal);
};

function mathUnaryFn(mathFn: (n: number) => number): BuiltinFn {
  return (args) => {
    if (!args.length) throw new TypeErrorInExpression("call", "math function requires 1 argument", "");
    const v = args[0];
    const val = getNumericValue(v);
    const result = mathFn(val);
    if (v.kind === "dimension") return new LatticeDimension(result, v.unit);
    if (v.kind === "percentage") return new LatticePercentage(result);
    return new LatticeNumber(result);
  };
}

const builtinMathFloor = mathUnaryFn(Math.floor);
const builtinMathCeil = mathUnaryFn(Math.ceil);
const builtinMathRound = mathUnaryFn(Math.round);
const builtinMathAbs = mathUnaryFn(Math.abs);

const builtinMathMin: BuiltinFn = (args) => {
  if (!args.length) throw new TypeErrorInExpression("call", "math.min requires at least 1 argument", "");
  let best = args[0];
  let bestVal = getNumericValue(best);
  for (let i = 1; i < args.length; i++) {
    const val = getNumericValue(args[i]);
    if (val < bestVal) { best = args[i]; bestVal = val; }
  }
  return best;
};

const builtinMathMax: BuiltinFn = (args) => {
  if (!args.length) throw new TypeErrorInExpression("call", "math.max requires at least 1 argument", "");
  let best = args[0];
  let bestVal = getNumericValue(best);
  for (let i = 1; i < args.length; i++) {
    const val = getNumericValue(args[i]);
    if (val > bestVal) { best = args[i]; bestVal = val; }
  }
  return best;
};

// -- Registry --

/**
 * All built-in Lattice v2 functions, keyed by function name.
 *
 * The transformer checks this registry before looking up user-defined functions.
 * User-defined functions shadow built-ins with the same name (matching Sass behavior).
 */
export const BUILTIN_FUNCTIONS: Map<string, BuiltinFn> = new Map([
  // Map functions
  ["map-get", builtinMapGet],
  ["map-keys", builtinMapKeys],
  ["map-values", builtinMapValues],
  ["map-has-key", builtinMapHasKey],
  ["map-merge", builtinMapMerge],
  ["map-remove", builtinMapRemove],
  // Color functions
  ["lighten", builtinLighten],
  ["darken", builtinDarken],
  ["saturate", builtinSaturate],
  ["desaturate", builtinDesaturate],
  ["adjust-hue", builtinAdjustHue],
  ["complement", builtinComplement],
  ["mix", builtinMix],
  ["rgba", builtinRgba],
  ["red", builtinRed],
  ["green", builtinGreen],
  ["blue", builtinBlue],
  ["hue", builtinHue],
  ["saturation", builtinSaturation],
  ["lightness", builtinLightness],
  // List functions
  ["nth", builtinNth],
  ["length", builtinLength],
  ["join", builtinJoin],
  ["append", builtinAppend],
  ["index", builtinIndex],
  // Type functions
  ["type-of", builtinTypeOf],
  ["unit", builtinUnit],
  ["unitless", builtinUnitless],
  ["comparable", builtinComparable],
  // Math functions
  ["math.div", builtinMathDiv],
  ["math.floor", builtinMathFloor],
  ["math.ceil", builtinMathCeil],
  ["math.round", builtinMathRound],
  ["math.abs", builtinMathAbs],
  ["math.min", builtinMathMin],
  ["math.max", builtinMathMax],
]);
