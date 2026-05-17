# @coding-adventures/forme-style-to-latex

The **second FM04 backend translator**. Takes a `StyleDocument` from
[`@coding-adventures/forme-style-ir`](../forme-style-ir) and emits a
LaTeX preamble fragment (`\definecolor` / `\setlength` /
`\newcommand` macros / `\if<flag>` conditionals).

Implements [FM04 §9.3](../../specs/FM04-forme-style-ir.md). Sister of
[`forme-style-to-css`](../forme-style-to-css) (the §9.2 CSS
backend) — they share the IR, the validator-trust posture, and the
warn-and-skip robustness contract, but differ in everything they emit.

## Quick start

```ts
import { translateToLatex } from "@coding-adventures/forme-style-to-latex";
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
        { kind: "color",   value: { kind: "token-ref", path: "colors.text" } },
        { kind: "leading", value: 1.5 },
      ],
    },
  ],
};

const { output, emittedRules, warnings } = translateToLatex(doc, {
  activeContexts: ["print"],
});

console.log(output);
// → "% forme-style-to-latex generated preamble
//    % --- context flags ---
//    \newif\ifprint \newif\ifscreen ...
//    % --- rules ---
//    % rule "body" — node type: paragraph
//    \newcommand{\formeNodeParagraph}{%
//      \color{RGB}{31,35,40}%
//      \linespread{1.5}\selectfont%
//    }"
```

## Translation strategy (FM04 §9.3)

**Selectors → named macros.**  LaTeX has no document tree to walk
against in the preamble, so each rule becomes a stable command the
document body invokes:

| Selector kind   | Macro form              | Example                  |
|-----------------|-------------------------|--------------------------|
| `node-type`     | `\formeNode<Type>`      | `\formeNodeParagraph`    |
| `node-type-level` | `\formeHeading<Word>` | `\formeHeadingOne`       |
| `custom-kind`   | `\formeKind<Slug>`      | `\formeKindCallout`      |
| `tag`           | `\formeTag<Slug>`       | `\formeTagWarning`       |
| `id`            | `\formeId<Slug>`        | `\formeIdMain`           |
| `role`          | `\formeRole<Slug>`      | `\formeRoleNote`         |

Composition selectors (`and`, `or`, `not`, `nth`, `child-of`,
`descendant-of`, `adjacent`) require runtime document-tree walking
LaTeX has no equivalent for — they **warn and skip** per FM04 §9.6.

**Properties → LaTeX commands** where there's a natural form:

| Style IR kind     | LaTeX command(s)                       |
|-------------------|----------------------------------------|
| `color`           | `\color{RGB}{r,g,b}` (xcolor)          |
| `font-family`     | `\setmainfont{...}` (fontspec)         |
| `font-size`       | `\fontsize{...}{...}\selectfont`       |
| `font-weight`     | `\fontseries{m|b|bx}\selectfont`       |
| `font-style`      | `\fontshape{n|it|sl}\selectfont`       |
| `leading`         | `\linespread{n}\selectfont`            |
| `space-before/after` | `\setlength{\parskip}{...}`         |
| `indent`          | `\setlength{\parindent}{...}`          |
| `max-width`       | `\setlength{\linewidth}{...}`          |
| `align`           | `\raggedright` / `\raggedleft` / `\centering` / justify-glue |
| `column-break`    | `\columnbreak` / `\nobreak`            |
| `page-break`      | `\pagebreak` / `\nopagebreak`          |
| `widow-orphan`    | `\widowpenalty=N\clubpenalty=N`        |
| `visible`         | `\let\formeVisible=\relax|\hphantom`   |

Decorative properties without a preamble equivalent
(`background`, `border`, `border-radius`, `shadow`, `opacity`,
`padding`, `display`, `vertical-align`, `tracking`) **warn and skip**.
They typically require TikZ / tcolorbox at the *call site* rather
than as preamble configuration.

**Contexts → `\if<flag>` conditionals**:

