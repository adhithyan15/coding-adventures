// ADVANCED-level renaming SOUNDNESS across a catch binding (CLOC19).
//
// The crux of try/catch support is that the catch parameter is a
// declared binding that the renamer must treat as RESERVED:
//
//   1. It must never itself be renamed (catch params are not in the
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
