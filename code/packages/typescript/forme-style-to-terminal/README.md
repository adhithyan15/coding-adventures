# @coding-adventures/forme-style-to-terminal

The **third FM04 backend translator**.  Takes a `StyleDocument` from
[`@coding-adventures/forme-style-ir`](../forme-style-ir) and emits a
**TypeScript module source string** that, when imported, exposes a
`ReadonlyMap<RuleId, { prefix, suffix }>` — per-rule ANSI SGR
wrappers the consumer drops around document content.

Implements [FM04 §9.4](../../specs/FM04-forme-style-ir.md).  The
third concrete backend (after [CSS](../forme-style-to-css) and
[LaTeX](../forme-style-to-latex)), making the multi-backend story
real: same IR, three independent targets, zero inter-package
coupling.

## Quick start

```ts
import { translateToTerminal } from "@coding-adventures/forme-style-to-terminal";
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
      id: styleRuleId("body"),
      selector: sel.type("paragraph"),
      properties: [
        { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
        { kind: "font-weight", value: 700 },
      ],
    },
  ],
};

const { output, emittedRules, warnings } = translateToTerminal(doc, {
  activeContexts: ["screen"],
});

// `output` is a TS module source string:
//
//   export interface AnsiStyle {
//     readonly prefix: string;
//     readonly suffix: string;
//   }
//   export const formeStyles: ReadonlyMap<string, AnsiStyle> = new Map([
//     // rule "body" — node-type:paragraph
//     ["body", { prefix: "\x1b[38;2;31;35;40;1m", suffix: "\x1b[0m" }],
//   ]);

// In the consumer (terminal renderer):
//   import { formeStyles } from "./generated";
//   const style = formeStyles.get(node.usedStyle);
//   if (style) process.stdout.write(style.prefix + text + style.suffix);
```

## Translation strategy (FM04 §9.4)

**Selectors are NOT mapped to anything.**  Terminals have no
document tree.  The Map key in the output is the **rule id** —
opaque, unique — and the consumer looks it up by id when rendering
a node whose `usedStyle` set contains that id.  Composition
selectors (`and`, `or`, `nth`, `child-of`, …) appear in the
per-rule comment as informational descriptions only.

**Properties → SGR fragments.**  Each rule combines all its
property's SGR parameters into a single `\x1b[<n>;<n>;<n>m`
sequence and the consumer wraps the text with a trailing
`\x1b[0m` reset.

| Style IR kind     | SGR                                  |
|-------------------|--------------------------------------|
| `color`           | `38;2;R;G;B` (foreground truecolour) |
| `background`      | `48;2;R;G;B` (background truecolour) |
| `font-weight` ≥600| `1` (bold)                           |
| `font-style` italic/oblique | `3` (italic)               |
| `text-decoration: underline`    | `4`                    |
| `text-decoration: line-through` | `9`                    |
| `text-decoration: overline`     | `53`                   |
| `visible: false`  | `8` (conceal)                        |

Everything else — `padding`, `margin`, `border`, `shadow`,
`opacity`, `font-size`, `font-family`, `align`, `vertical-align`,
`max-width`, `min-height`, page-breaks, etc. — **warn-skips**.
Terminals are a character grid with no concept of pixel layout,
typography choice, or page geometry.

**Contexts** filter rules through `activeContexts` (same shape as
the CSS/LaTeX translators).  There is no per-context conditional
emission machinery — the terminal IS what it is at render time;
the consumer chooses which contexts to activate by listing them.
`ext:*` contexts warn-skip per FM04 §9.6.

## Color models

| IR model | Terminal form                                 |
|----------|-----------------------------------------------|
| `rgb`    | direct (clamped + rounded to 0–255 ints)      |
| `hsl`    | converted inline (lossless within sRGB)       |
| `oklch`  | **warn-skip** (CIE round-trip out of scope v0)|
| `named`  | small built-in safe map (~15 common names)    |

## ANSI escape-sequence injection defence

Two attack surfaces, both addressed:

1. **Caller-controlled bytes never reach the terminal as control
   sequences.**  Every string interpolated into the output
   (`rule.id` for Map keys, selector descriptions for comments)
   routes through `stripAnsiUnsafe` (ESC, C1 CSI, C1 OSC,
   0x00–0x1F, 0x7F–0x9F) first.  Even if a hand-rolled IR bypasses
   the validator's grammar, the consumer's terminal cannot be
   driven by attacker-controlled escape sequences.

2. **TS-string-literal escaping.**  The output is a JS/TS module
   source; the Map keys and SGR strings land in double-quoted
   string literals.  A raw `\` or `"` in the input would terminate
   the literal early or alter neighbouring escape semantics.  The
   `escapeTsString` helper handles both, single-pass.

3. **`walkPath` prototype-pollution.**  Same deny-list + own-key
   `hasOwnProperty.call` defence as the CSS and LaTeX translators.

`scope` is **caller-trusted** (escaped for the TS string literal,
but otherwise concatenated verbatim).  Same posture as the CSS /
LaTeX translators' `scope`.

## Spec divergences

None.  Implements FM04 §9.4 end-to-end.

## v0 simplifications (documented)

- **24-bit truecolour only.**  Future option may add a 256-colour
  or 16-colour quantised fallback for older terminals.  Modern
  terminal emulators (iTerm2, kitty, Windows Terminal, GNOME
  Terminal ≥3.12, …) all support 24-bit.
- **OKLCH warn-skips** (CIE round-trip out of scope).
- **No SGR italic-on-italic / bold-on-bold idempotence elision** —
  the consumer is responsible for not double-wrapping the same
  rule on the same content.

## Tests

128 tests across 7 files:

- `escape.test.ts` (15 — ANSI-unsafe stripping for every dangerous
  byte range; TS-string escaping for `\` and `"`)
- `value-mappers.test.ts` (13 — color models, clamping, NaN
  defensiveness, named-color lookup, SGR fg/bg prefixes)
- `selector-mapper.test.ts` (19 — every Selector kind; composition
  forms; defensive control-char sanitisation; depth cap)
- `context-mapper.test.ts` (3 — kernel contexts; ext: rejection)
- `token-resolver.test.ts` (18 — happy path, proto-pollution
  defence, typed wrappers for all five leaf types)
- `property-mappers.test.ts` (44 — every kernel kind, exhaustive
  meta-check, defensive fallthroughs)
- `translate.test.ts` (16 — end-to-end, filtering, scope,
  reproducibility, ANSI-injection defence)

Coverage: **100% line / 97.43% branch** — above the FM04 §14.4
≥95% line target.
