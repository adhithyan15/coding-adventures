# FM04 — Forme Style IR: Design Tokens, Selectors, Rules, Contexts, Themes

> **Status:** Code-ready specification. Read alongside FM00 (vision),
> FM01 (kernel), FM03 (orchestrator).
> **Scope:** The Style intermediate representation — the typed,
> backend-agnostic shape that flows between style-producing stages
> (parsers, theme stages, transforms) and style-consuming stages
> (renderers, AOT compilers). The packages `forme-style-ir` and the
> per-backend translators `forme-style-to-css`,
> `forme-style-to-latex` (sketched; v0 ships only `-to-css`).
> **Out of scope:** Interactivity IR (FM05), the AOT compiler that
> derives per-page CSS slices (FM06), the dev-server and editor
> integration that lets authors preview style changes in real time
> (FM07).

---

## 0. Preface

FM00 §3 names three parallel intermediate representations: Content,
Style, Interactivity. FM01 §2.3.5 declared a stub `StyleDocument`
type — three fields (`tokens`, `rules`, `theme`) shaped just well
enough to let `Document` references compile without committing to a
shape. FM04 is the real shape.

The mental model: **a stylesheet is data, not code**. The same way
`document-ast` represents the *semantic* shape of content without
committing to HTML, the Style IR represents the *semantic* shape of
style without committing to CSS. Stages produce style values;
backends translate them. A single Style IR document can produce
CSS for the web, LaTeX style commands for print, PDF style
dictionaries, terminal ANSI codes, or nothing at all for a backend
that doesn't care.

What this enables:

1. **Per-page CSS slicing.** Each rendered page records which style
   rules it actually used; the AOT compiler emits per-page CSS that
   contains only those rules. Pages with no interactivity ship zero
   bytes of JS *and* tiny bytes of CSS.
2. **Backend swap.** Re-rendering the same content with a print
   backend doesn't need a new stylesheet — it needs a different
   translator. The Style IR is what survives the swap.
3. **Theme overlay.** A document can declare a base style plus a
   named theme override, and the consumer composes them. Themes are
   just more style IR values, not a separate concept.
4. **Plugin extension.** Plugins contribute new selectors,
   properties, and contexts without forking the IR — the same way
   they contribute new content kinds in FM01.

### 0.1 Relationship to other FM specs

- **FM00 §3.2** sketches the Style IR informally. FM04 pins it down.
- **FM01 §2.3.5** ships a stub `StyleDocument` interface; FM04
  replaces it without breaking field names (the stub's `tokens`,
  `rules`, `theme` survive into the real shape).
- **FM03** consumes `StyleDocument` values as part of `Document`;
  the orchestrator doesn't need to know the shape, only that it
  flows.
- **FM05 (Interactivity IR)** is parallel — same architectural
  pattern, different problem domain.
- **FM06 (AOT compiler)** consumes the per-page `usedStyle` list
  (FM01 §2.3.6) plus the full `StyleDocument` to produce per-page
  CSS / per-island JS / per-page PDF.

### 0.2 What this spec pins down

1. **Design tokens** — `TokenSet` shape, allowed kinds, references.
2. **Selectors** — the discriminated union of selector forms;
   composition via `and`/`or`/`nth`.
3. **Rules + properties** — `StyleRule` shape, the closed list of
   property kinds, their value types.
4. **Contexts** — print/screen/dark/mode etc.; the gating
   mechanism.
5. **Themes** — named theme references and the composition rule.
6. **The backend protocol** — what a translator must implement,
   how unknown properties degrade.
7. **Per-page collection** — how `RenderedPage.usedStyle` is
   populated and what it carries.
8. **Plugin extension** — how plugins add tokens, selectors,
   properties, contexts.
9. **Reproducible builds** — style-hash derivation, ordering rules.
10. **Package layout, testing, success criteria.**

### 0.3 Compatibility promise

`StyleDocument` and its components are stable within
`KERNEL_API_VERSION`. Adding a new property kind / selector form is
a backward-compatible minor; removing one requires a kernel major.
Plugin-contributed extensions follow the same rule scoped to their
own `apiVersion`.

---

## 1. Terminology

- **Style IR** — the typed value tree consumed and produced by
  stages. Lives in `@coding-adventures/forme-style-ir`.
- **StyleDocument** — the top-level value: tokens + rules +
  contexts + theme reference. The "stylesheet" of a Forme document.
- **Token** — a named design-system value (a color, a font size, a
  spacing unit). Referenced by name from properties so themes can
  swap them coherently.
- **Selector** — a value describing *which* nodes a rule applies
  to.
- **Rule** — a (selector, properties, context?) triple. The unit
  of style application.
- **Property** — one of the closed-list typed `StyleProperty`
  values (e.g. `{ kind: "color", value: ... }`).
- **Context** — a named gating condition: print-only, dark-mode-
  only, narrow-screen-only. Rules are conditionally applied based
  on context activation in the consumer.
- **Theme** — a named overlay that re-binds tokens (and optionally
  adds rules). One `StyleDocument` may reference one theme by
  name; the consumer composes by merging.
- **Backend** — the consumer that translates Style IR into a
  concrete output (CSS, LaTeX, terminal ANSI, PDF style dict).
- **Translator** — the package that performs the translation. One
  per backend.
- **usedStyle** — the per-page list of `StyleRuleId`s actually
  matched during rendering. Drives per-page CSS slicing.

---

## 2. The Three-IR Architecture (Restatement)

Skim if you've read FM00 §3.

```
Document = (Content, Style, Interactivity)
```

