# The conformance harness immediately found three latent cross-backend gaps

Expanding the `sir-conformance` corpus (6 → 11 Ruby programs) surfaced three
real bugs the moment broader features were exercised — exactly what the harness
is for. Each is a `case_eq`-style "works on some backends" gap, caught only
because the harness compares each backend to the **reference**, not to
backend-consensus (so backends *agreeing on a wrong answer* still fails).

1. **Frontend: array/hash index (`a[i]`, `h[k]`) is under-baked.**
   - `puts a[1]` (paren-less) **mis-parses** as `(puts a)[1]` — all four backends
     faithfully print the whole array then index, agreeing on the wrong answer.
   - `puts(a[1])` **fails to parse** outright ("Expected … got `[`").
   - `x = a[1]` parses but **fails SIR validation**: `var-ref scope=local
     references unknown name 'a'` — the index-base variable is mis-scoped during
     lowering. `x = a[1] + 0` (index inside arithmetic) *does* lower.
   These are Ruby-frontend parser/lowering bugs, not backend gaps.

2. **JavaScript backend: Ruby string method names aren't all renamed.** The JS
   backend translates Ruby method names to native JS **at emit time**
   (`upcase` → `toUpperCase`), unlike Python/Go/Rust which dispatch Ruby names in
   a runtime catalog. The rename table is missing the case pair, so `"x".upcase`
   raises `NoMethodError` on JS while the other three succeed. `.length` (spelled
   identically) works everywhere.

3. **JavaScript (and possibly Go/Rust) runtime: no `or`/`and` builtin.** A
   multi-value `when 1, 2, 3` lowers to `case_eq(...) or case_eq(...)` as a
   `BuiltinCall("or", …)`; the JS runtime's builtin table has no `or`, so it
   throws `unknown builtin: or` — the same shape as the `case_eq` gap. (Go/Rust
   were not reached before the JS failure, so they may share it.)

**Lesson:** each of these passed every per-backend unit test and only a
Ruby-source→all-backends run against a reference caught them. Keep growing the
corpus; every added feature is a chance to surface the next one. Programs that
hit an *unfixed* gap are kept OUT of the corpus (with a comment pointing here)
until the gap is fixed, so the suite stays green and the gaps stay visible.

Discovered: 2026-07-02, expanding the conformance corpus (PR after the harness landed).
