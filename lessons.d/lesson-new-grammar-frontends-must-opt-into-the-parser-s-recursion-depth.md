# Lesson: new grammar frontends must opt into the parser's recursion depth cap

**Context:** Adding `IF` to the COBOL runtime introduced the first *nestable*
construct. A security review found that deeply-nested `IF … IF … IF …` (one per
80-column card, so a statement flows across cards and the nest is real)
overflowed the native stack — an uncatchable `SIGSEGV`/abort that a
`Result`-returning entry point cannot report.

**Cause:** `parser::GrammarParser::new(...)` defaults `max_depth` to
`usize::MAX` — the recursion-depth guard is **opt-in**. `cobol-parser` built the
parser with `GrammarParser::new(...)` and never chained `.with_max_depth(...)`,
so every deep nest recursed once per level through `parse_rule` with no ceiling.
The shared parser already HAS the fix (`DEFAULT_MAX_RULE_DEPTH = 128`, below the
~192–224 overflow point on a 2 MB thread); the frontend just wasn't using it.

**Fix:** Every frontend that constructs a `GrammarParser` must chain
`.with_max_depth(DEFAULT_MAX_RULE_DEPTH)` on *all* construction paths (both the
panicking and the `try_` entry points). Deep nesting then returns a clean
"input nests deeper than the supported limit" parse error. This transitively
bounds any *runtime* tree-walk (e.g. a recursive `exec_stmt`) too, since the CST
can no longer be built deeper than the cap.

**Blind spot / grep before pushing a new frontend:**
`grep -rn "GrammarParser::new" code/packages/rust/<lang>-parser/src/` — every hit
that lacks a following `.with_max_depth(` is an unguarded native-stack DoS. This
is the same class as the closurec/json-parser deep-recursion DoS
(project_closurec_deep_recursion_dos): a nestable construct is the trigger, and
the mitigation belongs at the parser layer, not the runtime.
