# @coding-adventures/forme-style-to-css

The reference **Style IR → CSS translator**. Takes a `StyleDocument`
from [`@coding-adventures/forme-style-ir`](../forme-style-ir) and
emits a CSS string plus the per-rule metadata the AOT compiler (FM06)
uses for per-page slicing.

Implements [FM04 §9.2](../../specs/FM04-forme-style-ir.md). Second
package of the FM04 family; the IR substrate is in `forme-style-ir`.

## Quick start

```ts
import { translateToCss } from "@coding-adventures/forme-style-to-css";
import {
  emptyStyleDocument, styleRuleId, sel,
} from "@coding-adventures/forme-style-ir";

const doc = {
  ...emptyStyleDocument(),
  tokens: {
    ...emptyStyleDocument().tokens,
    colors: { text: { kind: "rgb", r: 31, g: 35, b: 40 } },
  },
  rules: [
    {
      id: styleRuleId("body-text"),
      selector: sel.type("paragraph"),
      properties: [
        { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
        { kind: "leading", value: 1.6 },
      ],
    },
  ],
};

const { output, emittedRules, warnings } = translateToCss(doc, {
  activeContexts: ["screen"],
});

// output:
//   paragraph {
//     color: rgb(31 35 40);
//     line-height: 1.6;
//   }
```

## Options

```ts
interface TranslateOptions {
  activeContexts: readonly string[];     // which named contexts are on
  usedRuleIds?: readonly StyleRuleId[];  // per-page CSS slicing input
  scope?: string;                        // CSS prefix applied to every selector
}
```

- **`activeContexts`** — rules with a `context` field apply only when
  their context is in this list. Rules with NO context always apply.
  Empty list ⇒ no contexts active.
- **`usedRuleIds`** — when set, ONLY rules with these ids emit. This
  is the FM06 per-page CSS slicing input — the renderer accumulates
  `usedStyle` ids per page; the AOT compiler passes them here.
- **`scope`** — optional CSS prefix applied to every selector
  (`scope=".page-abc123"` produces `.page-abc123 p`, `.page-abc123 h1`,
  …). Used for per-page CSS scoping.

## Returned shape

```ts
interface TranslateResult<string> {
  output: string;
  emittedRules: readonly StyleRuleId[];
  warnings: readonly StyleWarning[];
}
```

- `output` — the full CSS string. No trailing newline (file emitter
  adds one).
- `emittedRules` — rule ids that actually made it into output. The
  AOT compiler intersects this with each page's `usedStyle`.
- `warnings` — everything that was skipped or degraded (unresolved
  `TokenRef`, unknown `ext:*` kind, `ext:*` context with no
  translator). Translator never throws on Style IR shape issues per
  FM04 §9.6.

## Mapping tables (FM04 §9.2)

### Properties

| Style IR | CSS |
|---|---|
| `color` | `color: <value>` |
| `background` | `background-color: <value>` |
| `border-color` / `outline-color` | `border-color:` / `outline-color:` |
| `font-family` | `font-family: <stack>` (quotes families with spaces) |
| `font-size` | `font-size: <length>` |
| `font-weight` | `font-weight: <number>` |
| `font-style` | `font-style: <value>` |
| `text-transform` | `text-transform: <value>` |
| `leading` | `line-height: <number>` |
| `tracking` | `letter-spacing: <length>` |
| `text-decoration` | `text-decoration: <line> [style] [color] [thickness]` |
| `space-before` | `margin-top: <length>` |
| `space-after` | `margin-bottom: <length>` |
| `indent` | `text-indent: <length>` |
| `padding` | `padding: <top> <right> <bottom> <left>` |
| `max-width` / `min-height` | `max-width:` / `min-height:` |
| `align` | `text-align: <value>` |
| `vertical-align` | `vertical-align: <value>` |
| `border` (all sides) | `border: <width> <style> <color>` |
| `border` (per-side) | `border-<side>: <width> <style> <color>;` per side |
| `border-radius` | `border-radius: <length>` |
| `shadow` | `box-shadow: [inset] <oX> <oY> <blur> <spread> <color>` |
| `opacity` | `opacity: <number>` |
| `column-break: before/after` | `break-before/after: column` |
| `column-break: avoid` | `break-inside: avoid-column` |
| `page-break: before/after` | `break-before/after: page` |
| `page-break: avoid` | `break-inside: avoid-page` |
| `widow-orphan` | `widows: <n>; orphans: <n>` |
| `display` | `display: <value>` |
| `visible: false` | `visibility: hidden` |
| `visible: true` | `visibility: visible` |
| `ext:*` | warn-and-skip (per FM04 §9.6) |

