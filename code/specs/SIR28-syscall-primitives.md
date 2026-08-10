# SIR28 — A general syscall primitive family, starting with console output

## Status

New. Spec-only PR (specs-first). No code change — every frontend and backend
continues to do exactly what it does today. This spec defines a new,
not-yet-implemented `BuiltinCall` name (`__sys_write__`) and its gating
`Feature` (`Feature::ConsoleIO`); implementation (backends, then frontends)
lands in follow-up PRs per §5's ordering.

## Motivation

SIR's entire "touches the outside world" surface today is exactly three bare
`BuiltinCall(name, args)` names, none of which carry structured parameters:

- `"print"` / `"puts"` — implemented, inconsistently, by all 7 backends.
- `"gets"` (stdin read) — recognized by the Ruby frontend's effect table,
  implemented by zero backends.
- `"backtick"` (shell exec) — implemented by 2 of 7 backends (Python,
  TypeScript), gated by an ad-hoc body-walk (`uses_shell()` in
  `semantic-ir-to-python/src/emit.rs`) that scans for the literal name
  string — not by any `Feature`.

The `"print"`/`"puts"` case is a live bug, not a hypothetical: real Ruby's
`Kernel#print` never appends a newline; Python's `print()` and JS's
`console.log()` always do. Both lower to the identical bare
`BuiltinCall("print", [x])`, so a backend's newline policy is baked into
*which language it was written to match*, not carried in the IR. Confirmed by
direct inspection, the 7 backends already disagree with each other on the
same input: `semantic-ir-to-c`'s `_sir_print_v` and `semantic-ir-to-ruby`'s
`sir_print` never append a newline (matching real Ruby); `semantic-ir-to-
rust`'s `println!`, `-go`'s `fmt.Println`, `-python`'s `sir_print`
(docstring: "followed by a newline"), and `-javascript`'s `console.log`
always do. The same unchanged SIR module, run through two different backends,
produces different output — not because either implementation is "wrong" in
isolation, but because the bare name `"print"` was never a specification, it
was an assumption two different sets of authors made independently.

Fixing that one collision by adding a fourth ad-hoc parameter to bare
`"print"` would repeat the mistake at a different scale: the next "does this
call block?", "does this touch the filesystem?", "does this need a network
capability?" builtin would invent its own bespoke shape again, the way `print`
and `backtick` already have, independently, with no shared discipline. This
spec instead defines the general pattern once — a reserved, closed-contract
`BuiltinCall` name per operation, with policy carried as explicit typed
arguments rather than implied by the name — and applies it to the first,
most urgent case (console output). Files, process control, environment
access, and other "syscall-shaped" operations are reserved as named future
categories (§4) under the same discipline, not designed here.

## What this spec is and isn't

**Is:** the authoritative shape for `__sys_write__` (§2), the `Feature` that
gates it (§3), and the *category vocabulary and rollout discipline* that
every future syscall-shaped primitive in SIR should follow (§4, §5) — so the
next one (file I/O, `exit`, `ARGV`/`ENV`, `rand`, `Time.now`) has a template
to extend rather than another bare ad-hoc name to invent.

**Isn't:** a semantics change to any currently-shipping frontend or backend.
Nothing emits `__sys_write__` until its own follow-up PR (§5); until then
every frontend keeps emitting the bare `"print"`/`"puts"` it emits today,
unchanged.

**Isn't (yet):** an implementation of any category beyond console output.
File I/O, process control, environment/argv access, randomness, and time are
named in §4 as reserved vocabulary so a future spec doesn't have to invent
category boundaries from scratch, but none of them get a `BuiltinCall` shape,
a `Feature` variant, or backend/frontend code in this spec or its immediate
follow-ups.

## §2 — `__sys_write__`: the console-output primitive

```
BuiltinCall("__sys_write__", [
    StrLit(stream),          // "stdout" | "stderr"
    StrLit(terminator),      // "none" | "per_value" | "once"
    BoolLit(unpack_arrays),  // recursively flatten Seq args, one line per leaf
    ...values                // expressions to write, evaluated left-to-right
])
```

One reserved name, not a dispatcher-with-an-opcode-argument — matching the
OOP envelope's precedent (`sir-classes-oop.md`; `__new__`/`__method__`/
`__self__`/`__super__`/`__def_method__` are separate top-level names, not one
`__oop__` multiplexer). The dividing line that precedent sets: genuinely
different *operations* get separate reserved names; one operation varying
along independent *policy axes* gets one name with typed parameters. `print`,
`puts`, and JS's `console.log` are the same operation (format N values, write
bytes to a stream) varying along exactly two orthogonal axes — that's a
parameterization, not three different operations.

### §2.1 — Semantics

| Source form | `stream` | `terminator` | `unpack_arrays` |
|---|---|---|---|
| Ruby `print a, b` | `"stdout"` | `"none"` | `false` |
| Ruby `puts a, b` | `"stdout"` | `"per_value"` | `true` |
| Python `print(a, b)` | `"stdout"` | `"once"` | `false` |
| JS `console.log(a, b)` | `"stdout"` | `"once"` | `false` |

