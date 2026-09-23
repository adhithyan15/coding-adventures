// ADVANCED-level renaming across a catch binding (CLOC19).
//
// `closurec` treats the catch parameter as RESERVED:
//
//   1. It is never itself renamed (catch params are not in the
//      local-rename set), and
//   2. No other local may be renamed to a name that collides with it
//      (the catch param joins the fresh-name avoid set).
//
// Here `process` and its locals get short names (`process` -> `c`,
// `value` -> `a`, `temp` -> `b`), the param use inside the try block is
// rewritten consistently (`value + 1` -> `a + 1`), and the catch
// binding `err` is preserved verbatim and never aliased to `a`/`b`/`c`.
// `report(err, temp)` becomes `report(err, b)` — proving the rewrite
// reaches into the catch body while leaving the catch param alone.
//
// ONLY (2) IS A SOUNDNESS REQUIREMENT. An earlier version of this
// comment called both of them "the crux of try/catch support" and said
// the renamer "must" treat the binding as reserved. Measured against
// the pinned oracle, upstream Closure renames catch parameters at both
// SIMPLE and ADVANCED, and it
// satisfies (2) by picking a fresh name that does not collide rather
// than by reserving the original. Renaming a catch binding is sound
// under the same assumptions the rest of the renamer already makes (no
// `eval` reading the name, no `with`); reserving it is one conservative
// way to satisfy (2), and it is the way we chose.
//
// So this fixture pins OUR rule, not upstream's. Under this file's own
// flags.txt upstream produces nothing at all — no externs are passed,
// so `compute` and `report` are undefined and it exits 2 with zero
// bytes of stdout. With externs added it compiles and inlines
// `process` away entirely, and what survives has the catch binding
// renamed. Either way it does not emit these bytes. The parity cost of (1)
// is tracked as CCR-022 (#15856), and closing it means changing this
// fixture's golden, not just adding a code path. Full probes are in
// code/specs/CLOC19-try-catch.md under "(1) is ours, not a law".
function process(value) {
  var temp = value + 1;
  try {
    compute(temp);
  } catch (err) {
    report(err, temp);
  }
  return temp;
}
process(7);
