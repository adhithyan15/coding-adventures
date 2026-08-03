# sir-collection-methods — executable collection methods across all languages

## Status

New. Design/spec PR (specs-first). The next cascade under the full-syntax mandate,
chosen after the keyword-params cascade landed and the Ruby-1.0 bump was **held**
to close more gaps first.

Goal: make **collection methods** — `.map` / `.each` / `.select` / `.reject` /
`.reduce`/`.inject` / `.push`/`<<` / `.pop` / `.length`/`.size` / `.keys` /
`.values` / `.include?` / `.first` / `.last` / … — **parse, lower, and execute
end-to-end in every source language and every backend**. These are ubiquitous in
real Ruby/Python/JS; closing this gap is the single biggest step toward "run real
programs."

This is the mandate's **backend-runtime-library arm**: where a target language has
no native form (Go, and the reflective/uniform-dispatch cases), the backend ships a
small runtime library that implements the operation; where a native form exists
(JS `.map`, Python `.append`), the backend may direct-lower.

## Current state (from the 2026-07-01 dispatch survey — what already works)

The narrow waist for a method call **already exists** and needs no new node:

```
BuiltinCall { name: "__method__", args: [receiver, StrLit(method_name), ...args], effects, span }
```
- Receiver at `args[0]`; method name is always a `StrLit` at `args[1]`; call args follow;
  an optional trailing block is a `MakeClosure` (RB1) / `block_pass` envelope.

What is **already implemented** (do not rebuild):
- **Ruby frontend** (`ruby-to-semantic-ir`): every `recv.meth(...)`/`recv.meth { }` already
  lowers to `__method__` dispatch (`lower.rs` `fold_one_dot_call`). ✅
- **Python + TypeScript backends**: already emit dispatch to their OOP runtime
  (`_sir_oop_call_method` / `__SirOop.callMethod`), and **`sir-runtime-oop`
  (Python + TS) already implements 50+ collection methods** across Array / Hash /
  String / Numeric / Symbol, including block-passing (`apply` over `Closure`) and
  `Symbol#to_proc` (`&:sym`). ✅ So **Ruby→Python and Ruby→TS collection code runs
  today.**
- **JavaScript backend**: accepts `__method__`; routes to the JS OOP runtime.
  (Verify end-to-end as part of KWc below.)

The **real gaps** this cascade closes:
1. **Python & JavaScript frontends** currently **defer** all method/attribute calls to a
   positioned error (only `console.log` is special-cased in JS). → they never produce
   collection-method dispatch.
2. **Go & Rust backends reject** any `__method__` dispatch (they don't accept the OOP
   feature gate). Go has **no** native method forms, so it needs a **new Go runtime
   library**; Rust needs a **new Rust runtime library** (or an iterator-based
   direct-lowering).
3. **No dedicated feature flag**: `__method__` is implicitly gated by OOP features
   (`Classes`/`InstanceVars`/…). Collection methods have nothing to do with classes,
   so they can't be enabled without dragging in class semantics.

## Design

### C1 — core: a `Feature::MethodDispatch` flag (`semantic-ir`)

Add `Feature::MethodDispatch`, observed by the validator whenever a
`BuiltinCall("__method__", …)` is present. This **decouples** method dispatch from
`Classes`/OOP so a backend can accept collection methods without accepting classes,
and lets each backend gate cleanly. (This is a core `Feature` enum addition — a
*variant*, so per the cross-PR lesson every downstream `Feature` match must gain an
arm and `cargo build --workspace` must pass before merge. The observation/gating
mirrors how `DefaultParams`/`KeywordParams` were wired.)

Backends that already dispatch (Python/TS/JS) add `MethodDispatch` to their accepted
set; Go/Rust add it once their runtime lands (C5/C6).

No change to the `__method__` convention itself.

### C2 — Python frontend production (`python-to-semantic-ir`)

Lower Python method/attribute calls (currently deferred) to `__method__` dispatch,
mirroring Ruby's `fold_one_dot_call`:
- `lst.append(x)` → `BuiltinCall("__method__", [lst, StrLit("append"), x])`
- `lst.map(...)`, `d.keys()`, `d.values()`, `s.upper()`, `lst.pop()`, membership, etc.
- Python has no trailing-block syntax; higher-order calls pass a lambda/closure as an
  ordinary arg (already `MakeClosure`). Comprehensions stay out of scope (separate
  cascade).
- Declare `Feature::MethodDispatch` when produced.

### C3 — JavaScript frontend production (`javascript-to-semantic-ir`)

Lower JS member-method calls (currently deferred except `console.log`) to
`__method__` dispatch: `arr.map(fn)`, `arr.push(x)`, `arr.filter(fn)`,
`obj.hasOwnProperty(k)`, `str.toUpperCase()`, `.length` (property → `length`
dispatch or `SeqLen`), etc. Keep `console.log` special-casing. Declare the feature.

### C4 — verify/complete the JavaScript backend dispatch

Confirm `semantic-ir-to-javascript` emits `__method__` through a JS OOP runtime and
executes the v0 catalog through `node`; fill any missing catalog coverage so JS
reaches parity with Python/TS. Add `MethodDispatch` to accepted features.

