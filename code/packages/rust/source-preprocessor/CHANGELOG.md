# Changelog

## 0.2.0 — PREP01 slice 2: macro expansion

Macros. Object-like and function-like, argument pre-expansion, and termination
by hide sets rather than by refusing to rescan.

### Added

- `hideset` — interned hide sets (Prosser's "blue paint"), per **token**
  rather than per macro. The obvious rule ("do not expand a macro inside its own
  expansion") fails on mutual recursion, where neither macro is ever expanding
  itself; the module's header works the case through. Interning is what keeps a
  500-token macro body costing one set node instead of five hundred, asserted
  directly by `hide_sets_are_shared_across_an_expansion_not_cloned_per_token`.
- `macros` — `MacroTable`, `MacroDef` and `expand`. Iterative, with an explicit
  work stack: in Rust a stack overflow is an *abort*, so a recursive expander
  would turn `Bounds::macro_depth` from a diagnostic into a process kill.
  Argument pre-expansion is the half that changes observable output rather than
  merely terminating, so it is tabled rather than smoke-tested — getting the
  order backwards still terminates and still compiles, just differently.
- `Directive::Define` gained `params: Option<Vec<String>>`. `None` is
  object-like, `Some(names)` is function-like, and the distinction is the
  **dialect's** to make because it is lexical: C decides it on whether the `(`
  touches the name, and a language with different directive syntax may not use
  that rule at all.
- The engine's `Define` arm now installs a definition instead of refusing one,
  and expands ordinary lines through `macros::expand` before emitting them. Two
  orderings there are load-bearing: expansion happens *after* the
  skipped-group check, so an expansion bomb is not reachable from inside
  `@if 0`; and the macro table is per translation unit, so a definition made in
  an `@include`d file outlives that file (which is what makes a "header" of
  definitions work) while nothing leaks into the next unit.
- Every expansion loop is charged against the bounds — depth, fuel per token
  copied and per rescan, argument-list grouping depth, and **every token
  produced including tokens that are then discarded**. Counting only survivors
  is the hole slice 1 shipped and had to fix.

### Not added, deliberately

`stringize` and `paste` remain `Dialect` hooks defaulting to "unsupported".
MacroOct declines both and still has a complete macro facility, which is the
point: the engine expands MacroOct's macros through the same code that will
expand C's, with two of C's operators simply absent.

### VM-068: controlling expressions are now macro-expanded

After `@define LED_PORT 1`, the line `@if LED_PORT == 1` now takes the true
branch. Until this slice it read `LED_PORT` as an *undefined* name, took the
`@else` branch, and compiled to the wrong thing — silently, with no
diagnostic. PREP01 §7's own worked example is exactly that shape, so the
spec's canonical illustration of the feature was broken.

Fixed in the engine rather than the dialect, because no dialect could fix it:
`eval_condition` receives a bare token slice with neither the macro table nor
any expansion applied. The grouping-depth pre-scan now runs twice — once on
the raw tokens, once after expansion, since a macro body can introduce
grouping the source text did not have.

Proved by a matched *pair* of matrix rows, because either alone is satisfied
by a broken implementation: an always-zero evaluator passes the undefined-name
row, and an always-truthy one passes the defined-name row.

## 0.1.0 — PREP01 slice 1: engine core

First release. Includes, conditional compilation, source mapping and resource
bounds, with per-language dialect plug-ins. Macro expansion arrives in slice 2
and is refused with a diagnostic until then rather than silently ignored.

### Added

- `Dialect` trait — the per-language half: directive classification, condition
  evaluation, lexing, and optional stringize/paste hooks that default to
  "unsupported" so the engine cannot assume they exist.
- `SourceFs` trait with `MemoryFs` (tests, fully in-memory) and `RootedFs`
  (production, confined to declared search roots).
- `SourceMap` / `Locus` — per-token provenance in a side table, with the
  expansion chain interned parent-id style so the map is `O(tokens +
  expansions)` rather than `O(tokens x depth)`.
- `Bounds` — finite defaults, tighten-only for dialects *and* embedding hosts,
  covering include depth and cycles, total inclusions, source bytes, per-file
  bytes, tokens produced, token spelling length, synthesised text, grouping and
  conditional nesting, diagnostics, and a `fuel` catch-all.
- One-pass `preprocess` in which inclusion and conditional selection interleave,
  traversed with explicit stacks rather than native recursion.

### Notes on two things that are easy to get wrong

- **Skipped groups are inert.** Inside `@if 0` the engine tracks nesting only:
  no condition evaluation, no include resolution, no expansion. Otherwise every
  bomb the bounds guard against is reachable from the one place a reviewer
  stops reading.
- **No native recursion.** In Rust a stack overflow is an abort, not a
  catchable panic, so a recursive traversal would convert a depth *diagnostic*
  into a process kill and defeat the bounds entirely.

### Fixed during development

- `RootedFs` rejected `/etc/passwd` on Unix but not on Windows, because
  `Path::is_absolute()` is `false` for a root-relative path there (Windows
  requires a drive prefix). The gate now checks `has_root()` as well. Caught by
  the crate's own test suite on the platform that matters most here.