Each IR is independent. The renderer produces output from all
three; backends pick which they realise:

| Backend | Content | Style | Interactivity |
|---|---|---|---|
| HTML/CSS (web) | full | full | full |
| LaTeX (print) | full | full (subset) | drop |
| PDF | full | full (subset) | tiny subset (links, forms) |
| EPUB | full | full | reader-dependent |
| Terminal | text only | tiny subset (color, bold) | drop |
| Email | full | inline subset | drop |
| Image (social card) | text + image | full | drop |

The Style IR is rich enough that every backend has something to
do, but no backend is required to support every property. The
"unknown property" rule (§7.4) is what makes graceful degradation
work.

---

## 3. Design Tokens

Tokens are the design-system primitives — colors, type scales,
spacing scales, shadow definitions. They have **names**, and rules
**reference** them by name. The naming layer lets themes re-bind
the same name to a different concrete value without touching any
rule.

### 3.1 The `TokenSet` shape

```typescript
export interface TokenSet {
  /** Named colors.  Values are concrete color literals OR references
   *  to other tokens (so themes can build cascades). */
  readonly colors: ReadonlyRecord<string, Color | TokenRef>;

  /** Typography stack. */
  readonly typography: TypographyTokens;

  /** Spacing scale.  Indexed; rules say "space-2" or "space-md"
   *  rather than "0.5rem".  Reordering the scale is a theme concern. */
  readonly space: ReadonlyRecord<string, Length>;

  /** Corner-radius scale. */
  readonly radii: ReadonlyRecord<string, Length>;

  /** Drop-shadow definitions. */
  readonly shadows: ReadonlyRecord<string, Shadow>;

  /** Open-ended extension slot for plugin-contributed token groups.
   *  Keys are `ext:<plugin-name>:<group>`; values are JsonValue. */
  readonly extensions?: ReadonlyRecord<string, JsonValue>;
}

export interface TypographyTokens {
  readonly families: ReadonlyRecord<string, FontStack>;
  /** Type scale, indexed by name (`xs`, `sm`, `md`, `lg`, `xl`, …). */
  readonly scale: ReadonlyRecord<string, Length>;
  /** Numeric weight values (`regular`, `bold`, etc.). */
  readonly weights: ReadonlyRecord<string, number>;
  /** Line-height multipliers (`tight`, `normal`, `loose`). */
  readonly leading: ReadonlyRecord<string, number>;
  /** Letter-spacing values. */
  readonly tracking: ReadonlyRecord<string, Length>;
}

/** Comma-separated font fallback chain. */
export type FontStack = readonly string[];
```

### 3.2 Color values

```typescript
export type Color =
  | { readonly kind: "rgb";  readonly r: number; readonly g: number; readonly b: number; readonly a?: number }
  | { readonly kind: "hsl";  readonly h: number; readonly s: number; readonly l: number; readonly a?: number }
  | { readonly kind: "oklch"; readonly l: number; readonly c: number; readonly h: number; readonly a?: number }
  | { readonly kind: "named"; readonly name: string };
```

Why four representations:

- **rgb** is universal — every backend can produce a 24-bit RGB
  triple. Channel values are 0–255.
- **hsl** is the perceptually-intuitive one designers reach for.
- **oklch** is the modern perceptually-uniform color space; some
  backends produce it natively (CSS Color 4), others fall back to
  `rgb` via gamut mapping.
- **named** is a passthrough for backend-specific named colors
  (e.g. ANSI's `red`/`green`/etc. or LaTeX's xcolor names).

Translators are free to convert between representations as needed
to reach their output's native form.

### 3.3 Length values

```typescript
export type Length =
  | { readonly unit: "px";   readonly value: number }
  | { readonly unit: "rem";  readonly value: number }
  | { readonly unit: "em";   readonly value: number }
  | { readonly unit: "%";    readonly value: number }
  | { readonly unit: "vh";   readonly value: number }
  | { readonly unit: "vw";   readonly value: number }
  | { readonly unit: "pt";   readonly value: number }
  | { readonly unit: "mm";   readonly value: number }
  | { readonly unit: "in";   readonly value: number }
  | { readonly unit: "ch";   readonly value: number }
  | { readonly unit: "ex";   readonly value: number };
```

Print-focused backends (LaTeX, PDF) prefer absolute units (`pt`,
`mm`, `in`); web backends prefer relative ones (`rem`, `em`). Both
are first-class in the IR; translators convert as needed.

### 3.4 Shadow values

```typescript
export interface Shadow {
  readonly offsetX: Length;
  readonly offsetY: Length;
  readonly blur: Length;
  readonly spread: Length;
  readonly color: Color | TokenRef;
  readonly inset?: boolean;
}
```

Print backends typically drop shadows entirely; an emit-pdf
translator may convert to a thin outline or skip.

### 3.5 Token references

```typescript
export interface TokenRef {
  readonly kind: "token-ref";
  /** Path into the token tree, dotted: `"colors.primary"`,
   *  `"typography.scale.lg"`, `"space.md"`. */
  readonly path: string;
}
```

Rules use `TokenRef` instead of inlining concrete values whenever
the value should be theme-swappable. A property may also carry a
concrete value directly when theme-swap doesn't apply.

The translator resolves `TokenRef` against the active token set
(base tokens, optionally overlaid by theme tokens) at translate
time. Unresolved references emit a `StyleError` from the
translator; the Style IR itself never carries unresolved refs in
a valid value (the producer is responsible for declaring tokens
before referencing them).

### 3.6 Extension tokens

