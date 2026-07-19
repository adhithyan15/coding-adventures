// SIMPLE keeps unreferenced top-level functions (open-world); `treeshake` is
// ADVANCED-only (CLOC12.159 originally added it to SIMPLE; that was an
// open-world miscompile and is now reverted).
//
// `treeshake` deletes top-level `function`/`class` declarations that nothing
// references. Although *declaring* a function has no side effect, *deleting*
// an observable global is itself observable in an open-world setting: another
// script sharing the page could call `dead`. `treeshake` is therefore a
// CLOSED-WORLD pass and runs ONLY at ADVANCED (which demands `--externs` and
// treats un-exported globals as private). At SIMPLE nothing at top level is
// removed:
//
//   - `function dead()`  -- never called locally, but KEPT (open-world).
//   - `function live()`  -- referenced by `log(live())`, kept. `sink(live)`
//     passes it as a value without calling.
//
// Result: both functions survive —
// `function dead(){return 1};function live(){return 2};log(live());sink(live);`.
// Under ADVANCED, `dead` would be tree-shaken away. Under WHITESPACE_ONLY both
// functions also survive (it never runs treeshake).
function dead() { return 1; }
function live() { return 2; }
log(live());
sink(live);