- **`terminator: "none"`** — write each value's display form back-to-back, no
  newline anywhere (real Ruby `print`).
- **`terminator: "per_value"`** — write each value's display form followed by
  a newline, once per value (real Ruby `puts`). Combined with
  `unpack_arrays: true`, a `Seq` argument is recursively flattened first, so
  nested arrays print one leaf element per line — real Ruby `puts`'s
  behavior on `puts [1, [2, 3]]`.
- **`terminator: "once"`** — join all values' display forms with a single
  space, write once, followed by one newline (Python `print`, JS
  `console.log`). No `sep` parameter in v0: every current frontend that
  would use this mode joins with exactly one space; add a `sep` parameter in
  a future revision only when a real frontend needs a different one (Python's
  `print(a, b, sep=",")` is not modeled by any frontend today).
- **`unpack_arrays`** is only consulted under `"per_value"` in v0 — it is a
  legal (if currently unused) parameter under `"none"`/`"once"` rather than a
  validation error, since a future frontend construct might legitimately
  combine them.
- Each value's *display form* uses the target backend's existing
  stringification convention (`sir-display-convention.md`,
  `sir-display-inspect-split.md`) — `__sys_write__` does not change how a
  value renders to text, only how the resulting strings are joined,
  terminated, and where they're written.

### §2.2 — Anti-injection invariant

`stream` and `terminator` are validated at emission time (§3.1) against their
closed enum of legal values and routed through an explicit `match` in every
backend — never through a target language's dynamic file-handle-by-name or
eval-shaped facility. This is the same discipline `sir-classes-oop.md`
states for method dispatch ("explicit table lookup — never reflection… per
the C3 RCE lesson") applied to stream selection: a backend's emit arm for
`__sys_write__` must reject (not silently default) any `stream`/`terminator`
value outside the two/three literal strings this section defines.

### §2.3 — Explicitly out of scope for `__sys_write__`

- **Ruby's `p`** (inspect-formatted print) needs a third, orthogonal axis
  (formatter: display vs. inspect) that no current backend or frontend has a
  reason to implement yet. Reserved future name: `__sys_write_inspect__`.
  Ruby's bare `"p"` builtin stays exactly as it is today (declared by the
  frontend's effect table, implemented by zero backends) — this is a
  pre-existing gap, not one this spec introduces or is obligated to close.
- **`gets`** (stdin read) is a genuinely different operation (read, not
  write) and is left alone for the same reason — declared by one frontend,
  implemented by none, unchanged by this spec. A future `__sys_read__`
  primitive is a natural sibling but is not designed here.

## §3 — `Feature::ConsoleIO`

A module containing any `__sys_write__` `BuiltinCall` declares
`Feature::ConsoleIO` (added to `manifest.rs`'s `Feature` enum under a new
`// ── SIR28 (syscall primitive family) ──` comment group, following the
existing per-SIR-number grouping convention). A backend that has not
implemented `__sys_write__` correctly rejects a module using it via the
existing O(1) `accepts_features()` capability check (SIR10) — no new
rejection mechanism needed.

### §3.1 — Validator responsibility

Unlike the OOP envelope (`__new__`/`__method__`/etc.), which today has zero
structural validation of its argument shape in `validator.rs` — malformed
args are only ever caught late, per-backend, by whatever a given `emit.rs`
match arm happens to assume — `__sys_write__` should be validated at
validate time: `validator.rs`'s `Expr::BuiltinCall` arm gains a check that
`args[0]`/`args[1]` are `StrLit`s whose value is one of the closed set in
§2.1, rejecting anything else (including a non-literal expression in either
position) before any backend ever sees it. This is a small, one-time
improvement over the existing precedent, not a requirement this spec places
on any other builtin.

## §4 — Future categories (reserved vocabulary, not implemented)

No `Feature` variant, `BuiltinCall` shape, or backend/frontend code exists
for any of these yet. This table exists so a future contributor extending
SIR's syscall surface has category names and a capability-vocabulary mapping
to start from, rather than inventing new ones the way `print`/`backtick`
each did independently. The category column aligns 1:1 with the *existing*,
already-shipped publish-time supply-chain vocabulary in
`code/specs/13-capability-security.md`
(`code/specs/schemas/required_capabilities.schema.json`'s category enum:
`fs`/`net`/`proc`/`env`/`ffi`/`time`/`stdin`/`stdout`) so the two systems —
this spec's compile-time `Feature` gate on a *module*, and that spec's
publish-time capability manifest on a *runtime package* — read as one
vocabulary rather than needing a translation table, without merging the two
mechanisms (they answer different questions: "can this module compile
against this backend" vs. "what does this published runtime package touch
on disk/network/process once installed").

| Future category | Capability-security category | Reserved `Feature` name | Example future op |
|---|---|---|---|
| Console input | `stdin` | `Feature::ConsoleIO` (shared with §3 — read and write are one category) | `__sys_read__` |
| File I/O | `fs` | `Feature::FileIO` | `__sys_file_open__`, `__sys_file_read__` |
| Process control | `proc` | `Feature::ProcessControl` | `__sys_exit__` |
| Environment / argv | `env` | `Feature::EnvAccess` | `__sys_env_get__`, `__sys_argv__` |
| Randomness | *(none existing)* | `Feature::RandomSource` | `__sys_random__` |
| Time / clock | `time` | `Feature::TimeAccess` | `__sys_time_now__` |

Each row becomes real only when its own spec slice defines a concrete
`BuiltinCall` args shape a validator can check and a frontend can actually
emit — following `Feature::DefaultParams`'s precedent (a real IR shape
existed — `Param.default: Option<Expr>` — before the `Feature` variant was
added) rather than pre-declaring a `Feature` enum variant with no code path
that could ever observe it, which `sir-classes-oop.md`'s "Out of scope
(documented, v0)" section already establishes as the wrong pattern for a
capability with no decided shape yet.

`EffectSet`/`Effect` (`effects.rs`, 5 variants) is not extended by this spec.
It is currently pure descriptive metadata — no backend or the validator
branches on its contents today (confirmed by inspection: zero non-test reads
of `.effects` outside node construction). `__sys_write__` reuses the existing
`Effect::MayPrint`/`Effect::MayThrow` tags; a future category may need a
finer-grained effect (`MayWriteFile`, `MayExit`, distinct from console
output) but that's a cheap, purely-additive bitset extension (3 of 8 bits
free) to make in that category's own spec, when a real consumer of the
distinction exists — not speculatively here.

