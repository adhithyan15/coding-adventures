// SIMPLE keeps unreferenced top-level vars (open-world); `remove-unused-vars`
// is ADVANCED-only (CLOC12.158 originally added it to SIMPLE; that was an
// open-world miscompile and is now reverted).
//
// `remove-unused-vars` deletes unreferenced top-level `var/let/const`. It is a
// CLOSED-WORLD pass — safe only when the whole program is known — so it runs
// ONLY at ADVANCED. At SIMPLE the compiler is open-world: a top-level binding
// may be read by another script sharing the global object, so it is never
// deleted. What SIMPLE still does here is constant-fold each initializer:
//
//   - `var dead = 1 + 2;`  -- unreferenced, but KEPT (open-world). Its
//        initializer is still folded, so it emits as `dead=3`.
//   - `var live = 10;`     -- referenced by `log(live)` below; kept.
//   - `var impure = run();` -- unreferenced; kept (its call initializer is
//        preserved regardless — a call may have a side effect).
//
// Result: `var dead=3,live=10,impure=run();log(live);`. Under ADVANCED,
// `dead` (pure literal init) would be removed while `impure` (call init) is
// kept by the purity gate. Under WHITESPACE_ONLY every declaration survives
// verbatim AND `1 + 2` is left unfolded.
var dead = 1 + 2;
var live = 10;
var impure = run();
log(live);
