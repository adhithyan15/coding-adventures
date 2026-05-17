# @coding-adventures/forme-style-orchestrator

One-call orchestration over the FM04 Style IR family.  Wraps
**validate → compose theme → dispatch to translator** into a single
`compile(doc, target, options)` entry point so users don't have to
re-wire the four sibling packages by hand on every call.

Sixth (and capstone) package of the FM04 family; sits next to
[`forme-style-ir`](../forme-style-ir),
[`forme-style-theme`](../forme-style-theme),
[`forme-style-to-css`](../forme-style-to-css),
[`forme-style-to-latex`](../forme-style-to-latex), and
[`forme-style-to-terminal`](../forme-style-to-terminal).

Per [FM04 §13 (composition concerns)](../../specs/FM04-forme-style-ir.md)
and [FM03 (orchestrator)](../../specs/FM03-forme-orchestrator.md).
No new spec ground broken — pure integration glue.

## Quick start

```ts
import { compile, isCompileError } from "@coding-adventures/forme-style-orchestrator";
import { createThemeRegistry } from "@coding-adventures/forme-style-theme";

const themes = createThemeRegistry();
themes.register({
  name: "dark",
  tokens: { colors: { text: { kind: "rgb", r: 240, g: 240, b: 240 } } },
});

const result = compile(doc, "css", {
  activeContexts: ["screen"],
  theme: "dark",
  themeRegistry: themes,
});

if (isCompileError(result)) {
  console.error("validation failed:", result.errors);
} else {
  console.log(result.output);      // CSS text
  console.log(result.emittedRules);// which rule ids landed
  console.log(result.warnings);    // translator-level warnings
}
```

## The pipeline

```
        ┌──────────────────────────────────────────────────┐
        │ 1. validateStyleDocument(doc)                    │
        │    → throws StyleError → CAPTURED in result.errors│
        │    → returns { document, warnings }              │
        └──────────────────────────────────────────────────┘
                              │
                              ▼
        ┌──────────────────────────────────────────────────┐
        │ 2. options.theme supplied?                       │
        │    • Theme value → composeWithTheme(doc, theme)  │
        │    • string name → themeRegistry.lookup(name)    │
        │      → found    → composeWithTheme(...)          │
        │      → missing  → WARNING "THEME_NOT_FOUND"; base│
        │      → no registry → throw TypeError             │
        └──────────────────────────────────────────────────┘
                              │
                              ▼
        ┌──────────────────────────────────────────────────┐
        │ 3. dispatch by target                            │
        │    • "css"      → translateToCss(doc, options)   │
        │    • "latex"    → translateToLatex(doc, options) │
        │    • "terminal" → translateToTerminal(doc, opts) │
        │    • else       → throw TypeError                │
        └──────────────────────────────────────────────────┘
                              │
                              ▼
              { target, output, emittedRules, warnings, errors }
```

## API

```ts
function compile(
  doc: unknown,
  target: "css" | "latex" | "terminal",
  options: CompileOptions,
): CompileResult;

function isCompileError(r: CompileResult): boolean;
function isCompileSuccess(r: CompileResult): boolean;

/** Convenience: validate + canonicalise → byte-stable fingerprint. */
function fingerprintDocument(doc: unknown): string | null;

interface CompileOptions {
  activeContexts: readonly string[];
  usedRuleIds?: readonly StyleRuleId[];
  scope?: string;
  theme?: string | Theme;
  themeRegistry?: ThemeRegistry;
}

interface CompileResult {
  target: CompileTarget;
  output: string;             // empty when errors non-empty
  emittedRules: readonly StyleRuleId[];
  warnings: readonly StyleWarning[];
  errors: readonly StyleErrorEntry[];
}
```

## What it throws vs. what it captures

**Captures into `result.errors` (never throws):**
- Validator rejection (`StyleError`)

**Throws (programmer error):**
- `target` outside `"css" | "latex" | "terminal"` — `TypeError`
- `options.theme` is a string but `options.themeRegistry` is absent — `TypeError`

**Warns (in `result.warnings`):**
- Theme name not found in registry (`THEME_NOT_FOUND`) — translation still happens with base tokens
- Whatever the translator warns (unresolved tokens, `ext:*` kinds without translators, model-not-expressible colors)
- Whatever the validator warned (carried forward)

## Reproducibility (FM03)

`compile(doc, target, options)` is **pure**: same triple → byte-identical output.  This drives FM03's content-addressed cache and FM06's AOT compiler.

`fingerprintDocument(doc)` is the convenience helper for cache keys — runs the validator + canonical serializer + returns a byte-stable string (or `null` if validation fails).

## Security posture

Three concerns inherited from upstream — all preserved at the orchestrator boundary:

1. **Validator-error capture.**  `StyleError.errors[]` is copied via spread into a new frozen array; the error object itself doesn't leak.  No stack trace included in the result.
2. **Theme registry lookup.**  Pass-through to `forme-style-theme`'s registry which is `Map`-backed (own-key semantics) and refuses prototype-pollution names defensively.
3. **Error message safety.**  Error / warning messages route user-supplied theme names through `JSON.stringify` — never raw — so a malicious name like `"\n\rEvil"` lands in the message as the JSON-escaped form `"\\n\\rEvil"`.

## Tests

21 tests in one file:

- Happy-path dispatch (CSS / LaTeX / terminal)
- Validator failure capture (`null` input, structurally-broken doc)
- Theme composition (by value, by name via registry, by unknown name, none)
- Throws on bad target + bad theme-with-no-registry combinations
- Options pass-through (`activeContexts`, `usedRuleIds`, `scope`)
- Reproducibility — same input → byte-identical output
- Type guards (`isCompileError` / `isCompileSuccess`) complementary
- `fingerprintDocument` on valid / invalid input

Coverage: **97.77% line / 94.73% branch** — above the FM04 §14.4
≥95% line target.  Uncovered branches are the defensive
"validator threw non-`StyleError`" re-raise path (unreachable via
the validator's documented behaviour; defensive against future
breakage).
