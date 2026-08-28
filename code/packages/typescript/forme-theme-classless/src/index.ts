/** A reusable, dependency-free classless Forme theme with light/dark modes. */

import {
  emptyStyleDocument,
  sel,
  styleRuleId,
  type Color,
  type Length,
  type StyleDocument,
  type StyleProperty,
  type StyleRule,
} from "@coding-adventures/forme-style-ir";

const px = (value: number): Length => ({ unit: "px", value });
const rem = (value: number): Length => ({ unit: "rem", value });
const em = (value: number): Length => ({ unit: "em", value });
const percent = (value: number): Length => ({ unit: "%", value });
const named = (name: string): Color => ({ kind: "named", name });
const ref = (path: string) => ({ kind: "token-ref" as const, path });
const box = <T>(vertical: T, horizontal: T) => ({
  top: vertical, right: horizontal, bottom: vertical, left: horizontal,
});

function rule(
  id: string,
  selector: StyleRule["selector"],
  properties: readonly StyleProperty[],
  context?: string,
): StyleRule {
  const base: StyleRule = { id: styleRuleId(id), selector, properties };
  return context === undefined ? base : { ...base, context };
}

const empty = emptyStyleDocument();

/**
 * The default Coding Adventures prose theme. It is a plain StyleDocument—not
 * renderer state—so callers can compose or replace it before rendering.
 */