## §5 — Rollout discipline

Per the ordering principle already established by `DefaultParams`/
`KeywordParams`'s actual rollout (core-IR shape lands first; each backend
gains acceptance independently, one PR per backend, purely additive since
nothing emits the new shape yet; each frontend migrates to emit it
independently, one PR per frontend or small batch, only after backends
already understand it):

1. This spec + `Feature::ConsoleIO` + the validator check (§3.1) land first,
   as core-IR PRs. No behavior change.
2. Each of the 7 backends gains a `__sys_write__` emit arm implementing §2.1,
   independently, as small additive PRs. Nothing emits the new op yet, so
   these carry zero regression risk to existing programs.
3. Each frontend that currently emits bare `"print"`/`"puts"` migrates to
   emit `__sys_write__` instead, independently, only after step 2 has landed
   for every backend that frontend's programs currently target.
4. Once every such frontend has migrated (confirmed by a grep sweep finding
   zero remaining bare `"print"`/`"puts"` emitters), the now-dead bare-name
   emit arms are removed from every backend in a final cleanup PR — not kept
   indefinitely as a second, unreferenced code path. Precedent:
   `__rescue_marker__`/`__ensure_marker__` were fully deleted, with zero
   remaining occurrences in the tree, once `Feature::Exceptions` superseded
   them — the same treatment applies here.

## §6 — Out of scope / future forks

- **Any category from §4** — file I/O, process control, env/argv access,
  randomness, time. Each is a separate future spec, not designed here.
- **A `sep` parameter for `terminator: "once"`** — no current frontend needs
  one; add only when a real frontend does (§2.1).
- **Extending `EffectSet`** with finer I/O-effect granularity — cheap,
  additive, deferred to whichever future category first has a real consumer
  for the distinction (§4).
- **Unifying with `code/specs/13-capability-security.md`'s publish-time
  mechanism** — the two answer different questions at different times (see
  §4's explanation); this spec aligns vocabulary with it but does not merge
  the mechanisms.
- **`SYSCALL00-host-syscall-library.md` / `SYSCALL01-syscall-condition-
  integration.md`** — an unrelated, pre-existing spec pair for a completely
  different subsystem (a separate bytecode-VM "LANG" chain: JVM/CLR/BEAM/WASM
  emitters), not wired to SIR's `Effect`/`Feature` machinery and not
  reused here beyond the general idea that "reserve category/name space for
  future growth" is worth doing — hence this spec's own §4, not a dependency
  on those documents. Naming this spec `SIR28-syscall-primitives.md` rather
  than a bare `SYSCALL*` filename is a deliberate choice to avoid the two
  being confused with each other.

## Cross-reference index

| Concern | Authoritative doc |
|---|---|
| `__sys_write__` shape, semantics, `Feature::ConsoleIO` | this spec, §2–§3 |
| Future syscall category vocabulary | this spec, §4 |
| Rollout/migration ordering discipline | this spec, §5 |
| Value display/stringification convention (unchanged by this spec) | `sir-display-convention.md`, `sir-display-inspect-split.md` |
| OOP envelope precedent for reserved-name dispatch | `sir-classes-oop.md`, `SIR25-language-agnostic-object-model.md` |
| Publish-time supply-chain capability vocabulary | `13-capability-security.md` |
| Effect tag lattice (unchanged by this spec) | `semantic-ir/src/effects.rs` |
