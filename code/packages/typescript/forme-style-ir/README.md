# @coding-adventures/forme-style-ir

The **Forme Style IR** — design tokens, selectors, rules, contexts,
and the top-level `StyleDocument` value. The typed substrate that
translators (CSS, LaTeX, terminal, PDF) all consume.

Implements [FM04](../../specs/FM04-forme-style-ir.md). First of three
planned packages in the FM04 family; the CSS and theme translators
will follow.

## What's in the box

```
src/
├── tokens.ts            — TokenSet, Color, Length, Shadow, TokenRef, typography
├── selectors.ts         — Selector union (13 forms) + sel.* constructors
├── properties.ts        — StyleProperty union (29 kernel kinds + ext:*)
├── contexts.ts          — CONTEXT_PRINT / SCREEN / DARK / NARROW / WIDE / …
├── style-document.ts    — StyleDocument, Theme, StyleRule, StyleRuleId
├── style-error.ts       — StyleError (throw), StyleWarning (return)
├── validate.ts          — validateStyleDocument(value): collect-all-then-throw
└── canonical.ts         — canonicalStyleDocument(doc): byte-stable JSON for hashing
```

Single runtime dependency on `@coding-adventures/forme-types` for
`JsonValue` / `ReadonlyRecord`. Pure types + a validator + a
serialiser. No I/O, no network, no environment access.

## Quick taste

```ts
import {
  styleRuleId, sel,
  type StyleDocument, type StyleProperty,
} from "@coding-adventures/forme-style-ir";

const properties: readonly StyleProperty[] = [
  { kind: "color",       value: { kind: "token-ref", path: "colors.text" } },
  { kind: "font-family", value: { kind: "token-ref", path: "typography.families.body" } },
  { kind: "font-size",   value: { kind: "token-ref", path: "typography.scale.md" } },
];

const doc: StyleDocument = {
  kind: "StyleDocument",
  tokens: {
    colors: {
      text: { kind: "rgb", r: 31, g: 35, b: 40 },
    },
    typography: {
      families: { body: ["Inter", "system-ui", "sans-serif"] },
      scale:    { md: { unit: "rem", value: 1 } },
      weights:  { regular: 400 },
      leading:  { normal: 1.5 },
      tracking: { normal: { unit: "em", value: 0 } },
    },
    space: {}, radii: {}, shadows: {},
  },
  rules: [
    { id: styleRuleId("body-text"), selector: sel.type("paragraph"), properties },
  ],
  contexts: [],
  theme: null,
};
```

## Tokens (FM04 §3)

Design-system primitives that rules reference by *name*. The naming
layer is what lets themes re-bind the same name to a different
concrete value without rewriting any rule.

- **Colors** in four representations: `rgb`, `hsl`, `oklch`, `named`.
  Translators convert between them.
- **Lengths** in 11 units (`px`, `rem`, `em`, `%`, `vh`, `vw`, `pt`,
  `mm`, `in`, `ch`, `ex`). Print backends prefer absolute; web
  backends prefer relative. Both are first-class.
- **Typography** — font stacks, type scale, weights, leading,
  tracking — each keyed by author-chosen names.
- **Spacing** + **radii** scales.
- **Shadows** with explicit offsets / blur / spread / color / inset.
- **Extension slot** under `ext:<plugin>:<group>` for
  plugin-contributed token groups.

`TokenRef` (`{ kind: "token-ref", path: "colors.text" }`) is how a
rule refers to a token by dotted path. The validator checks shape;
*resolution* is the translator's job (because that's where the
composed theme is in scope).

## Selectors (FM04 §4)

Thirteen forms — intentionally smaller than CSS's 30+:

- **Identity**: `node-type`, `node-type-level` (heading 1–6 lifted to
  first-class), `custom-kind`, `tag`, `id`, `role`.
- **Position**: `nth` with literal index or CSS-style `an+b` formula.
- **Structural relations**: `child-of`, `descendant-of`, `adjacent`.
- **Composition**: `and`, `or`, `not`.

**Specificity is source-order only**. Later rules win. No CSS-style
"ID beats class" calculation — CSS specificity is famously
surprising; source order is predictable.

`sel.*` ergonomic constructors mean you can write
`sel.and(sel.type("paragraph"), sel.tag("intro"))` rather than the
full object literal.

## Properties (FM04 §5)

A **closed list** of ~30 property kinds, intentionally finite. The
discriminated union gives backends exhaustive type-safety: a `switch
(prop.kind)` covers every case, and adding a new kind triggers a
compile error in every translator that doesn't yet handle it.

Anything outside the closed list lives under `ext:<plugin>:<name>`
— backends that understand a given extension act on it; others
ignore it per FM04 §9.6.

