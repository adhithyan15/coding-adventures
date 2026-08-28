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

const rem = (value: number): Length => ({ unit: "rem", value });
const percent = (value: number): Length => ({ unit: "%", value });
const named = (name: string): Color => ({ kind: "named", name });
const ref = (path: string) => ({ kind: "token-ref" as const, path });

function rule(
  id: string,
  selector: StyleRule["selector"],
  properties: readonly StyleProperty[],
): StyleRule {
  return { id: styleRuleId(id), selector, properties };
}

const empty = emptyStyleDocument();

/**
 * Portable document styling goes through Style IR. The companion landing.css
 * owns browser-specific layout primitives (grid, flex, pseudo-elements, and
 * breakpoints) that the current backend-neutral IR intentionally cannot model.
 */
export const landingStyle: StyleDocument = {
  ...empty,
  tokens: {
    ...empty.tokens,
    colors: {
      text: named("#15231f"),
      muted: named("#485852"),
      surface: named("#f3efe5"),
      accent: named("#d83c19"),
    },
    typography: {
      families: {
        body: ["Inter", "ui-sans-serif", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "sans-serif"],
        display: ["Iowan Old Style", "Palatino Linotype", "Palatino", "Georgia", "serif"],
      },
      scale: { body: rem(1), h1: rem(6), h2: rem(3), h3: rem(1.5) },
      weights: { regular: 400, strong: 800 },
      leading: { body: 1.6, tight: 1 },
      tracking: {},
    },
    space: {},
    radii: {},
    shadows: {},
  },
  rules: [
    rule("landing-body", sel.type("body"), [
      { kind: "font-family", value: ref("typography.families.body") },
      { kind: "font-size", value: ref("typography.scale.body") },
      { kind: "leading", value: ref("typography.leading.body") },
      { kind: "color", value: ref("colors.text") },
      { kind: "background", value: ref("colors.surface") },
    ]),
    rule("landing-display", sel.or(sel.heading(1), sel.heading(2)), [
      { kind: "font-family", value: ref("typography.families.display") },
      { kind: "leading", value: ref("typography.leading.tight") },
    ]),
    rule("landing-h1", sel.heading(1), [
      { kind: "font-size", value: ref("typography.scale.h1") },
      { kind: "font-weight", value: ref("typography.weights.regular") },
    ]),
    rule("landing-h2", sel.heading(2), [
      { kind: "font-size", value: ref("typography.scale.h2") },
      { kind: "font-weight", value: ref("typography.weights.regular") },
    ]),
    rule("landing-h3", sel.heading(3), [
      { kind: "font-size", value: ref("typography.scale.h3") },
      { kind: "font-weight", value: ref("typography.weights.strong") },
    ]),
    rule("landing-link", sel.type("a"), [
      { kind: "color", value: ref("colors.text") },
      { kind: "text-decoration", value: { line: "none" } },
    ]),
    rule("landing-image", sel.type("img"), [
      { kind: "max-width", value: percent(100) },
    ]),
  ],
};

export default landingStyle;
