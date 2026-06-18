// SIMPLE-level tree-shaking of unused functions (CLOC12.159).
//
// The SIMPLE pipeline now ends with `treeshake`, which deletes top-level
// `function`/`class` declarations that nothing references. It is the
// function-shaped complement to remove-unused-vars (which skips
// functions). Removing an unused function declaration is always safe --
// declaring a function has no side effect.
//
//   - `function dead()`  -- never called, so treeshake removes it.
//   - `function live()`  -- called, so it survives. It is called TWICE
//     so the single-use inliner leaves it in place, keeping this fixture
//     focused on treeshake removing the unreferenced `dead`.
//
// Under WHITESPACE_ONLY both functions survive verbatim.
function dead() { return 1; }
function live() { return 2; }
log(live());
log(live());
