/**
 * tokens.ts — design tokens (FM04 §3).
 *
 * Tokens are the design-system primitives — colors, type scales,
 * spacing scales, shadow definitions — that rules reference *by
 * name*.  The naming layer is what lets themes re-bind the same
 * name to a different concrete value without rewriting any rule.
 *
 * Rough analogy: in CSS, an author might write `color: #1f2328`
 * directly.  In Style IR, they declare a token `colors.text` ↦
 * `#1f2328` and reference it as `{ kind: "token-ref", path:
 * "colors.text" }`.  A "high-contrast" theme then overrides
 * `colors.text` ↦ `#000`, and every rule referencing the token
 * picks up the override coherently — no per-rule edit, no
 * search-and-replace.
 *
 * Four color representations (`rgb`, `hsl`, `oklch`, `named`),
 * eleven length units, and one shadow shape cover the visual
 * primitives every document-shaped output (HTML, LaTeX, PDF,
 * terminal, EPUB, email) needs.  Backends convert between
 * representations as their output requires — `oklch` may become
 * gamut-mapped `rgb` for a PDF, ANSI 24-bit for a terminal, etc.
 *
 * @module tokens
 */

import type { JsonValue, ReadonlyRecord } from "@coding-adventures/forme-types";

// ─── Color ────────────────────────────────────────────────────────────────

/**
 * A color value.  Four variants cover the spectrum from "universal,
 * any backend can produce it" (rgb) through "designer-intuitive"
 * (hsl) to "perceptually uniform / modern" (oklch) plus
 * "passthrough for backend-specific names" (named).
 *
 * Channel ranges (per FM04 §3.2):
 * - `rgb`:   r, g, b ∈ [0, 255]; a ∈ [0, 1] (default 1)
 * - `hsl`:   h ∈ [0, 360); s, l ∈ [0, 100]; a ∈ [0, 1]
 * - `oklch`: l ∈ [0, 1]; c ∈ [0, ~0.5]; h ∈ [0, 360); a ∈ [0, 1]
 * - `named`: any string the backend recognises ("red", "transparent",
 *            LaTeX xcolor names, ANSI names, …)
 */
export type Color =
  | { readonly kind: "rgb";   readonly r: number; readonly g: number; readonly b: number; readonly a?: number }
  | { readonly kind: "hsl";   readonly h: number; readonly s: number; readonly l: number; readonly a?: number }
  | { readonly kind: "oklch"; readonly l: number; readonly c: number; readonly h: number; readonly a?: number }
  | { readonly kind: "named"; readonly name: string };

// ─── Length ───────────────────────────────────────────────────────────────

/**
 * A typed length.  Print backends prefer absolute units (`pt`, `mm`,
 * `in`); web backends prefer relative ones (`rem`, `em`, `%`).  Both
 * are first-class — the IR never collapses to a single unit.  The
 * translator picks what its output natively expresses; out-of-domain
 * units (asking a terminal renderer about `rem`) degrade per the
 * unknown-value rule in the translator.
 */
export type Length =
  | { readonly unit: "px"; readonly value: number }
  | { readonly unit: "rem"; readonly value: number }
  | { readonly unit: "em"; readonly value: number }
  | { readonly unit: "%"; readonly value: number }
  | { readonly unit: "vh"; readonly value: number }
  | { readonly unit: "vw"; readonly value: number }
  | { readonly unit: "pt"; readonly value: number }
  | { readonly unit: "mm"; readonly value: number }
  | { readonly unit: "in"; readonly value: number }
  | { readonly unit: "ch"; readonly value: number }
  | { readonly unit: "ex"; readonly value: number };

/** Frozen list of allowed length units — used by the validator. */
export const LENGTH_UNITS = Object.freeze([
  "px", "rem", "em", "%", "vh", "vw", "pt", "mm", "in", "ch", "ex",
] as const);

export type LengthUnit = (typeof LENGTH_UNITS)[number];

// ─── Shadow ───────────────────────────────────────────────────────────────

/**
 * A drop shadow.  Print backends (PDF / LaTeX) typically drop
 * shadows entirely or substitute a thin outline; the translator
 * decides.
 */
export interface Shadow {
  readonly offsetX: Length;
  readonly offsetY: Length;
  readonly blur: Length;
  readonly spread: Length;
  readonly color: Color | TokenRef;
  readonly inset?: boolean;
}