The `extensions` field gives plugins a place to attach
domain-specific tokens (e.g. a code-syntax-highlight plugin's
palette, a math-typography plugin's symbol scale). Keys follow the
manifest extension convention from FM01 §2.5: `ext:<package>:<group>`.

The IR itself is opaque to extension contents — translators that
understand a given extension key consume its contents, others
ignore.

---

## 4. Selectors

Selectors describe *which nodes* a rule applies to. The IR's
selector vocabulary is intentionally smaller than CSS: CSS has 30+
selectors, many of which encode legacy assumptions; the Style IR
has the dozen that matter for documents.

### 4.1 The selector union

```typescript
export type Selector =
  | NodeTypeSelector
  | NodeTypeLevelSelector
  | CustomKindSelector
  | TagSelector
  | IdSelector
  | RoleSelector
  | NthSelector
  | ChildOfSelector
  | DescendantOfSelector
  | AdjacentSelector
  | AndSelector
  | OrSelector
  | NotSelector;
```

Each form is documented below.

### 4.2 Node-type selectors

```typescript
/** Match every node of a given DocumentAst block- or inline-node type. */
export interface NodeTypeSelector {
  readonly kind: "node-type";
  /** A DocumentAst type name: "paragraph", "blockquote", "code_block",
   *  "list", "table", etc. */
  readonly type: string;
}

/** Match a heading at a specific level. */
export interface NodeTypeLevelSelector {
  readonly kind: "node-type-level";
  readonly type: "heading";
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
}
```

`node-type` covers the common case ("style every paragraph").
`node-type-level` exists because heading-level styling is so
universal that forcing it through a generic `node-type` +
attribute selector would be awkward; it's lifted to first-class.

### 4.3 Custom-kind selectors

```typescript
export interface CustomKindSelector {
  readonly kind: "custom-kind";
  /** A plugin-registered content kind, e.g. "callout", "youtube-embed". */
  readonly customKind: string;
}
```

Matches `RawBlockNode` / `RawInlineNode` / `CustomEmbedNode` whose
`kind` field equals `customKind`. This is how a plugin's added
content type gets first-class styling without forking the AST.

### 4.4 Tag and id selectors

```typescript
export interface TagSelector {
  readonly kind: "tag";
  /** Matches when the node's frontmatter (or ancestor's) declares
   *  this tag. */
  readonly tag: string;
}

export interface IdSelector {
  readonly kind: "id";
  readonly id: string;
}
```

`tag` is for content-tagged styling: "every post tagged `warning`
gets a red border." `id` is for one-off node-targeted overrides.

### 4.5 Semantic role selectors

```typescript
export interface RoleSelector {
  readonly kind: "role";
  /** ARIA-style role names: "navigation", "main", "complementary",
   *  "byline", "footnote", … */
  readonly role: string;
}
```

Roles let the same style apply across syntactic shapes — "the
byline of a post" might be a paragraph in one document and a
blockquote in another; tagging both with `role: byline` lets a
single rule cover both.

### 4.6 Position selectors

```typescript
export interface NthSelector {
  readonly kind: "nth";
  readonly of: Selector;
  /** 0-based index, OR a formula. */
  readonly n: number | NthFormula;
}

export interface NthFormula {
  /** Coefficient of n in `an + b`. */
  readonly a: number;
  /** Constant offset. */
  readonly b: number;
  /** Match "every nth" starting from the end? */
  readonly fromEnd?: boolean;
}
```

`nth` covers `first-child`, `last-child`, `odd`, `even`,
`every-third` patterns. The formula is the CSS `an+b` shape
because that vocabulary is well-understood; we don't introduce a
new name for the same idea.

### 4.7 Structural relation selectors

```typescript
/** Match a node that is a direct child of an outer-selector match. */
export interface ChildOfSelector {
  readonly kind: "child-of";
  readonly parent: Selector;
  readonly child: Selector;
}

/** Match a node that has an ancestor matching outer. */
export interface DescendantOfSelector {
  readonly kind: "descendant-of";
  readonly ancestor: Selector;
  readonly descendant: Selector;
}

/** Match a node immediately following a previous-sibling match. */
export interface AdjacentSelector {
  readonly kind: "adjacent";
  readonly previous: Selector;
  readonly following: Selector;
}
```

These cover the CSS `>` (child), descendant (space), and `+`
(adjacent sibling) combinators. We do NOT include the `~` general-
sibling combinator — its semantics are rarely useful for documents
and add complexity to the matcher.

### 4.8 Composition

```typescript
export interface AndSelector {
  readonly kind: "and";
  readonly all: readonly Selector[];
}

export interface OrSelector {
  readonly kind: "or";
  readonly any: readonly Selector[];
}

export interface NotSelector {
  readonly kind: "not";
  readonly inner: Selector;
}
```

`and` matches nodes satisfying every inner selector. `or` matches
nodes satisfying any. `not` matches nodes that don't match the
inner one (typically used inside `and`).

Composition is the right model for documents — CSS's class
proliferation is partly because its selectors don't compose
ergonomically. Style IR rules typically have one composed selector
expressing intent precisely.

### 4.9 Specificity

Selector matching is unambiguous; specificity matters only when
multiple rules match the same node. The Style IR's specificity
rule:

1. **Rules declared later in `StyleDocument.rules` win.** A rule
   index is the source-order index. Later beats earlier.
2. **Rules from a theme override beat rules from the base
   StyleDocument**, regardless of position. Themes are explicit
   overrides.
3. **The `important: true` flag on a property promotes it.**
   Among multiple `important: true` properties matching the same
   node, the same source-order rule applies. `important` is
   discouraged but supported for theme-system corner cases.

There is NO selector-specificity calculation (CSS-style "ID
selectors beat class selectors"). The rationale: CSS specificity
is famously surprising. Source order is predictable.

---

## 5. Rules and Properties

### 5.1 The `StyleRule` shape

```typescript
export interface StyleRule {
  /** Opaque branded id; populated by the producer.  Used for
   *  per-page `usedStyle` tracking and AOT slicing. */
  readonly id: StyleRuleId;
  readonly selector: Selector;
  readonly properties: readonly StyleProperty[];
  /** Optional context.  If absent, rule applies unconditionally.
   *  See §6. */
  readonly context?: string;
}

export type StyleRuleId = string & { readonly __brand: "StyleRuleId" };
```

The producer (a stage that emits style — a parser plugin, a theme
stage, an editor surface) is responsible for assigning unique
`id`s. The translator may use them as anchors but must not depend
on any internal structure beyond uniqueness.

### 5.2 Property closed list

The Style IR's property vocabulary is intentionally finite. This
is the inverse of CSS's "every property opaque to the parser"
approach: we define a closed enum so backends can give type-safe
exhaustive handling.

```typescript
export type StyleProperty =
  // ─── Color / fill ─────────────────────────────────────────────────
  | { kind: "color";              value: Color | TokenRef; important?: boolean }
  | { kind: "background";         value: Color | TokenRef; important?: boolean }
  | { kind: "border-color";       value: Color | TokenRef; important?: boolean }
  | { kind: "outline-color";      value: Color | TokenRef; important?: boolean }

  // ─── Typography ───────────────────────────────────────────────────
  | { kind: "font-family";        value: FontStack | TokenRef; important?: boolean }
  | { kind: "font-size";          value: Length | TokenRef; important?: boolean }
  | { kind: "font-weight";        value: number | TokenRef; important?: boolean }
  | { kind: "font-style";         value: "normal" | "italic" | "oblique"; important?: boolean }
  | { kind: "text-transform";     value: "none" | "uppercase" | "lowercase" | "capitalize"; important?: boolean }
  | { kind: "leading";            value: number | TokenRef; important?: boolean }
  | { kind: "tracking";           value: Length | TokenRef; important?: boolean }
  | { kind: "text-decoration";    value: TextDecoration; important?: boolean }

  // ─── Layout / spacing ─────────────────────────────────────────────
  | { kind: "space-before";       value: Length | TokenRef; important?: boolean }
  | { kind: "space-after";        value: Length | TokenRef; important?: boolean }
  | { kind: "indent";             value: Length | TokenRef; important?: boolean }
  | { kind: "padding";            value: BoxSides<Length | TokenRef>; important?: boolean }
  | { kind: "max-width";          value: Length | TokenRef; important?: boolean }
  | { kind: "min-height";         value: Length | TokenRef; important?: boolean }
  | { kind: "align";              value: "start" | "end" | "center" | "justify"; important?: boolean }
  | { kind: "vertical-align";     value: "baseline" | "top" | "middle" | "bottom"; important?: boolean }

  // ─── Decoration ───────────────────────────────────────────────────
  | { kind: "border";             value: BorderSpec; important?: boolean }
  | { kind: "border-radius";      value: Length | TokenRef; important?: boolean }
  | { kind: "shadow";             value: Shadow | TokenRef; important?: boolean }
  | { kind: "opacity";            value: number; important?: boolean }

  // ─── Page break (print) ───────────────────────────────────────────
  | { kind: "column-break";       value: "before" | "after" | "avoid"; important?: boolean }
  | { kind: "page-break";         value: "before" | "after" | "avoid"; important?: boolean }
  | { kind: "widow-orphan";       value: number; important?: boolean }

  // ─── Visibility ───────────────────────────────────────────────────
  | { kind: "display";            value: "block" | "inline" | "inline-block" | "none"; important?: boolean }
  | { kind: "visible";            value: boolean; important?: boolean }

  // ─── Extension ────────────────────────────────────────────────────
  | { kind: `ext:${string}`;      value: JsonValue; important?: boolean };
```

### 5.3 Supporting types

```typescript
export interface BoxSides<T> {
  readonly top: T;
  readonly right: T;
  readonly bottom: T;
  readonly left: T;
}

export interface TextDecoration {
  readonly line: "none" | "underline" | "overline" | "line-through";
  readonly style?: "solid" | "dashed" | "dotted" | "wavy";
  readonly color?: Color | TokenRef;
  readonly thickness?: Length;
}

export interface BorderSpec {
  readonly width: Length;
  readonly style: "none" | "solid" | "dashed" | "dotted" | "double";
  readonly color: Color | TokenRef;
  readonly sides?: ReadonlyArray<"top" | "right" | "bottom" | "left">;
}
```

`BoxSides<T>` is generic so the same shape covers padding (lengths)
and would cover borders if we expanded to per-side specs (we keep
borders single-sided in v0 via `sides`).

### 5.4 Why the closed list

Three reasons:

1. **Translators get exhaustive type-safety.** A CSS translator's
   `switch (prop.kind)` covers every case the type system knows
   about. Adding a new property kind triggers a compile error in
   every translator that doesn't yet handle it — exactly when we
   want to know.

2. **The set of properties is finite-ish.** Documents have a
   relatively small palette of visual primitives. CSS has ~400
   properties; documents need ~30. We cover the common 30 and
   leave the rest to extension.

3. **Extensions exist for the rest.** A plugin that needs
   `mask-image` or `clip-path` declares an `ext:mask:image`
   property; backends that understand it handle it, others ignore
   per §7.4.

### 5.5 The `important` flag

Discouraged. Use only when a theme system genuinely needs to
override base styles, e.g. a high-contrast accessibility theme
that overrides every color the base document specifies. Among
multiple important properties matching the same node, source
order from §4.9 still decides.

---

## 6. Contexts

Contexts are named gating conditions on rules. The consumer
activates contexts at translate time; rules with a `context` field
apply only when their context is active.

### 6.1 Standard contexts

```typescript
/** Activate `print` when emitting to print backends. */
export const CONTEXT_PRINT = "print";
/** Activate `screen` when emitting to screen backends. */
export const CONTEXT_SCREEN = "screen";
/** Activate `dark` for dark-mode rendering. */
export const CONTEXT_DARK = "dark";
/** Activate `narrow` for narrow viewports. */
export const CONTEXT_NARROW = "narrow";
/** Activate `wide` for wide viewports. */
export const CONTEXT_WIDE = "wide";
/** Activate `reduced-motion` when the user prefers reduced motion. */
export const CONTEXT_REDUCED_MOTION = "reduced-motion";
/** Activate `high-contrast` when the user prefers high contrast. */
export const CONTEXT_HIGH_CONTRAST = "high-contrast";
```

These are the kernel-blessed contexts. Custom contexts (plugin-
defined) follow `ext:<plugin>:<name>`.

### 6.2 Multiple contexts

A single rule has at most one `context`. To express "print AND
high-contrast," declare two rules with the same selector and
properties, one per context. This is intentionally explicit:
combining contexts opens design questions (does activating both
require AND or OR?) that we avoid by not having compound contexts.

### 6.3 Translator behavior

The translator receives an "active contexts" set as part of its
options:

```typescript
export interface TranslateOptions {
  readonly activeContexts: ReadonlyArray<string>;
  // ... backend-specific options
}
```

Rules whose `context` is in `activeContexts`, or who have no
context, are applied. Others are skipped.

For backends that produce media queries (CSS), contexts may also
be emitted as gates so the runtime browser activates them
appropriately. For backends that produce a single output (print
PDF), contexts must be resolved at translate time — the translator
picks the single active context set.

---

## 7. Themes

### 7.1 The theme reference

```typescript
export interface StyleDocument {
  // ...
  /** Named theme override.  Null if no theme is applied. */
  readonly theme: string | null;
}
```

A document declares it uses a named theme by setting the `theme`
field. The translator (or a theme-resolution stage) looks up the
theme and composes it with the base StyleDocument.

### 7.2 Theme composition

A theme is itself a partial `StyleDocument`:

```typescript
export interface Theme {
  readonly name: string;
  /** Token overrides.  Sparse — only specified tokens override the
   *  base.  Unspecified tokens stay at the base value. */
  readonly tokens?: Partial<TokenSet>;
  /** Additional rules.  Appended to the document's rules list,
   *  giving them later-in-order specificity (§4.9 rule 2). */
  readonly rules?: readonly StyleRule[];
}
```

Composition rule:

1. Start with base `StyleDocument`.
2. Deep-merge `theme.tokens` over `tokens` (per-named-token
   override; missing entries stay at base).
3. Append `theme.rules` to `rules`.

Themes are themselves Style IR values — there's no separate
"theme format." The same producer pipeline that emits
`StyleDocument`s emits `Theme`s.

### 7.3 Theme registry

The orchestrator (FM03) holds a theme registry keyed by name.
Theme-producing stages register themselves; theme-consuming
stages look up by name. A future first-party stage may scan a
`.forme/themes/` directory and auto-register.

In v0 the registry is in-memory and per-run; persistent themes
land with FM07 (CLI / dev server).

---

## 8. The `StyleDocument` Type

Composed of everything above:

```typescript
export interface StyleDocument {
  readonly kind: "StyleDocument";
  readonly tokens: TokenSet;
  readonly rules: readonly StyleRule[];
  /** Contexts THIS document declares.  Rules referencing contexts
   *  not in this list still translate, but the translator emits a
   *  warning — most likely a typo. */
  readonly contexts: readonly string[];
  /** Optional named theme.  Resolution happens in a separate stage. */
  readonly theme: string | null;
  /** Open extension slot for plugin-contributed top-level data. */
  readonly extensions?: ReadonlyRecord<string, JsonValue>;
}
```

This replaces the FM01 §2.3.5 stub. The three stub fields
(`tokens`, `rules`, `theme`) are preserved as field names; their
shapes are now precise.

---

## 9. Backend Translators

### 9.1 The translator interface

```typescript
export interface StyleTranslator<Out> {
  /** Backend identifier.  E.g. "css", "latex", "pdf-style". */
  readonly backend: string;
  /** Targeted Style IR major version. */
  readonly styleIrVersion: number;
  /** Translate a StyleDocument to the backend's native form. */
  translate(doc: StyleDocument, options: TranslateOptions): TranslateResult<Out>;
}

export interface TranslateResult<Out> {
  /** The translated output. */
  readonly output: Out;
  /** Rule IDs that were emitted to the output.  The renderer
   *  reads this to populate `RenderedPage.usedStyle` (FM01 §2.3.6). */
  readonly emittedRules: readonly StyleRuleId[];
  /** Warnings (unknown properties, unresolved tokens, etc.).
   *  Translators MAY emit warnings rather than failing for
   *  forward-compat. */
  readonly warnings: readonly StyleWarning[];
}

export interface StyleWarning {
  readonly code: string;
  readonly message: string;
  readonly ruleId?: StyleRuleId;
  readonly propertyKind?: string;
}
```

### 9.2 The CSS translator (`forme-style-to-css`)

The reference translator. Produces a single CSS string from a
`StyleDocument`. Optionally accepts a "scope" prefix for per-page
CSS slicing (FM06).

Property mapping:

| Style IR | CSS |
|---|---|
| `color` | `color: <value>` |
| `background` | `background-color: <value>` |
| `font-family` | `font-family: <stack>` |
| `font-size` | `font-size: <length>` |
| `font-weight` | `font-weight: <number>` |
| `leading` | `line-height: <number>` |
| `tracking` | `letter-spacing: <length>` |
| `space-before` | `margin-top: <length>` |
| `space-after` | `margin-bottom: <length>` |
| `indent` | `text-indent: <length>` |
| `padding` | `padding: <top> <right> <bottom> <left>` |
| `align` | `text-align: <value>` |
| `border` | `border: <width> <style> <color>` |
| `border-radius` | `border-radius: <length>` |
| `shadow` | `box-shadow: <offsetX> <offsetY> <blur> <spread> <color>` |
| `opacity` | `opacity: <number>` |
| `column-break: before` | `break-before: column` |
| `page-break: before` | `break-before: page` |
| `display` | `display: <value>` |
| `visible: false` | `visibility: hidden` |
| `ext:*` | (translator-specific extension hook) |

Selector mapping:

| Style IR | CSS |
|---|---|
| `node-type` | element selector (e.g. `p`, `blockquote`) |
| `node-type-level` heading | `h1`, `h2`, … `h6` |
| `custom-kind` | `[data-kind="<name>"]` |
| `tag` | `[data-tag~="<name>"]` |
| `id` | `#<id>` |
| `role` | `[role="<name>"]` |
| `nth` | `:nth-child(<n>)` / `:nth-last-child(<n>)` |
| `child-of` | `<parent> > <child>` |
| `descendant-of` | `<ancestor> <descendant>` |
| `adjacent` | `<previous> + <following>` |
| `and` | concatenate (no space) |
| `or` | comma-separate |
| `not` | `:not(<inner>)` |

Context mapping:

| Context | CSS |
|---|---|
| `print` | `@media print` |
| `screen` | `@media screen` |
| `dark` | `@media (prefers-color-scheme: dark)` |
| `narrow` | `@media (max-width: 40rem)` |
| `wide` | `@media (min-width: 80rem)` |
| `reduced-motion` | `@media (prefers-reduced-motion: reduce)` |
| `high-contrast` | `@media (prefers-contrast: more)` |
| `ext:<name>` | translator-specific extension hook |

Unknown property kinds are emitted as warnings; the CSS output
omits them.

### 9.3 The LaTeX translator (sketch)

`forme-style-to-latex` (deferred to v0.2). Maps style properties
to LaTeX style commands (`\usepackage`, `\newcommand`, etc.).
Many properties have no direct LaTeX equivalent (`shadow`,
`opacity`, `border-radius`); these warn-and-skip.

Selectors translate to context-aware document-class hooks. The
LaTeX translator's design is more involved than CSS because LaTeX
doesn't have a selector system at all — every style change is a
command. The translator emits a per-environment style preamble
plus per-element command wrappers.

### 9.4 The terminal translator (sketch)

`forme-style-to-terminal`. Maps:
- `color: <rgb>` → ANSI 24-bit color escape
- `background: <rgb>` → ANSI 24-bit background escape
- `font-weight >= 600` → ANSI bold (`\e[1m`)
- `font-style: italic` → ANSI italic (terminal-dependent)
- `text-decoration: { line: underline }` → ANSI underline

Most other properties drop silently.

### 9.5 The PDF translator (sketch)

`forme-style-to-pdf`. Produces PDF style dictionaries embedded in
the PDF stream. Most properties translate; layout-affecting ones
(`max-width`, page breaks) need to coordinate with the paint
backend (FM06).

### 9.6 The unknown-property rule

Every translator MUST:

1. Recognise the kernel's closed-list properties (§5.2) and emit
   either output or a documented "doesn't apply to this backend"
   skip.
2. Recognise `ext:*` properties whose namespace they understand;
   ignore others.
3. Emit a `StyleWarning` for every property they skip (allows
   linting).
4. Never throw on unknown properties. Throwing breaks the AOT
   compiler's "ship the minimum artifact" guarantee — a future
   property kind shouldn't fail today's builds.

---

## 10. Per-Page Collection (`usedStyle`)

`RenderedPage.usedStyle` (FM01 §2.3.6) is the per-page list of
`StyleRuleId`s the page actually used. This is the AOT compiler's
input for CSS slicing.

### 10.1 How a renderer populates it

A renderer walks the document tree, matches each node against the
StyleDocument's rules, and records each matching rule's id. The
output list is:

```typescript
readonly usedStyle: readonly StyleRuleId[];
```

Order is deterministic — sorted by source-order index of the rule
in `StyleDocument.rules`. This makes per-page CSS hashes stable
across renderer implementations.

### 10.2 Translator integration

The translator receives the `emittedRules` list as part of
`TranslateResult`. Combined with a renderer's per-page
`usedStyle`, the AOT compiler intersects:

```
per-page-css-rules = translator.emittedRules ∩ renderedPage.usedStyle
```

That intersection is what gets bundled per-page. Pages that use
no rules get zero CSS bytes (beyond a `<style></style>` shell);
pages that use many rules get exactly those rules.

### 10.3 Why this matters

Without per-page slicing, the entire site's CSS ships with every
page — typical SSGs do this and ship 50–200 KB of CSS per page
even when 95% is unused. With per-page slicing, the bytes
correlate with the page's actual visual complexity.

Combined with FM05's per-page island tracking, this is the
mechanism that delivers FM00's "ship the minimum artifact" thesis.

---

## 11. Plugin Extension

Plugins extend the Style IR through their FM02 manifest's
`contributes.styleExtensions` block (a new section this spec
adds; FM02's existing manifest schema gains it as a backward-
compatible extension):

```toml
[[contributes.styleExtensions]]
# Add a new property kind.
kind = "property"
name = "mask:image"
valueSchema = "./schemas/mask-image.json"

[[contributes.styleExtensions]]
kind = "selector"
name = "custom-data"
schema = "./schemas/custom-data-selector.json"

[[contributes.styleExtensions]]
kind = "context"
name = "print-spread"

[[contributes.styleExtensions]]
kind = "token-group"
namespace = "ext:my-plugin:palette"
```

Translators that understand a given extension act on it; others
ignore. The host doesn't enforce semantic correctness across
extensions — that's a translator-side contract.

---

## 12. Reproducible Builds

The Style IR contributes to FM03 §8 reproducible builds by being
purely declarative: no time, no random, no ambient I/O. A
`StyleDocument` produced from the same inputs is byte-identical
across runs.

Hash derivation: `computeRevisionId(canonicalJson(styleDocument))`.

Key ordering rules for determinism:
- `tokens.colors` etc. are records; iteration must be sorted by
  key (the canonical-JSON encoder handles this).
- `rules` array order is significant (per §4.9) and preserved.
- `contexts` array is treated as a set; sorted before hashing.
- `theme` is a name reference; resolution happens in a separate
  stage so the unresolved document is the hash input.

---

## 13. Package Layout

Three new TypeScript packages (plus one TBD for LaTeX, one for
PDF, one for terminal — sketched but not v0 deliverables):

### 13.1 `@coding-adventures/forme-style-ir`

Pure types and validators. Depends on `forme-types` only.

- `src/style-document.ts` — `StyleDocument`, `Theme`
- `src/tokens.ts` — `TokenSet`, `Color`, `Length`, `Shadow`, `TokenRef`
- `src/selectors.ts` — the `Selector` union, helpers
- `src/properties.ts` — `StyleProperty` union, value types
- `src/contexts.ts` — context constants
- `src/validate.ts` — `validateStyleDocument(doc)`: same one-pass-
  many-errors pattern as FM03 §2.4 / FM02 §3.3 validators
- `src/canonical.ts` — `canonicalStyleDocument(doc)`: byte-stable
  serialisation for hashing
- `src/style-error.ts` — `StyleError`, `StyleWarning`

### 13.2 `@coding-adventures/forme-style-to-css`

The reference translator. Depends on `forme-style-ir` only.

- `src/translate.ts` — `translateToCss(doc, options): TranslateResult<string>`
- `src/property-mappers.ts` — per-property CSS emitters
- `src/selector-mapper.ts` — selector → CSS string
- `src/context-mapper.ts` — context → @media query
- `src/token-resolver.ts` — TokenRef → concrete value
- `src/extensions.ts` — extension registration hooks

### 13.3 `@coding-adventures/forme-style-theme`

Theme management: registry, composition, resolution.

- `src/theme-registry.ts` — `createThemeRegistry()`
- `src/compose.ts` — `composeWithTheme(base, theme)`
- `src/resolve.ts` — `resolveTokenRefs(doc, tokens)`

### 13.4 Dependency graph

```
forme-types ◄── forme-style-ir ◄── forme-style-to-css
                              ◄── forme-style-theme
                              ◄── forme-style-to-latex (v0.2)
                              ◄── forme-style-to-pdf (v0.2)
                              ◄── forme-style-to-terminal (v0.2)
```

`forme-style-ir` is intentionally dependency-free beyond the
kernel. Translators depend on the IR but not on each other.

---

## 14. Testing Contract

### 14.1 `forme-style-ir`

- **Type-level tests** for the discriminated unions (selectors,
  properties): every variant has an exhaustive `switch`-style test
  ensuring TypeScript can narrow correctly.
- **Validation** covers every documented rejection reason (unknown
  property kind in `kind` field for non-`ext:` properties, malformed
  selectors, unresolved `TokenRef` in rules, etc.).
- **Canonical** is a fixed point: `parse(canonical(doc)) ===
  parse(canonical(parse(canonical(doc))))` for representative
  documents.

### 14.2 `forme-style-to-css`

- Every property kind in the closed list maps to its documented
  CSS output.
- Every selector form maps to its documented CSS form.
- Every context maps to its documented `@media` query.
- Unknown properties emit warnings, not errors; CSS output omits
  them.
- `usedStyle`-based slicing produces CSS containing exactly the
  requested rules.
- Reproducibility: `translate(doc, opts)` is a pure function;
  same inputs → identical output bytes.

### 14.3 `forme-style-theme`

- Compose-with-theme: token overrides apply; rules append.
- Token-ref resolution: known refs resolve; unknown refs return a
  `StyleError`.
- Theme registry: registration, lookup, replacement.

### 14.4 Coverage target

≥ 95% line and branch across the three packages.

### 14.5 Integration with FM01/FM03

The hello-world demo (already present per FM03's §15.2 reference)
adopts the Style IR by replacing `forme-render-static`'s inline
classless theme with a `StyleDocument` value produced by a new
`forme-theme-classless` stage. The renderer consumes that
document, the CSS translator emits CSS, and the AOT compiler
slices per-page. End-to-end the demo's output remains
byte-identical to today's; the new substrate just makes the
mechanism inspectable.

---

## 15. Open Questions

1. **Default theme registry source.** v0 uses an in-memory
   registry. Future: a `.forme/themes/` directory convention with
   one `.toml` or `.ts` file per theme.
2. **Theme inheritance.** A theme can override base tokens; can
   one theme inherit from another? Probably yes (deep-merge
   chain), but defer the spec until a real use case appears.
3. **Animation properties.** No `transition` / `animation` in v0.
   Add when the Interactivity IR (FM05) needs it.
4. **Logical properties.** CSS has `padding-inline-start` (RTL-
   aware). The Style IR uses physical box sides; a logical-
   property layer can be added as an `ext:i18n:*` extension.
5. **Gradient values.** `background: linear-gradient(...)` isn't
   expressible today. Add `Gradient` union member to the color/
   background slot when needed.
6. **Container queries.** CSS Container Queries (`@container`)
   are useful but raise selector-context questions. Defer; the
   `ext:` mechanism lets a plugin add them.
7. **Per-page style scoping.** The CSS translator can produce
   scope prefixes (`.page-abc123 .blockquote { ... }`) so per-
   page CSS doesn't collide with adjacent pages' rules. The
   exact scope-key derivation is a translator detail; v0 picks a
   hash of the page's route.
8. **CSS-in-JS interop.** Producing inline `style` attributes
   instead of a stylesheet (for email backends) is a separate
   translator (`forme-style-to-inline`). Deferred.
9. **Style hot-reload.** During `forme watch`, a style change
   should re-render without re-rendering content. The
   orchestrator's incremental rebuild (FM03 §6) handles this once
   it lands; until then watch mode re-runs the full pipeline.
10. **Validator strictness.** Should a `TokenRef` to a missing
    token be a hard error (validator) or a soft warning
    (translator)? v0: producer is responsible for emitting valid
    docs; validator catches obvious mistakes; translator catches
    runtime composition issues.

---

## 16. Success Criteria

FM04 is complete when:

1. **All three packages exist** under
   `code/packages/typescript/forme-style-*`, each with the repo
   standard files.
2. **Test coverage ≥ 95%** across the three.
3. **The reference Astro-style pipeline produces byte-identical
   output via the Style IR** as it does today via the inline
   theme. Migration is mechanical, not behavioural.
4. **Per-page CSS slicing works**: a fixture pipeline with two
   pages, each using a disjoint subset of rules, produces two CSS
   files with non-overlapping content.
5. **Theme composition works**: a base StyleDocument + a theme
   that overrides three colors produces output where those three
   colors are overridden and everything else is preserved.
6. **Documented graceful degradation**: a property kind unknown
   to a translator emits a warning, doesn't break the build.
7. **Style hash determinism**: two consecutive runs over identical
   StyleDocument values produce identical hashes.
8. **CSS-translator output validates** against a CSS linter (no
   syntax errors, no undefined custom properties).

---

## Appendix A — `StyleDocument` JSON Schema

(Informative — the full schema is too long for the body; this is
the top-level shape every implementation must accept.)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Forme StyleDocument",
  "type": "object",
  "required": ["kind", "tokens", "rules", "contexts", "theme"],
  "properties": {
    "kind": { "const": "StyleDocument" },
    "tokens": { "$ref": "#/$defs/TokenSet" },
    "rules": { "type": "array", "items": { "$ref": "#/$defs/StyleRule" } },
    "contexts": { "type": "array", "items": { "type": "string" } },
    "theme": { "type": ["string", "null"] },
    "extensions": { "type": "object" }
  }
}
```

Full definitions for `TokenSet`, `StyleRule`, `Selector`,
`StyleProperty` live in `code/packages/typescript/forme-style-ir/schemas/`
once that package lands.

---

## Appendix B — Glossary

- **Backend** — the consumer of Style IR (CSS, LaTeX, PDF, terminal).
- **Context** — a named gating condition (`print`, `dark`, `narrow`).
- **Property** — one of the closed-list typed `StyleProperty` values.
- **Rule** — a `(selector, properties, context?)` triple.
- **Selector** — a value describing which nodes a rule applies to.
- **Style IR** — the typed value tree this spec defines.
- **StyleDocument** — the top-level Style IR value.
- **StyleRuleId** — opaque branded id used for `usedStyle` tracking.
- **Theme** — a named overlay (partial StyleDocument).
- **Token** — a named design-system value.
- **TokenRef** — a reference to a token by dotted path.
- **Translator** — a package that maps StyleDocument to a backend's
  native form.
- **usedStyle** — per-page list of `StyleRuleId`s populating
  `RenderedPage.usedStyle`.

---

## Appendix C — Pointers to sibling specs

- **FM00** — Forme vision (Style IR sketched in §3.2)
- **FM01** — Kernel (`StyleDocument` stub in §2.3.5)
- **FM02** — Plugin host (manifest extensions for style)
- **FM03** — Orchestrator (carries `Document` containing
  `StyleDocument`)
- **FM05** — Interactivity IR (parallel to this spec)
- **FM06** — AOT compiler (consumes `usedStyle` for slicing)
- **FM07** — CLI and dev server (theme registry persistence)

## Appendix D — This is a living document

Like FM00/FM01/FM02/FM03, FM04 evolves as implementation lands.
Where running code disagrees with this spec, the code wins and the
spec is updated; the history of the tension is part of the
project's record.
