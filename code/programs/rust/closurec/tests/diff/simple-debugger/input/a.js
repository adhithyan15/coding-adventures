// SIMPLE-level optimization of a program containing `debugger;` (CLOC21 made
// it representable; CLOC24 strips it).
//
// Before CLOC21, *any* program containing a `debugger` statement failed the
// typed-AST parse (DebuggerStatement was unrepresentable) and closurec
// silently fell back to WHITESPACE_ONLY — zero optimization. CLOC21 made the
// statement representable; CLOC24 now STRIPS it at SIMPLE/ADVANCED, matching
// the upstream Closure Compiler. This fixture is the end-to-end oracle:
//
//   * `1 + 2` is constant-folded to `3` (a scope-local, sound fold that runs
//     at SIMPLE).
//   * `debugger;` is REMOVED — a development-only breakpoint has no effect on
//     a shipped program, so the dce pass strips it at SIMPLE/ADVANCED.
//     (WHITESPACE_ONLY, which never runs that pass, would keep it.)
//   * `function log` is KEPT verbatim. SIMPLE is open-world: it never deletes
//     or inlines observable top-level names (another script sharing the page
//     could call `log`). The single-use inline + treeshake that would fold
//     `log(1)` into `report(1)` runs only at ADVANCED (closed-world). See the
//     sibling ADVANCED behavior: `report(1);var x=3;use(x);`.
function log(p) {
  report(p);
}
log(1);
var x = 1 + 2;
debugger;
use(x);