### C5 — Go backend runtime library + dispatch (`semantic-ir-to-go` + new Go runtime)

The largest piece. Go has no native method dispatch, so:
- Ship a **new Go runtime package** (`sir-runtime-oop` Go analogue, e.g.
  `code/packages/go/sir-runtime-oop`) providing `CallMethod(recv, name, args…, block)`
  with the same v0 catalog (Array/Hash/String/Numeric), block application over the Go
  closure representation, and `Symbol#to_proc`. Model it on the Python/TS runtime's
  catalog for behavioural parity (same method names, same semantics), execution-proofed
  against the reference backends.
- Emit `__method__` → `sirruntimeoop.CallMethod(recv, "name", args…)`; import the
  package conditionally (mirror the Python/TS runtime-import gating). Accept
  `Feature::MethodDispatch`.
- Where a Go-native form is trivially correct and cheaper (`len(x)` for `length`/`size`
  already via `SeqLen`), keep direct-lowering; route the rest through the runtime.

### C6 — Rust backend runtime library + dispatch (`semantic-ir-to-rust` + new Rust runtime)

Same shape as Go: a **new Rust runtime crate** implementing the catalog over the Rust
`Value` representation with block application, or an iterator-based direct-lowering
where it's clean. Emit dispatch to it; accept `MethodDispatch`. (Rust's owned/borrow
model makes a `Value`-boxed runtime the pragmatic v0 — match the runtime-lib approach
rather than fighting the type system with native iterators.)

### Native-vs-runtime policy (per the mandate)

- **Direct-lower** when the target has a faithful native form the backend can emit
  without the runtime (e.g. `length`/`size` → existing `SeqLen`; a later optimization
  pass could direct-lower JS `.map`/Python `.append`). Correctness first: v0 routes
  through the runtime uniformly except where a native node already exists.
- **Runtime library** everywhere else (all of Go; the reflective/block cases).

## Milestones (one PR per crate; core first)

| # | Crate | Content |
|---|-------|---------|
| C0 | `code/specs/` | **this spec** (design PR, surfaced first) |
| C1 | `semantic-ir` | `Feature::MethodDispatch` + validator observation + downstream `Feature`-match arms (workspace-build gate) |
| C2 | `python-to-semantic-ir` | lower `.method(...)`/attribute calls → `__method__` (execution-proof Python→Python) |
| C3 | `javascript-to-semantic-ir` | lower member-method calls → `__method__` (execution-proof JS→JS) |
| C4 | `semantic-ir-to-javascript` (+ JS runtime) | confirm/complete dispatch + catalog parity (node exec-proof) |
| C5 | `code/packages/go/sir-runtime-oop` + `semantic-ir-to-go` | Go runtime lib + dispatch emission (go-run exec-proof vs reference) |
| C6 | `sir-runtime-oop` (Rust) + `semantic-ir-to-rust` | Rust runtime lib + dispatch emission (rustc exec-proof vs reference) |

**Sequencing:** C1 (core `Feature` variant) merges **first**; the rest rebase on it.
C2/C3 (frontends) and C4 are independent, disjoint lanes that can run in parallel once
C1 lands. C5/C6 (new runtime packages + their backends) are the largest and mutually
disjoint. Every backend/frontend PR: tests via the linker override, clippy clean,
security-review gate, `cargo build --workspace` before pushing anything core-touching.

## Verification

- **C1:** validator observes `MethodDispatch` on any `__method__`; gating test; workspace build.
- **Frontends (C2/C3):** lowering-assertion tests (`.append`/`.map`/`.keys` → correct dispatch
  shape) + round-trip through `validate` + **execution-proof** (Python→Python via python3,
  JS→JS via node) of a small collection program (`[1,2,3].map/select/reduce`, `dict.keys`).
- **Backends (C4/C5/C6):** emitted-shape tests + **execution-proof** through the native toolchain
  (node/go run/rustc), diffing stdout against the Python/TS reference for the same SIR module
  (e.g. Ruby `[1,2,3].select { |x| x.even? }.map { |x| x*10 }` → `[20]` in every backend).
- **Runtime parity:** the Go/Rust runtime catalogs match the Python/TS method names + semantics
  (same v0 set); a shared golden-program suite runs through all five backends.

## Addendum — the C backend lane (2026-08-02/03, retroactive spec sync)

**Why this section exists:** the C lane below was built without a preceding spec
update — a process gap against this repo's "specs first" standard. This addendum
brings the spec in sync with what actually shipped, per the standing rule that a
diverged implementation must update the spec and call out what changed and why.

### What diverged from the original design

- **No `Feature::MethodDispatch` flag was added.** C1 above proposed a dedicated
  feature so a backend could accept collection methods without accepting classes.
  In practice, every `__method__` call's method-name argument is an `Expr::StrLit`,
  so the existing `Feature::Strings` gate already covers it for free — a backend
  that accepts `Strings` can already receive a module containing `__method__`
  calls, no new variant needed. The C backend instead gates structurally: an
  `is_builtin_method` allowlist (`semantic-ir-to-c/src/emit.rs`) rejects any
  `__method__` call whose method name it doesn't recognize, at emit time, before
  ever reaching the runtime — the same "reject cleanly, don't emit code that fails
  at runtime" posture C1 wanted, achieved without the extra `Feature` variant.
