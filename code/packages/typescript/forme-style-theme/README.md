# @coding-adventures/forme-style-theme

Theme registry + composition for Forme **Style IR**.  Three small
exports, each a focused FM04 concern:

| Export                 | Spec ref       | Concern                                                                |
|------------------------|----------------|------------------------------------------------------------------------|
| `composeWithTheme`     | FM04 §7.2      | Apply a `Theme`'s sparse token overrides + appended rules to a base `StyleDocument`.   |
| `createThemeRegistry`  | FM04 §13.3     | In-memory `{ register, lookup, list }` keyed by theme name.            |
| `resolveTokenRefs`     | FM04 §3.5      | Bulk-resolve `TokenRef`s against a document's tokens (analyser pre-pass). |

This is the **third** package of the FM04 family; the IR substrate
lives in [`forme-style-ir`](../forme-style-ir) and the reference CSS
translator in [`forme-style-to-css`](../forme-style-to-css).

## Why a separate package?

The FM04 spec deliberately splits the *IR* (types + validator),
*translators* (one per output format), and the *composition layer*
into separate packages so each can evolve independently — themes are
useful even when only the LaTeX translator is loaded, and the LaTeX
translator can ship without a theme registry if a backend doesn't
need one.

## Quick start

```ts
import {
  composeWithTheme, createThemeRegistry, resolveTokenRefs,
} from "@coding-adventures/forme-style-theme";
import {
  emptyStyleDocument, styleRuleId, sel,
  type Theme,
} from "@coding-adventures/forme-style-ir";

const base = {
  ...emptyStyleDocument(),
  tokens: {
    ...emptyStyleDocument().tokens,
    colors: {
      text: { kind: "rgb", r: 31, g: 35, b: 40 },
      link: { kind: "rgb", r: 9,  g: 105, b: 218 },
    },
  },
  rules: [
    {
      id: styleRuleId("body"),
      selector: sel.type("paragraph"),
      properties: [
        { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
      ],
    },
  ],
};

// 1. A theme registry — hot-reload friendly (replace-on-duplicate).
const themes = createThemeRegistry();
themes.register({
  name: "dark",
  tokens: { colors: { text: { kind: "rgb", r: 240, g: 240, b: 240 } } },
} as const satisfies Theme);

// 2. Composition — merge the theme onto the base document.
const dark = themes.lookup("dark");
if (dark) {
  const composed = composeWithTheme(base, dark);
  // composed.tokens.colors.text === { rgb 240/240/240 } (overridden)
  // composed.tokens.colors.link === { rgb 9/105/218 }   (preserved)
}

// 3. Bulk-resolve refs (analyser pre-pass — e.g. AOT CSS slicer).
const resolved = resolveTokenRefs(base, [
  { kind: "token-ref", path: "colors.text" },
  { kind: "token-ref", path: "colors.nonexistent" },
]);
// resolved.get("colors.text")        → { kind: "rgb", r: 31, g: 35, b: 40 }
// resolved.get("colors.nonexistent") → null
```

## Composition semantics (FM04 §7.2)

1. **Token override is per-named-entry within each bucket.**  A theme
   that contributes one color does not wipe other base colors;
   entries the theme doesn't mention stay at their base value.  This
   applies recursively into `typography`'s five sub-buckets
   (`families`, `scale`, `weights`, `leading`, `tracking`).
2. **Token values are atomic — no value-level merge.**  Overriding
   `colors.text` swaps the *whole* color value; you can't half-merge
   an `rgb` with an `hsl`.
3. **Rules are appended.**  Theme rules trail the base's, so they
   naturally win on equal-specificity ties per FM04 §4.9 (source
   order = specificity).
4. **`contexts`, `theme`, and `extensions` are preserved verbatim
   from `base`.**  Themes don't redeclare contexts or pivot the
   document's identity.
5. **Inputs are not mutated.**  `composeWithTheme` always returns a
   new `StyleDocument`.

## Registry semantics (FM04 §13.3)

- **Replace-on-duplicate** — registering the same name overwrites.
  The use case is dev-mode hot-reload; production stages register
  each theme once.
- **`list()` returns names sorted lexicographically** so downstream
  callers (config dumps, error messages, AOT manifests) are
  byte-stable across runs.
- **`lookup()` is read-only** — returns the registered `Theme` by
  reference (no defensive copy; `Theme` is `readonly` throughout the
  IR).
- **Each `createThemeRegistry()` call yields an independent
  instance** — no shared global state.

## Bulk resolution (FM04 §3.5)

`resolveTokenRefs(doc, refs)` returns a `Map<string, ResolvedValue |
null>` keyed by `ref.path`.  Null means unresolvable (path missing,
cycle, type mismatch, non-leaf landing).  Token-chain depth is
capped at 8 hops — covers any sensible design system and converts
cycles into `null` rather than a stack overflow.

Resolution is independent of any translator backend — the same map
feeds the CSS slicer (FM06), the LaTeX preamble extractor, theme
coverage reporters, and so on.

## Security posture

Two attack surfaces, both addressed:

1. **Deep-merge prototype-pollution.**  Token names like
   `__proto__`, `constructor`, and `prototype` are refused at every
   level of the bucket walk.  Merged records are backed by
   `Object.create(null)` so even if an attacker found a way past
   the deny-list, `Object.prototype` would still be untouched.
2. **Registry name pollution.**  The registry stores `Theme` values
   in a `Map<string, Theme>` (own-key semantics by construction)
   and additionally refuses the three forbidden names defensively.

`TokenRef.path` walks also refuse the forbidden segments and require
`hasOwnProperty`, defending against a hand-rolled `TokenRef` that
bypasses the validator's grammar.

## Spec divergences

None known.  Implements FM04 §7.2 / §13.3 / §3.5 verbatim.

## v0 simplifications

- **No persistent registry** — in-memory only.  Persistent backing
  (filesystem, database) lands when FM06 (AOT compiler) actually
  needs it.
- **No multi-theme composition** — `composeWithTheme(base, theme)`
  composes one theme at a time.  For multi-theme stacks, chain
  calls: `composeWithTheme(composeWithTheme(base, theme1), theme2)`.

## Tests

51 tests across 3 files:

- `compose.test.ts` (18 tests — token override, rule append,
  immutability, passthrough fields, extensions, reproducibility,
  prototype-pollution defence)
- `registry.test.ts` (13 tests — CRUD, replace-on-duplicate, input
  validation, isolation, bounded self-referential lookup)
- `resolve.test.ts` (20 tests — concrete leaves, chains, failure
  modes, bulk semantics, prototype traversal defence)

Coverage: **100% line / 96.51% branch** — above the FM04 §14.4
≥95% line target.