export const classlessTheme: StyleDocument = {
  ...empty,
  tokens: {
    ...empty.tokens,
    colors: {
      text: named("#1f2937"),
      muted: named("#64748b"),
      surface: named("#ffffff"),
      softSurface: named("#f1f5f9"),
      accent: named("#2563eb"),
      border: named("#cbd5e1"),
    },
    typography: {
      families: {
        body: ["ui-sans-serif", "system-ui", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "sans-serif"],
        mono: ["ui-monospace", "SFMono-Regular", "Menlo", "Consolas", "monospace"],
      },
      scale: {
        small: rem(0.9),
        body: rem(1),
        h1: rem(2.25),
        h2: rem(1.55),
        h3: rem(1.25),
      },
      weights: { regular: 400, strong: 650 },
      leading: { tight: 1.2, body: 1.65, code: 1.5 },
      tracking: {},
    },
    space: { xs: rem(0.25), sm: rem(0.5), md: rem(1), lg: rem(2), xl: rem(4) },
    radii: { sm: px(5), md: px(9) },
    shadows: {},
  },
  contexts: ["dark", "narrow", "high-contrast"],
  rules: [
    rule("shell-html", sel.type("html"), [
      { kind: "align", value: "center" },
    ]),
    rule("shell-body", sel.type("body"), [
      { kind: "display", value: "inline-block" },
      { kind: "max-width", value: { unit: "ch", value: 72 } },
      { kind: "align", value: "start" },
      { kind: "font-family", value: ref("typography.families.body") },
      { kind: "font-size", value: ref("typography.scale.body") },
      { kind: "leading", value: ref("typography.leading.body") },
      { kind: "color", value: ref("colors.text") },
      { kind: "background", value: ref("colors.surface") },
      { kind: "padding", value: { top: rem(2), right: rem(1.25), bottom: rem(4), left: rem(1.25) } },
    ]),
    rule("shell-main", sel.type("main"), [
      { kind: "max-width", value: { unit: "ch", value: 72 } },
    ]),
    rule("shell-header", sel.type("header"), [
      { kind: "max-width", value: { unit: "ch", value: 72 } },
      { kind: "space-after", value: ref("space.lg") },
      { kind: "color", value: ref("colors.muted") },
      { kind: "font-size", value: ref("typography.scale.small") },
    ]),
    rule("headings", sel.or(sel.heading(1), sel.heading(2), sel.heading(3), sel.heading(4), sel.heading(5), sel.heading(6)), [
      { kind: "leading", value: ref("typography.leading.tight") },
      { kind: "font-weight", value: ref("typography.weights.strong") },
      { kind: "space-before", value: ref("space.lg") },
      { kind: "space-after", value: ref("space.md") },
    ]),
    rule("heading-h1", sel.heading(1), [
      { kind: "font-size", value: ref("typography.scale.h1") },
      { kind: "space-before", value: px(0) },
    ]),
    rule("heading-h2", sel.heading(2), [
      { kind: "font-size", value: ref("typography.scale.h2") },
      { kind: "border", value: { width: px(1), style: "solid", color: ref("colors.border"), sides: ["bottom"] } },
      { kind: "padding", value: { top: px(0), right: px(0), bottom: rem(0.35), left: px(0) } },
    ]),
    rule("heading-h3", sel.heading(3), [{ kind: "font-size", value: ref("typography.scale.h3") }]),
    rule("paragraph", sel.type("p"), [{ kind: "space-after", value: ref("space.md") }]),
    rule("link", sel.type("a"), [
      { kind: "color", value: ref("colors.accent") },
      { kind: "text-decoration", value: { line: "underline", thickness: px(1) } },
    ]),
    rule("strong", sel.type("strong"), [{ kind: "font-weight", value: ref("typography.weights.strong") }]),
    rule("emphasis", sel.type("em"), [{ kind: "font-style", value: "italic" }]),
    rule("inline-code", sel.type("code"), [
      { kind: "font-family", value: ref("typography.families.mono") },
      { kind: "font-size", value: em(0.9) },
      { kind: "padding", value: box(em(0.15), em(0.35)) },
      { kind: "background", value: ref("colors.softSurface") },
      { kind: "border-radius", value: ref("radii.sm") },
    ]),
    rule("code-block", sel.type("pre"), [
      { kind: "font-size", value: ref("typography.scale.small") },
      { kind: "leading", value: ref("typography.leading.code") },
      { kind: "padding", value: box(ref("space.md"), ref("space.md")) },
      { kind: "background", value: ref("colors.softSurface") },
      { kind: "border-radius", value: ref("radii.md") },
    ]),
    rule("code-block-inner", sel.descendantOf(sel.type("pre"), sel.type("code")), [
      { kind: "padding", value: box(px(0), px(0)) },
      { kind: "background", value: named("transparent") },
      { kind: "border-radius", value: px(0) },
    ]),
    rule("blockquote", sel.type("blockquote"), [
      { kind: "color", value: ref("colors.muted") },
      { kind: "padding", value: { top: rem(0.25), right: rem(1), bottom: rem(0.25), left: rem(1) } },
      { kind: "border", value: { width: px(4), style: "solid", color: ref("colors.border"), sides: ["left"] } },
    ]),
    rule("lists", sel.or(sel.type("ul"), sel.type("ol")), [
      { kind: "padding", value: { top: px(0), right: px(0), bottom: px(0), left: rem(1.5) } },
      { kind: "space-after", value: ref("space.md") },
    ]),
    rule("list-item", sel.type("li"), [{ kind: "space-after", value: ref("space.xs") }]),
    rule("index-item", sel.descendantOf(sel.id("post-index"), sel.type("li")), [
      { kind: "padding", value: { top: rem(1), right: px(0), bottom: rem(1), left: px(0) } },
      { kind: "border", value: { width: px(1), style: "solid", color: ref("colors.border"), sides: ["bottom"] } },
    ]),
    rule("index-link", sel.descendantOf(sel.id("post-index"), sel.type("a")), [
      { kind: "font-size", value: rem(1.1) },
      { kind: "font-weight", value: ref("typography.weights.strong") },
    ]),
    rule("index-time", sel.descendantOf(sel.id("post-index"), sel.type("time")), [
      { kind: "display", value: "block" },
      { kind: "font-size", value: ref("typography.scale.small") },
      { kind: "color", value: ref("colors.muted") },
    ]),
    rule("index-summary", sel.descendantOf(sel.id("post-index"), sel.type("p")), [
      { kind: "color", value: ref("colors.muted") },
      { kind: "space-before", value: ref("space.sm") },
    ]),
    rule("rule", sel.type("hr"), [
      { kind: "border", value: { width: px(1), style: "solid", color: ref("colors.border"), sides: ["top"] } },
      { kind: "space-before", value: ref("space.lg") },
      { kind: "space-after", value: ref("space.lg") },
    ]),
    rule("image", sel.type("img"), [{ kind: "max-width", value: percent(100) }]),
    rule("table-cells", sel.or(sel.type("th"), sel.type("td")), [
      { kind: "border", value: { width: px(1), style: "solid", color: ref("colors.border") } },
      { kind: "padding", value: box(rem(0.45), rem(0.7)) },
      { kind: "align", value: "start" },
    ]),
    rule("table-heading", sel.type("th"), [
      { kind: "background", value: ref("colors.softSurface") },
      { kind: "font-weight", value: ref("typography.weights.strong") },
    ]),
    rule("dark-shell", sel.or(sel.type("body"), sel.type("main")), [
      { kind: "color", value: named("#e5e7eb") },
      { kind: "background", value: named("#111827") },
    ], "dark"),
    rule("dark-muted", sel.or(
      sel.type("header"),
      sel.type("blockquote"),
      sel.descendantOf(sel.id("post-index"), sel.type("time")),
      sel.descendantOf(sel.id("post-index"), sel.type("p")),
    ), [
      { kind: "color", value: named("#a8b3c5") },
    ], "dark"),
    rule("dark-link", sel.type("a"), [{ kind: "color", value: named("#7dd3fc") }], "dark"),
    rule("dark-soft-surface", sel.or(sel.type("code"), sel.type("pre"), sel.type("th")), [
      { kind: "background", value: named("#1f2937") },
    ], "dark"),
    rule("dark-borders", sel.or(sel.heading(2), sel.type("blockquote"), sel.type("hr"), sel.type("th"), sel.type("td"), sel.descendantOf(sel.id("post-index"), sel.type("li"))), [
      { kind: "border-color", value: named("#475569") },
    ], "dark"),
    rule("narrow-shell", sel.type("body"), [
      { kind: "padding", value: { top: rem(1.25), right: rem(0.85), bottom: rem(3), left: rem(0.85) } },
    ], "narrow"),
    rule("contrast-link", sel.type("a"), [
      { kind: "text-decoration", value: { line: "underline", thickness: px(2) } },
    ], "high-contrast"),
  ],
};

export default classlessTheme;