- **A C backend lane was never in the C1–C6 milestone table.** The table above
  only planned Python/JS frontends (C2/C3), the JS backend (C4), and new Go/Rust
  runtime *packages* (C5/C6). The C backend took a different shape entirely: no
  separate runtime package — the v0 catalog is dispatched directly inside
  `semantic-ir-to-c`'s existing runtime-C-source template (`_sir_builtin_method_v`
  in `runtime.rs`), delivered as a **slice cascade** (below) rather than one PR.

### C-backend slice cascade (actual delivery)

Each slice is Ruby-frontend → `__method__` dispatch (already existed, see "Current
state" above) → a batch of C runtime dispatch arms + `is_builtin_method` entries,
shipped as its own spec-adjacent PR (tests + changelog + README each time, per this
repo's standard workflow):

| Slice | Content | Status | PR |
|-------|---------|--------|-----|
| 1 | 0-arity String: `length`/`size`, `upcase`, `downcase`, `reverse`, `empty?`, `to_s` | ✅ merged | #9273 |
| 2 | 1-arg String queries: `include?`, `start_with?`, `end_with?`, `index` | ✅ merged | #9277 |
| 3 | 0-arg Array query/transform: `count`, `first`, `last`, `sort`, `min`, `max`, `sum`, `uniq`, `compact`, `flatten`, `to_a` | ✅ merged | #9617 |
| 4 | Array mutation + 1-arg query: `push`, `pop`, `shift`, `fetch`, `values_at`, `rotate`, `zip` | ✅ merged | #9650 |
| 5 | Array block methods: `each`, `map`, `select`, `reject`, `any?`, `all?`, `none?`, `sort_by`, `each_with_index`, `reduce`/`inject` | ✅ merged | #9628 |
| 6 | Hash non-block: `keys`, `values`, `to_h`, `dig`, `merge`, `delete`, `clear`, `invert` | ✅ merged | #9657 |
| 7 | Hash block: `each_key`, `each_value`, `group_by`, `partition` (+ `each`/`map`/`select`/`reject`/`sort_by`/`sum` widen to Hash) | ✅ merged | #9668 |
| — | Bug fix: `Array#sum` ignored a block argument | ✅ merged | #9673 |
| — | Bug fix: bracket-index (`a[i]`/`a[i] = v`) had no grammar rule at all — new `__method__("[]"/"[]=", …)` dispatch | ✅ merged | #9686 |
| 8 | Remaining String methods: `capitalize`, `strip`/`lstrip`/`rstrip`, `chomp`, `chars`, `bytes`, `split`, `replace`, `sub`, `gsub`, `to_i`, `to_f`, `to_sym`, `swapcase`, `tr`, `each_char` (block) — semantics matched against the Python/TS `sir-runtime-oop` reference catalog | ✅ merged | #9694 |
| 9 | Numeric methods: `abs`, `to_i`, `to_f`, `even?`, `odd?`, `zero?`, `positive?`, `negative?`, `pred`, `floor`, `ceil`, `round`, `divmod`, `fdiv`, `clamp`, `between?`, `gcd`, `digits` + block methods `times`/`upto`/`downto`/`step` | 🚧 this PR | — |
| 10 | Symbol + Object/Bool generic methods | planned | — |
| — | Cross-backend conformance corpus for the full collection-method catalog | planned | — |

**Explicitly out of scope for slice 8** (deferred, not silently dropped): Ruby's
char-set String methods (`count`/`delete`/`squeeze` taking a character-set string),
padding methods (`ljust`/`rjust`/`center`), and the `*`/`+` String operators.
`*`/`+` in particular are Ruby *binary operators*, not dot-calls — the Ruby
frontend has no lowering path for them at all yet (same pre-existing gap as `<<`
for `Array#push`; see the "Ruby frontend: add `<<` as a binary operator" backlog
item), so there is nothing for a C dispatch arm to receive regardless. The
char-set and padding methods are deferred to keep this slice reviewable at a
similar size to its predecessors; they are tracked as follow-up work.

**Explicitly out of scope for slice 9**: the multi-digit `round(ndigits)`
form (only the 0-arg `round` is implemented) — deferred for the same
"keep the slice reviewable" reason, tracked as follow-up work alongside
slice 8's deferrals.

## Out of scope (documented)

- Comprehensions (`[x*2 for x in xs]`, Ruby `map`-via-block is in scope but Python/Ruby
  comprehension *syntax* is a separate cascade).
- User-defined method resolution order / mixins (`include`/`extend`) — a later cascade;
  this cascade is built-in collection/String/Numeric methods on core types only.
- Lazy enumerators, `Enumerator`, `each_with_object`-style accumulators beyond the v0 catalog.
- The native-direct-lowering optimization pass (v0 routes through the runtime uniformly
  except where a native SIR node already exists).