The `important` flag exists but is discouraged. Use only when a theme
system genuinely needs to override base rules.

## Contexts (FM04 §6)

Named gating conditions. Translators activate some subset; rules
whose `context` is in the active set (or has no context) apply.

Kernel-blessed contexts: `print`, `screen`, `dark`, `narrow`,
`wide`, `reduced-motion`, `high-contrast`. Plugin contexts follow
`ext:<plugin>:<name>`.

A rule has at most **one** context. To express "print AND
high-contrast" the producer declares two rules with the same
selector + properties. Compound contexts open AND/OR ambiguity
questions we sidestep.

## Validation (FM04 §14.1)

```ts
const { document, warnings } = validateStyleDocument(maybeDoc);
```

- **Errors** are thrown as a single `StyleError` carrying every
  violation in one walk (same pattern as `forme-pipeline-config`'s
  `ConfigError`). No bail-on-first.
- **Warnings** are returned alongside the validated document — soft
  issues like a context name that's neither standard nor `ext:*`
  (likely a typo) or a context that the document doesn't declare in
  its `contexts` list.

Documented rejection reasons (14 error codes in `STYLE_ERROR_CODES`):

| Code | When |
|---|---|
| `MALFORMED` | wrong type / missing required field |
| `DUPLICATE_RULE_ID` | two rules share an id |
| `EMPTY_RULE_ID` | id is an empty string |
| `UNKNOWN_PROPERTY_KIND` | kind not in `PROPERTY_KINDS` and not `ext:*` |
| `UNKNOWN_SELECTOR_KIND` | kind not in `SELECTOR_KINDS` |
| `INVALID_HEADING_LEVEL` | `node-type-level` level outside 1–6 |
| `INVALID_TOKEN_REF_PATH` | bad dotted-identifier path |
| `INVALID_LENGTH_UNIT` | length unit not in `LENGTH_UNITS` |
| `INVALID_COLOR` / `INVALID_COLOR_CHANNEL` | malformed color |
| `INVALID_PROPERTY_VALUE` | value shape wrong for the kind |
| `EMPTY_COMPOSITION` | `and`/`or` with empty inner array |
| `UNKNOWN_CONTEXT` | (warning) context not standard and not `ext:*` |
| `INVALID_EXTENSION_KEY` | token-set extension key doesn't match `ext:<package>:<group>` |

## Canonical serialisation (FM04 §12)

```ts
const bytes = canonicalStyleDocument(doc);
// → byte-stable JSON suitable for hashing
```

For FM03 reproducible builds: same document (deep-equal by value)
⇒ identical bytes ⇒ identical hash. Rules:

- Object keys are emitted in lexicographic order at every depth.
- `rules` array order is **significant** (per §4.9 source order is
  specificity) and preserved.
- `contexts` array is treated as a **set** — sorted before emitting.
- `undefined` values are dropped (matches `JSON.stringify`).
- Non-finite numbers throw `RangeError` — the validator should catch
  them upstream; this is the last line of defence.

## Themes (FM04 §7)

A `Theme` is itself a partial `StyleDocument`:

```ts
interface Theme {
  readonly name: string;
  readonly tokens?: Partial<TokenSet>;   // sparse overrides
  readonly rules?: readonly StyleRule[]; // appended after base rules
}
```

Composition (FM04 §7.2) is deep-merge of tokens + append of rules.
This package defines the **type**; the composition function and
theme registry live in the follow-up `forme-style-theme` package.

## Tests

```
npx vitest run --coverage
```

144 tests, **98.06% line / 96.3% branch coverage** — above the FM04
§14.4 95% target. Tests are organised by concern: tokens, selectors,
properties, contexts, validate (errors), validate-coverage
(happy-path variants), canonical (round-trip + hash stability).

## What's NOT in this package

- **Translators** — `forme-style-to-css`, `-latex`, `-pdf`,
  `-terminal` are separate packages (FM04 §13.2 / §9.3-9.5). This
  package is the *substrate*; translators are the *consumers*.
- **Theme composition + registry** — `forme-style-theme` (FM04
  §13.3). The `Theme` type lives here; the composition function
  doesn't.
- **`TokenRef` resolution** — happens in the translator, because
  that's where the composed theme is in scope. The validator only
  checks `TokenRef` *shape* (dotted-identifier path).
- **Unknown-property graceful degradation** — that's a translator
  concern per FM04 §9.6. The validator rejects unknown kernel kinds
  outright; translators are the layer that warns-and-skips.

## Dependencies

- `@coding-adventures/forme-types` — `JsonValue`, `ReadonlyRecord`.
