// SIMPLE-level unused-variable removal (CLOC12.158).
//
// The SIMPLE pipeline now ends with `remove-unused-vars` (after
// constant-fold -> fold-control-flow -> dce -> inline). It deletes
// top-level bindings nothing references, when their initializer is
// side-effect-free. This fixture shows all three outcomes at once:
//
//   - `var dead = 1 + 2;`  -- unreferenced. constant-fold first turns
//        `1 + 2` into the literal `3`, then remove-unused-vars drops the
//        whole declaration (literal init => pure => safe to delete). This
//        proves the two passes compose.
//   - `var live = 10;`     -- referenced by `log(live)` below, so it
//        survives.
//   - `var impure = run();` -- unreferenced, BUT its initializer is a
//        call, which may have a side effect. The purity gate keeps it.
//
// Under WHITESPACE_ONLY every declaration survives verbatim.
var dead = 1 + 2;
var live = 10;
var impure = run();
log(live);
