## Unreleased - CV2 S10f review fixes: match work budget, per-suite slim meter, pattern depth

- `match` usefulness check has a work budget (`MATCH_BUDGET`, 200,000 steps per question); over it the case is `ill-typed` (section 14 names no code; this is a reading). A 38-arm match over 18 `Result` columns is refused in well under a second.
- `runSuite` charges one suite its own work only (`checkWork(corpus, depth, record)`); control re-run accounting stays in the whole-run planner `checkWork(corpus, depth)`. wasm-leb128 suites run at slim again. The planner still refuses the whole corpus at slim (37,782,980 ordinals vs 32,000,000), pending an erratum on how section 14 counts re-runs.
- Pattern depth counts the outermost pattern as level 1, so a ninth level is refused, with code `depth`.