| Context           | Conditional        |
|-------------------|--------------------|
| `print`           | `\ifprint`         |
| `screen`          | `\ifscreen`        |
| `dark`            | `\ifdark`          |
| `narrow`          | `\ifnarrow`        |
| `wide`            | `\ifwide`          |
| `reduced-motion`  | `\ifreducedmotion` |
| `high-contrast`   | `\ifhighcontrast`  |

The translator emits the `\newif\if<flag>` declarations at the top of
the preamble so document authors can toggle them at compile time
(e.g. `\printtrue` to activate the `print` context).

## Unit conversions (documented assumptions)

| IR unit | LaTeX form        | Conversion                       |
|---------|-------------------|----------------------------------|
| `pt`    | `Npt`             | passthrough                      |
| `mm`, `in`, `ex`, `em` | `N<unit>` | passthrough               |
| `px`    | `Npt`             | 1px = 0.75pt (CSS standard)      |
| `rem`   | `Nem`             | rem ≈ em (LaTeX has no "root em"; document author tunes the root font size) |
| `%`, `vh`, `vw`, `ch` | (skip)  | no page-geometry context in preamble |

## Color models

`xcolor`'s `RGB` model is the lingua franca; HSL is converted inline
(lossless within sRGB).  OKLCH warn-skips for v0 — round-tripping
through CIE conversion is out of scope.  Named colors fall back to a
small built-in safe map (matches `\usepackage{xcolor}` with no
options); unknown names warn-skip.

## LaTeX special-character escaping

All user-controlled strings (selector targets, color names, font
names, rule ids) route through `escape.ts` first.  The ten LaTeX
specials (`\ % $ & _ # { } ^ ~`) become their canonical escape
forms; ASCII control characters (0x00–0x1F, 0x7F) are stripped.

The backslash and accent escapes use placeholder substitution so
their synthetic `{` / `}` don't get double-escaped on the brace pass.

## Security posture

Three concerns explicitly verified before push:

1. **LaTeX injection via insufficient escaping.**  Every interpolated
   string routes through `escapeLatexText` (text-mode) or `latexIdent`
   (command-name).  Tests pin escape coverage for all ten specials
   and round-trip through every public-facing string path.
2. **Prototype-pollution in `walkPath`.**  Mirrors
   `forme-style-to-css`'s defence — deny-listed segments + own-key
   `hasOwnProperty` check.  Three tests pin the rejections.
3. **Control-character handling.**  Stripped from every escape
   helper before further processing.  Tests pin behaviour for both
   text and identifier paths.

`scope` is **caller-trusted** (concatenated verbatim) — same posture
as `forme-style-to-css`'s `scope`.  Documented in the JSDoc.

## Spec divergences

None known.  Implements FM04 §9.3 LaTeX target end-to-end.

## v0 simplifications

- **OKLCH colors warn-skip** — round-trip through CIE / sRGB
  gamut mapping is out of scope; add when a real document needs it.
- **LTR-only `align`** — `start` → `\raggedright`, `end` →
  `\raggedleft`.  A future i18n layer (`ext:i18n:*`) re-emits
  contextually for RTL.
- **`tracking` warns** — needs `microtype`'s `\letterspacing`;
  emit manually until we want microtype as a documented dependency.
- **`text-decoration: line-through` warns** — needs `ulem`'s
  `\sout{}`.

## Tests

155 tests across 7 files:

- `escape.test.ts` (18 — every LaTeX-special, control-char strip,
  identifier sanitisation, all-ten-in-one composite)
- `value-mappers.test.ts` (25 — color models, length units,
  font-stack escaping, fallback comments)
- `selector-mapper.test.ts` (20 — simple kinds, identifier
  sanitisation, composition warn-skips, defensive numeric encoding)
- `context-mapper.test.ts` (5 — kernel contexts + flag declarations)
- `token-resolver.test.ts` (14 — happy path, prototype-pollution
  defence, typed wrappers, cycle cap, NaN rejection)
- `property-mappers.test.ts` (52 — every kernel kind, exhaustive
  meta-check, defensive fallthroughs)
- `translate.test.ts` (16 — happy path, filtering, scope, important,
  reproducibility, LaTeX-injection defence)

Coverage: **100% line / 96.15% branch** — above FM04 §14.4
≥95% line target.
