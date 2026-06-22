# sir-method-dispatch — faithful built-in method dispatch for SIR backends

## Status

New. Closes the "Ruby method-dispatch boundary (terminal v0 cut-line)" recorded in
[`sir-runtime.md`](sir-runtime.md): today `sir-runtime-oop`'s
`call_method`/`callMethod` resolves only `is_a?`/`kind_of?`/`instance_of?`/`class`
plus a `define_method` table and **returns `nil` for every other method**, so
`arr.each`, `arr.map`, `"x".upcase`, `3.times`, `h.keys`, … all evaluate to nil
instead of running. This spec defines the bounded, type-dispatched built-in method
library that makes them execute, and the block-passing contract they rely on. It is
the foundation `&:sym` (Symbol#to_proc) builds on — **delivered in M2**, see the
*Symbol#to_proc (`&:sym`)* section below.

This is a runtime-package change only — no `semantic-ir` core or frontend change. The
frontend already emits `recv.meth(args…)` as `BuiltinCall("__method__", [recv,
StrLit("meth"), …args])`, with a trailing block lowered (per RB1) to a `MakeClosure`
appended to the args.

## Dispatch algorithm (extends current `call_method`/`callMethod`)

Resolution order for `call_method(recv, name, *args)`:

1. **Reflective built-ins** (unchanged): `is_a?`, `kind_of?`, `instance_of?`, `class`.
2. **User `define_method` table** (unchanged): a registered singleton/instance method.
3. **Built-in method catalog (new):** look up `(receiver_kind, name)` where
   `receiver_kind` is derived from `class_of(recv)` (`Array`, `Hash`, `String`,
   `Integer`, `Float`, `Symbol`, `NilClass`, `TrueClass`/`FalseClass`, else
   `Object`). A universal `Object` table (`to_s`, `inspect`, `nil?`, `==`, `!=`,
   `respond_to?`, `freeze`, `dup`, `tap`, `then`/`yield_self`) is consulted for any
   receiver after the type-specific table.
4. **Fallback:** return `nil` (unchanged floor) **only** for a method outside the
   catalog — and `respond_to?` reports this honestly.

The catalog floor stays `nil`-not-raise so existing programs never regress; the
difference is the catalog is now large rather than empty.

## Block-passing contract

A trailing `MakeClosure` arg is the block. The dispatcher detects a trailing
`Closure` (Python: `isinstance(arg, Closure)`; TS: `instanceof Closure`) and treats
it as the block, applying it via `sir-runtime-core`'s `apply(closure, [elem, …])`
(proc-lenient arity, already implemented). Methods that require a block when none is
given fall back to returning an `Enumerator`-less nil in v0 (documented), except the
common `each`→returns receiver, `map`/`select`→materialise eagerly.

`sir-runtime-oop` gains a dependency on `sir-runtime-core` (`apply`, `Closure`,
`to_display`, `eq`) — leaf-to-root in BUILD.

## v0 built-in catalog (bounded, faithful subset)

Chosen for coverage of everyday Ruby; the long tail is added incrementally and the
nil-floor + `respond_to?` keep the boundary honest.

- **Array** (`list`): `each`, `each_with_index`, `map`/`collect`, `select`/`filter`,
  `reject`, `reduce`/`inject`, `find`/`detect`, `flat_map`, `count`, `length`/`size`,
  `first`, `last`, `include?`, `index`, `push`/`<<`, `pop`, `shift`, `unshift`,
  `reverse`, `sort`, `min`, `max`, `sum`, `join`, `uniq`, `flatten`, `compact`,
  `to_a`, `empty?`, `any?`, `all?`, `none?`.
- **Hash** (`dict`): `each`/`each_pair`, `keys`, `values`, `has_key?`/`key?`/
  `include?`/`member?`, `has_value?`/`value?`, `fetch`, `merge`, `map`, `select`,
  `reject`, `length`/`size`, `empty?`, `to_a`, `dig`, `store`/`[]=`.
- **String**: `length`/`size`, `upcase`, `downcase`, `capitalize`, `reverse`,
  `strip`/`lstrip`/`rstrip`, `chomp`, `chars`, `bytes`, `split`, `include?`,
  `start_with?`, `end_with?`, `replace`, `sub`, `gsub` (literal), `index`, `to_i`,
  `to_f`, `to_sym`, `empty?`, `*`, `+`, `each_char`.
- **Integer/Float (Numeric)**: `times`, `upto`, `downto`, `step`, `abs`, `to_s`,
  `to_i`, `to_f`, `even?`, `odd?`, `zero?`, `positive?`, `negative?`, `succ`/`next`,
  `pred`, `floor`, `ceil`, `round`, `gcd`, `pow`/`**`, `digits`.
- **Symbol**: `to_s`, `to_proc` (the M2 hook), `to_sym`, `length`, `upcase`,
  `downcase`.
- **Object/universal**: `to_s`, `inspect`, `nil?`, `==`, `!=`, `respond_to?`,
  `freeze`, `frozen?`, `dup`/`clone`, `tap`, `then`/`yield_self`, `send`/`__send__`
  (routes back through `call_method` — also the M2 substrate).
- **nil/true/false**: `to_s`, `inspect`, `nil?`, `to_a` (nil→`[]`), `&`/`|` for bools.

Receiver-mutating methods (`push`, `<<`, `pop`, `store`, …) mutate in place and
return the Ruby-specified value. Block methods use SIR truthiness for predicate
results (`select`/`any?`/…). `to_s`/`inspect`/`join` route non-string parts through
`sir-runtime-core.to_display`.

## Backend wiring

For the **M1** catalog no emit change is required: `recv.meth(args)` already routes
to `call_method`/`callMethod`, and `sir-runtime-oop` is imported whenever a module
uses `__method__` (already the case).

**M2** adds one emit-layer rule (see next section): a `&:sym` / `&proc` block
argument that survives frontend normalization — i.e. a `block_pass` envelope sitting
in a `__method__` dispatch call's argument list — is recognized by `emit_arg` and the
`__method__` arm and rewritten to a `symToProc`/unwrapped-proc value.

## Symbol#to_proc (`&:sym`) — M2

Ruby's `&:sym` block argument converts a `Symbol` to a block via `Symbol#to_proc`:
the proc calls the named method on its first argument, forwarding the rest. So
`[1, 2, 3].map(&:to_s)` is `[1, 2, 3].map { |x| x.to_s }` and a two-arg block shape
(`include?`-style) binds the first argument as receiver and forwards the rest.

**Lowering (existing).** The Ruby→SIR frontend lowers a `&`-prefixed block argument
to `BuiltinCall("block_pass", [inner])`. The Q9f call-site normalization unwraps that
envelope **only at user-method `DirectCall` sites** (threading the proc as the
trailing block parameter). A block-pass to a **method-dispatch** call (`recv.meth(&…)`,
the `__method__` envelope) is *not* normalized — it reaches the backend intact.

**Emit (M2).** `emit_arg` and the `__method__` argument loop recognize a surviving
`block_pass` envelope:

| inner shape | emitted (py / ts) | meaning |
|---|---|---|
| `SymLit("m")` (`&:m`) | `_sir_oop_sym_to_proc(intern("m"))` / `__SirOop.symToProc(intern("m"))` | `Symbol#to_proc` |
| any other (`&proc`)   | the inner operand, unwrapped | the operand already *is* the proc/block |

**Runtime.** `sir-runtime-oop` gains `sym_to_proc(sym) -> Closure` (py) /
`symToProc(sym): Closure` (ts): a `sir-runtime-core` `Closure` whose body dispatches
`call_method(recv, name, *rest)`. Its arity is variadic, so `apply` forwards a block
method's arguments unadjusted (first = receiver, rest forwarded) — matching `&:sym`'s
Ruby arity. The body routes through `call_method`, so an out-of-catalog method bottoms
out at the `nil` floor rather than raising.

**v0 boundary.** Operator symbols (`&:+`, `&:<`) are emitted as **native** arithmetic
/ comparison, not routed through the dispatch catalog, so `inject(&:+)` is not yet
supported (the proc would resolve `+` to `nil`). Operator dispatch is tracked with the
later numeric-fidelity item.

## Out of scope (documented, honest floor)

- The full Ruby core/stdlib method surface — only the v0 catalog above runs; anything
  else still returns `nil` and `respond_to?` reports `false` for it.
- Methods needing an `Enumerator` when called block-less (e.g. bare `each`) — v0
  returns the receiver/nil rather than a lazy enumerator.
- Regex-semantic `sub`/`gsub` (pattern objects) — v0 does literal-string replacement;
  regex receivers route through `sir-runtime-regex` in a later phase.
- Keyword arguments to built-ins (`round(half:)`, `sort_by`-vs-`sort`) — positional
  v0.
- Coercion/`method_missing`/refinements.

## Verification

- `sir-runtime-oop` unit tests per catalog method (py: pytest + `mypy --strict` +
  ruff; ts: vitest + strict tsc), coverage ≥ 95%.
- Ruby→Python and Ruby→TS execution proofs: `[1,2,3].map { |x| x * 2 }` → `[2,4,6]`,
  `"hi".upcase` → `"HI"`, `{a: 1}.keys`, `3.times { |i| puts i }`, run through
  `python` / `node` with the local runtime packages on the path; skip gracefully if
  the interpreter is absent.
- `respond_to?` honesty test: a catalog method → `true`, an out-of-catalog method →
  `false`, and that out-of-catalog call still returns `nil` (no raise).
