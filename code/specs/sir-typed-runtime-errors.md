# sir-typed-runtime-errors — typed exceptions from runtime operations

## Status

New. Design/spec PR (specs-first). Backlog #53 — the cross-backend follow-up from
the exception-hierarchy cascade's E3/E4 review. Toward the north star (any Ruby →
correct same-result output): a Ruby program that rescues a *runtime* error
(`ZeroDivisionError`, `IndexError`, `KeyError`, `NoMethodError`) must catch it, and
identically across all 5 backends.

## Current state (2026-07-01 survey)

Explicit `raise Foo` works and typed rescue matches a built-in class hierarchy
(all 5 backends). The ancestry ALREADY contains the relevant classes
(`ZeroDivisionError`, `IndexError`, `KeyError`, `NoMethodError`, `RangeError`,
`TypeError`, `ArgumentError`, `RuntimeError` → `StandardError` → `Exception`), and
every backend runtime has a typed-raise entry point (`raise_error(class,msg)` /
`_sir_new_error` / `panic_any(SirError)` / `throw new SirError`). So `rescue
ZeroDivisionError` WOULD match — **if the runtime raised one.** It doesn't:

| Op (Ruby) | Ruby behavior | Python | TS | JS | Go | Rust |
|---|---|---|---|---|---|---|
| `1 / 0` | raises `ZeroDivisionError` (int AND float) | native `ZeroDivisionError` (host, untyped) | returns `Infinity` (no error!) | `Infinity` | `panic("division by zero")` → caught as generic **StandardError** | `panic!` → **uncatchable** (re-raised) |
| `arr.fetch(100)` | raises `IndexError` | returns nil/default | nil | nil | nil | nil |
| `hash.fetch(k)` (miss) | raises `KeyError` | returns nil/default | nil | nil | nil | nil |
| `obj.undefined` | raises `NoMethodError` | returns nil (silent) | nil | user-obj: typed ✓ / native: JS `TypeError` | `panic(...)` → generic StandardError | `panic!`/nil |
| `arr[100]` (index op) | returns **nil** (NOT an error) | nil ✓ | nil ✓ | nil ✓ | nil ✓ | nil ✓ |

Two problems: (a) these ops don't raise the correct **typed** error (so `rescue
ZeroDivisionError` misses them or catches an over-broad `StandardError`); (b) the
**divergence** flagged in E3/E4 — a non-`SirError` host fault is classified as
`StandardError` in Python/TS/JS/Go (over-broad, swallows real bugs) but re-raised
uncatchable in Rust. Making the ops raise typed `SirError`s fixes BOTH: the
Ruby-semantic faults become typed + uniform, and the residual non-`SirError`
classification then only ever sees genuine codegen bugs (which should stay raw).

## Design — runtime-only, per backend (NO core-IR, NO frontend change)

Each faulting **runtime operation** raises the correct typed `SirError` via the
backend's existing raise entry point. Precise Ruby semantics (load-bearing — do
NOT over-raise):
- **Division `/` by zero** (`1/0`, `1.0/0`) → `ZeroDivisionError` ("divided by 0").
  Applies to the `/` builtin for int and float. (JS/TS must ADD the check — native
  `/` yields `Infinity`; Python must WRAP its native `ZeroDivisionError` into a
  `SirError` so it's rescue-matchable.)
- **`.fetch` on a sequence, OOB** → `IndexError`. **`.fetch` on a hash, missing key
  (no default block/arg)** → `KeyError`. The plain **index operators `arr[i]` /
  `hash[k]` still return `nil`** (Ruby does NOT raise for `[]`) — unchanged.
- **Unknown method** (`obj.undefined`, `nil.foo`) → `NoMethodError` with the
  Ruby-shaped message (`undefined method 'x' for <class>`). Replaces the current
  nil-floor / generic-StandardError panic / JS-native-TypeError.
- Keep everything Ruby returns nil for (missing hash `[]`, `Array#first` on empty,
  etc.) returning nil — do NOT raise.

The residual "non-`SirError` fault → ?" policy: once the Ruby-semantic ops raise
typed `SirError`s, a leftover raw host fault is (almost always) a genuine
translator/codegen bug. Recommended uniform policy: such faults are NOT caught by a
bare `rescue`/`rescue StandardError` (surface the bug) — i.e. move the other
backends toward Rust's re-raise stance for TRUE host faults, now that legitimate
runtime errors are properly typed. (Confirm per-backend feasibility during impl; if
a backend can't cleanly distinguish, document.)

## Milestones (one PR per backend runtime — disjoint, parallelizable)

| # | Crate(s) | Content |
|---|---|---|
| T0 | `code/specs/` | this spec |
| T1 | `semantic-ir-to-python` + `sir-runtime-core`/`-oop` (py) | `/`→ZeroDivisionError (wrap native), `.fetch`→IndexError/KeyError, unknown method→NoMethodError |
| T2 | `semantic-ir-to-typescript` + `sir-runtime-core`/`-oop` (ts) | same; add explicit `/0` check (native gives Infinity) |
| T3 | `semantic-ir-to-javascript` (inline runtime) | same; `/0` check; wrap native-method allowlist-reject as NoMethodError |
| T4 | `semantic-ir-to-go` (inline runtime) | `_sir_divide`/`_sir_seq`-fetch/`_sir_method_unknown` → `_sir_new_error(typed,…)` |
| T5 | `semantic-ir-to-rust` (inline runtime) | `divide`/seq-fetch/method-unknown → `panic_any(SirError{typed,…})` |

All disjoint per-backend runtimes → parallelizable after this spec. Each: unit
tests + **execution-proof** through the native toolchain that `begin; 1/0; rescue
ZeroDivisionError => e; …; end` catches (and matches the reference), plus
`arr.fetch(oob)`→IndexError, `h.fetch(miss)`→KeyError, `obj.undefined`→NoMethodError;
and that `arr[oob]`/`h[miss]` still return nil (no over-raise). Security-review gate
(the typed-raise is explicit-string, no reflection). Cross-backend parity: one
golden runtime-error suite through all 5.

## Out of scope

- `retry`, custom exception subclass bodies, `$!`/`$@` globals, `Exception#backtrace`.
- Full coverage of every Ruby core-method error condition — this cascade does the
  high-frequency four (ZeroDivision/Index/Key/NoMethod); more can be added
  incrementally as stdlib breadth grows.
