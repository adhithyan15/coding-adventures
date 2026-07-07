# sir-display-inspect-split — Ruby `to_s` vs `inspect` display forms

## Status

New (design PR). Follow-up to the merged **`sir-display-convention`** spec, which
delivered the *boolean* increment (`#t`/`#f` → `true`/`false` under
`source_language == "ruby"`, now merged on all five backends). This spec covers
the **rest** of the Ruby display divergence — `nil`, symbols, strings, and
collection elements — which all hinge on the distinction the current runtimes do
not make: **`to_s` (what `puts`/`print`/string-interpolation render) vs
`inspect` (what `p` renders, and what every Array/Hash *element* renders as)**.

Written spec-first because it **restructures the display path** (one display
function → a `to_s`/`inspect` pair) across five crates and **changes existing
Ruby output** (e.g. `p :foo` must become `:foo`), so the tables and wiring want
review before implementation.

## The two forms (Ruby)

`puts x` / `print x` / `"#{x}"` use **`to_s`**; `p x` and every *element* of a
printed Array/Hash use **`inspect`**. They differ only for a few types:

| value | `to_s` (`puts`) | `inspect` (`p`, elements) |
|-------|-----------------|---------------------------|
| `nil` | `""` (empty) | `nil` |
| `true`/`false` | `true`/`false` | `true`/`false` |
| `:foo` (symbol) | `foo` | `:foo` |
| `"hi"` (string) | `hi` | `"hi"` (quoted, escaped) |
| `1` / `1.5` | `1` / `1.5` | `1` / `1.5` |
| `[1, :a, "b"]` | `[1, :a, "b"]` (elements via **inspect**) | `[1, :a, "b"]` |
| `{"a"=>1}` (hash) | `{"a"=>1}` (via **inspect**) | `{"a"=>1}` |

Two consequences the current single-function design gets wrong:
1. **`puts` of a collection renders its elements with `inspect`** — so
   `puts ["a"]` prints `["a"]` (quoted), even though `puts "a"` prints `a`.
2. **`p`/inspect quotes strings and `:`-prefixes symbols**, and `p nil` is
   `nil` while `puts nil` is an empty line.

The Lisp/Twig default keeps its existing single form (Scheme `display`), so all
current non-Ruby goldens stay byte-for-byte unchanged.

## Current state per backend (survey)

- **Python (`sir-runtime-oop`)** and **TypeScript (`sir-runtime-oop`)** already
  ship complete, cycle-safe `_ruby_to_s`/`_ruby_inspect` (Py) and
  `rubyToS`/`rubyInspect` (TS) — but they are used **only in error messages**,
  not in the `puts`/`print`/`p` display path (which still routes through the
  Lisp `to_display`). Work here is mostly **wiring**.
- **Rust / Go / JS** have a single `format_d` / `_sir_format` / `formatSeen`
  and **no** Ruby inspect/to_s helper. Work here is **build + wire**.
- `p` may not yet be lowered by the frontends as a distinct builtin from
  `puts`/`print` — confirm the SIR surface (a `p`/`inspect` builtin or method)
  during implementation; if absent, add it (small frontend/emitter task) so the
  inspect form has a call site beyond collection elements.

## Design

**Two convention-aware entry points** in each runtime:

- `sir_to_s(v)` — the `puts`/`print`/interpolation form.
- `sir_inspect(v)` — the `p` form; **also used for every element** rendered
  inside `sir_to_s`/`sir_inspect` of an Array/Hash.

Both branch on the already-plumbed display convention (the `SIR_DISPLAY_RUBY`
constant / `set_display_convention` from `sir-display-convention`). Under
`lisp`, both fall back to today's single form (no behavioural change). Under
`ruby`, they implement the table above. Collection rendering always recurses
through `sir_inspect` for elements (both forms), matching Ruby.

**Routing:**
- `puts`/`print` → `sir_to_s`.
- `p`/`inspect` → `sir_inspect`.
- Array/Hash element formatting inside either → `sir_inspect`.
- String interpolation → `sir_to_s`.
- Exception messages already use the Ruby inspect helper where they should;
  leave as-is.

**Cycle/depth safety** (already present in the Py/TS helpers) is mandatory in
the Go/Rust/JS builds too — a self-referential array renders `[...]` and depth
is capped, upholding the never-panic / never-hang floor (mirrors the existing
`format_d` cycle guard).

## Rollout (each an independent, exec-proofed PR)

1. **Spec** (this PR).
2. **Python** + **TS**: wire the existing `_ruby_to_s`/`_ruby_inspect` into
   `puts`/`print`/`p`/element-rendering under the `ruby` convention (small,
   low-risk — helpers already tested).
3. **Rust**, **Go**, **JS**: add convention-aware `sir_to_s`/`sir_inspect`
   (cycle-safe) and route the display builtins + element rendering through them.
4. Each PR: exec-proof under the native toolchain — `puts :a`→`a`, `p :a`→`:a`,
   `puts nil`→empty, `p nil`→`nil`, `p "x"`→`"x"`, `puts ["a", :b]`→`["a", :b]`
   — plus a Twig regression proving the Lisp default is unchanged. SECURITY
   gate every push.

## Non-goals / open questions (for review)

- **`p` return value** (`p x` returns `x`; `p a, b` returns `[a, b]`) — include
  or defer? Spec assumes display-only first; return-value semantics can follow.
- **Full string-escape table** for `inspect` (`"\n"`, `"\t"`, `"\""`, non-ASCII
  `\uXXXX`) — first cut handles quote-wrap + the common `\n`/`\t`/`\\`/`\"`
  escapes; exhaustive escaping is a follow-up.
- **Float formatting** (`1.0` vs `1`) is already native-correct; not in scope.
- Confirm the frontend lowers `p`/`inspect` distinctly; add the builtin if not.
- This borders the maintainers' conformance surface — confirm ownership before
  the multi-backend implementation.