// ─── Token references ─────────────────────────────────────────────────────

/**
 * A reference to a token by *dotted path* into the `TokenSet` tree.
 *
 * Examples:
 *   `{ kind: "token-ref", path: "colors.primary" }`
 *   `{ kind: "token-ref", path: "typography.scale.lg" }`
 *   `{ kind: "token-ref", path: "space.md" }`
 *
 * Rules use `TokenRef` instead of inlining concrete values whenever
 * the value should be theme-swappable.  Properties may still carry
 * concrete values directly when theme-swap doesn't apply (e.g. a
 * one-off marker color baked into the design).
 *
 * The validator checks shape only — the *resolution* (lookup against
 * a `TokenSet`) is the translator's responsibility, because that's
 * the layer that knows whether a theme has been composed in yet.
 */
export interface TokenRef {
  readonly kind: "token-ref";
  /** Dotted path into the token tree.  See module docstring. */
  readonly path: string;
}

/** Type guard for `TokenRef`. */
export function isTokenRef(value: unknown): value is TokenRef {
  return (
    typeof value === "object"
    && value !== null
    && (value as { kind?: unknown }).kind === "token-ref"
    && typeof (value as { path?: unknown }).path === "string"
  );
}

// ─── Typography ───────────────────────────────────────────────────────────

/** A comma-separated font fallback chain, e.g. `["Inter", "system-ui", "sans-serif"]`. */
export type FontStack = readonly string[];

/**
 * The typography token group.  Each sub-record is keyed by *token
 * name*, not by some preordained scale step name — designers pick the
 * names ("xs", "sm", "md", … or "compact", "comfortable", "spacious",
 * …) and rules reference whatever names the token set declares.
 */
export interface TypographyTokens {
  /** Named font stacks.  Example: `{ body: ["Inter", "sans-serif"], mono: ["SF Mono", "monospace"] }`. */
  readonly families: ReadonlyRecord<string, FontStack>;
  /** Type-size scale.  Indexed by name. */
  readonly scale: ReadonlyRecord<string, Length>;
  /** Numeric weight values.  Example: `{ regular: 400, bold: 700 }`. */
  readonly weights: ReadonlyRecord<string, number>;
  /** Line-height multipliers (unitless).  Example: `{ tight: 1.2, normal: 1.5, loose: 1.8 }`. */
  readonly leading: ReadonlyRecord<string, number>;
  /** Letter-spacing values. */
  readonly tracking: ReadonlyRecord<string, Length>;
}

// ─── TokenSet ─────────────────────────────────────────────────────────────

/**
 * The full design-token bundle a `StyleDocument` carries.  Five
 * named buckets cover the visual vocabulary of the typical document:
 * colors, typography, spacing, corner radii, shadows.  Anything
 * domain-specific lives in `extensions` under the kernel-blessed
 * namespace pattern `ext:<plugin>:<group>` (FM01 §2.5).
 *
 * Colors can themselves be `TokenRef`s — that's how a "primary" color
 * cascades from "the user-chosen brand color" through "darken-on-
 * hover" derivations without baking the literal in.
 */
export interface TokenSet {
  /** Named colors.  Values may be literals OR refs into the same set. */
  readonly colors: ReadonlyRecord<string, Color | TokenRef>;
  /** Typography stack. */
  readonly typography: TypographyTokens;
  /** Spacing scale.  Reordering the scale is a theme concern. */
  readonly space: ReadonlyRecord<string, Length>;
  /** Corner-radius scale. */
  readonly radii: ReadonlyRecord<string, Length>;
  /** Drop-shadow definitions. */
  readonly shadows: ReadonlyRecord<string, Shadow>;
  /**
   * Plugin-contributed token groups.  Keys must follow `ext:<package>:<group>`.
   * Values are opaque `JsonValue` — only the contributing plugin's translator
   * understands their shape.
   */
  readonly extensions?: ReadonlyRecord<string, JsonValue>;
}

/**
 * Construct an empty `TokenSet` with all five required buckets
 * present-but-empty.  Convenient for theme bases that override
 * sparsely.
 */
export function emptyTokenSet(): TokenSet {
  return {
    colors: {},
    typography: {
      families: {},
      scale: {},
      weights: {},
      leading: {},
      tracking: {},
    },
    space: {},
    radii: {},
    shadows: {},
  };
}