### Selectors

| Style IR | CSS |
|---|---|
| `node-type` | element selector (`p`, `blockquote`, …) |
| `node-type-level` heading | `h1`–`h6` |
| `custom-kind` | `[data-kind="<name>"]` |
| `tag` | `[data-tag~="<name>"]` |
| `id` | `#<id>` |
| `role` | `[role="<name>"]` |
| `nth` literal (0-based IR) | `:nth-child(<n+1>)` (1-based CSS) |
| `nth` formula | `:nth-child(<an+b>)` or `:nth-last-child(...)` |
| `child-of` | `<parent> > <child>` |
| `descendant-of` | `<ancestor> <descendant>` |
| `adjacent` | `<previous> + <following>` |
| `and` | concatenate (cartesian product over inner `or`s) |
| `or` | comma-separate |
| `not` | `:not(<inner>)` |

Composition combinations work as expected — `and(or(p, h1), .intro)`
expands to `p[data-tag~="intro"], h1[data-tag~="intro"]`.

### Contexts

| Style IR | CSS |
|---|---|
| `print` | `@media print` |
| `screen` | `@media screen` |
| `dark` | `@media (prefers-color-scheme: dark)` |
| `narrow` | `@media (max-width: 40rem)` |
| `wide` | `@media (min-width: 80rem)` |
| `reduced-motion` | `@media (prefers-reduced-motion: reduce)` |
| `high-contrast` | `@media (prefers-contrast: more)` |
| `ext:*` | warn-and-skip |

## TokenRef resolution

Every property that accepts `<T> | TokenRef` (color, length, number,
shadow, font-stack) resolves the ref against `doc.tokens` at
translation time. Chains follow up to 8 hops (covers any sensible
design system); cycles return null and emit a warning per FM04 §9.6.

The translator only emits properties whose refs resolve to the
*expected* type — e.g. asking for `color` but the ref resolves to a
`Length` returns null + warning rather than emitting nonsense CSS.

## Reproducibility

The translator is **pure**:

- No clock reads, no random, no `process.env`
- Same input → byte-identical output
- Drives FM03 reproducible builds

Order of emission:

1. Unconditional rules (no `context`) in source order — outermost in
   the cascade.
2. `@media` blocks, one per active context, each containing the
   rules for that context in source order.

Within a `@media` block, rules are emitted source-order — preserving
FM04 §4.9 "later beats earlier" specificity in the resulting CSS.

## Security

- Validator-side input shape is the producer's responsibility; this
  package is structurally tolerant — bad shapes emit warnings and
  skip, never throw.
- Selector identifiers / attribute values pass through CSS escape
  helpers (`\<hex>` for identifiers, `\"` / `\\` inside attribute
  values). Defensive against attacker-controlled rule definitions.
- No I/O, no network, no env, no shell.

## Tests

```
npx vitest run --coverage
```

114 tests across 6 files:
- `value-mappers.test.ts` (18 tests) — color/length/font-stack/shadow
- `selector-mapper.test.ts` (22 tests) — every selector form
- `context-mapper.test.ts` (3 tests) — context → @media body
- `token-resolver.test.ts` (12 tests) — TokenRef resolution + cycle detection
- `property-mappers.test.ts` (40 tests) — every kernel property kind
- `translate.test.ts` (19 tests) — end-to-end integration + slicing + scoping

Coverage: **98.92% line / 94.23% branch** — above the FM04 §14.4
≥95% line target.

## Roadmap

- ✅ `forme-style-ir` (#3400) — the IR substrate
- ✅ `forme-style-to-css` (this package) — reference CSS translator
- ⏳ `forme-style-theme` — theme registry + composition (FM04 §13.3)
- ⏳ `forme-style-to-latex` / `-pdf` / `-terminal` — other backends
  (FM04 §9.3-9.5; v0.2)

## Dependencies

- `@coding-adventures/forme-style-ir` — the IR consumed.
- `@coding-adventures/forme-types` — `JsonValue` (transitively).
