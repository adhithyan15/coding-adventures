# sir-display-convention — source-language-faithful value display for SIR backends

## Status

New (design PR). **Surfaced from a 2026-07-06 survey.** Every SIR backend — Rust
(`format_d`), Go (`format`), JavaScript (`formatSeen`), Python (`to_display`),
TypeScript (`to_display`) — and both shared runtime libraries hard-code the
**Twig/Scheme (Lisp) display convention**. The Python reference states it plainly:

> *"`nil` prints as `nil`, booleans as `#t` / `#f`, a symbol as its bare name, a
> pair as a Lisp list."*

There is **no source-language conditional** anywhere in the display path. So a
translated **Ruby** program's `puts true` prints `#t`, not `true`. This breaks the
north star (*any Ruby → correct same-result output*) for every program that prints a
boolean — and, as the table below shows, for `nil`, symbols, strings, and collection
element rendering as well. The Python/TS *reference* backends are equally non-Ruby
here; the whole stack was built Twig-first and Ruby display-conformance was never done.

This is a **cross-cutting foundational change** (the display path feeds `puts`/`print`/
`p`/`inspect`, string interpolation, exception messages, and Array/Hash element
rendering) touching seven crates. It is written spec-first for review **before** any
implementation, per CLAUDE.md.

## The divergence (current Lisp convention vs. Ruby target)

For a program whose `source_language == "ruby"`, `puts x` (Ruby `Kernel#puts`, i.e.
`to_s` semantics) must render:

| value | current (Lisp) | Ruby `puts` (`to_s`) | Ruby `p` (`inspect`) |
|-------|----------------|----------------------|----------------------|
| `true` / `false` | `#t` / `#f` | `true` / `false` | `true` / `false` |
| `nil` | `nil` | *(empty line)* | `nil` |
| `:foo` (symbol) | `foo` | `foo` | `:foo` |
| `"hi"` (string) | `hi` | `hi` | `"hi"` (quoted, escaped) |
| `[1, 2]` | `[1, 2]` | `[1, 2]` | `[1, 2]` |
| `{"a" => 1}` (hash) | backend-native (`{'a': 1}` / `{a: 1}` / …) | `{"a" => 1}` | `{"a" => 1}` |
| `1.0` (float) | native | `1.0` | `1.0` |

Two orthogonal axes fall out:

1. **Convention** — driven by `source_language`: `Lisp` (Twig), `Ruby`, and later
   `Python` (`True`/`False`/`None`), `JavaScript` (`true`/`null`/`undefined`). Only
   `Lisp` and `Ruby` are in scope now (the two frontends that exist end-to-end).
2. **Form** — `to_s` (unquoted, `puts`) vs `inspect` (quoted strings, `:sym`, `nil`
   literal; `p` and array/hash *elements*, which Ruby always renders via `inspect`).
   The current single `to_display` conflates these; Ruby needs both.

## Design

**Reuse the existing `source_language` metadata — no new IR/manifest field.**
`Module.metadata.source_language: Option<String>` already exists
(`semantic-ir/src/metadata.rs:39`) and is already threaded to every backend emitter
(e.g. Go `emit.rs:93` reads it for a header comment). The `RUNTIME` string is emitted
wholesale by each backend (`out.push_str(RUNTIME)`), so the display function lives
inside that fixed text and cannot branch at emit time directly.

**Mechanism (minimal, per backend):**

1. The emitter derives a display-convention tag from `source_language`
   (`Some("ruby") → "ruby"`, else `"lisp"`) and emits **one constant line** ahead of
   `RUNTIME`, e.g.
   - Go: `const _sirDisplay = "ruby"`
   - JS: `const SIR_DISPLAY = "ruby";`
   - Rust: `const SIR_DISPLAY: &str = "ruby";`
   - Python/TS runtime-lib: a module-level `_SIR_DISPLAY` the emitter sets, or a
     parameter threaded into `to_display` (the lib is imported, not inlined — see
     *Runtime-library backends* below).
2. The display function gains a single branch on that constant. Structure it as
   `display(v)` (to_s form) and `inspect(v)` (repr form), each convention-aware;
   `puts`/`print` call `display`, `p`/`inspect` and all *collection elements* call
   `inspect`. This replaces the conflated `to_display`.
3. **Unknown / absent `source_language` ⇒ `lisp`** (today's behaviour) — so Twig
   output and every existing golden test are byte-for-byte unchanged. Ruby conformance
   is *additive*, gated behind `source_language == "ruby"`.

**Runtime-library backends (Python, TS).** `sir-runtime-core` is an imported package,
not inlined. Two options, decided at review: (a) `to_display`/`inspect` take a
convention argument the emitted code passes at each call site; (b) the emitted module
sets a package-level convention once at startup (`set_display_convention("ruby")`).
Recommendation: **(b)** — one call in the emitted prelude, no per-call-site churn, and
the backends that INLINE their runtime (Go/Rust/JS) use the emitted constant
equivalently.

**Security / dispatch.** Purely a formatting change; no method dispatch, no
`recv[name]`, no reflection — orthogonal to [[dynamic-dispatch-rce]]. The convention
tag is a fixed emitter-chosen string literal, never source-derived data.

## Rollout (sequenced to avoid contended crates)

Each backend is an independent crate → independent PR. Sequence so we never collide
with an in-flight stdlib PR in the same crate:

1. **Core/shared first:** `semantic-ir` — (no change needed; confirm `source_language`
   accessor ergonomics) + a conformance doc.
2. **Reference backends:** `sir-runtime-core` (Python), TS `sir-runtime-core` — add the
   convention-aware `display`/`inspect` + `set_display_convention`, wire the Python/TS
   emitters to emit the prelude call. (These are the reference; get them Ruby-correct
   first so the others have a target.)
3. **Inlined-runtime backends, one at a time as their crate frees up:** Rust, Go, JS —
   emit the `SIR_DISPLAY` constant + branch `display`/`inspect`.
4. Each PR: exec-proof (`puts true`→`true`, `p :foo`→`:foo`, `puts nil`→empty,
   `p "hi"`→`"hi"`, `puts [1, true, :x]`→`[1, true, :x]`) executed under the native
   toolchain, plus a Twig regression proving `#t`/`#f` is untouched when
   `source_language != "ruby"`. SECURITY-REVIEW gate on every push.

## Non-goals / open questions (for review)

- **In scope:** `Lisp` and `Ruby` conventions only (the two end-to-end frontends).
  `Python`/`JavaScript` source conventions are stubbed in the enum but deferred.
- **`inspect` string escaping** (Ruby `"a\nb".inspect == "\"a\\nb\""`) — full escape
  tables are a follow-up; first cut handles the quote wrapping + common escapes.
- **Hash `=>` rendering** for Ruby is the largest element-formatter change; may be its
  own PR after the scalar (`bool`/`nil`/`symbol`/`string`) cut lands.
- Does the team want the convention keyed strictly on `source_language`, or an explicit
  `display_convention` override (e.g. a Ruby program that wants Lisp output)? Spec
  assumes strictly `source_language`-derived; easy to widen later.
- This borders the maintainers' conformance frontier — **confirm ownership before
  implementing beyond the spec.**
