// SIMPLE-level optimization of a program containing `debugger;`.
//
// Before CLOC21, *any* program containing a `debugger` statement failed the
// typed-AST parse (DebuggerStatement was unrepresentable) and closurec
// silently fell back to WHITESPACE_ONLY — zero optimization. CLOC21 made the
// statement representable. CLOC24 then STRIPPED it at SIMPLE/ADVANCED, and
// **CCR-053 undid that**, because stripping was wrong:
//
//   * `1 + 2` is constant-folded to `3` (a scope-local, sound fold that runs
//     at SIMPLE).
//   * `debugger;` is KEPT. CLOC24 removed it, and both the pass and this
//     fixture claimed that matched upstream Closure. Measured against the
//     pinned oracle, it does not: upstream keeps `debugger` at SIMPLE and
//     ADVANCED wherever it is reachable, and drops it only as collateral when
//     the enclosing statement goes — after a `return`, or inside
//     `if (false) { … }`. Both of those still happen here, because they are
//     ordinary reachability and branch folding.
//
//     It is also not a free size win: `debugger` is observable behaviour, so
//     removing it changes what the program does under an attached debugger.
//   * `function log` is KEPT verbatim. SIMPLE is open-world: it never deletes
//     or inlines observable top-level names (another script sharing the page
//     could call `log`). The single-use inline + treeshake that would fold
//     `log(1)` into `report(1)` runs only at ADVANCED (closed-world).
//
// This fixture still does NOT match upstream byte-for-byte: upstream renames
// the parameter (`function log(a)`), and we do not. That is CCR-022, tracked
// separately, and it is the only remaining difference on this input.
function log(p) {
  report(p);
}
log(1);
var x = 1 + 2;
debugger;
use(x);
