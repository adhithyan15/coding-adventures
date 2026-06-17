// SIMPLE-level dead-code elimination (CLOC12.157).
//
// The SIMPLE pipeline runs `dce` after `constant-fold` and
// `fold-control-flow`. This function exercises both of dce's jobs at
// once, and shows the passes composing inside one block:
//
//   - `keep();`                  -- live, before the return: retained.
//   - `if (4 > 5) { neverRuns(); }`
//        constant-fold turns `4 > 5` into `false`;
//        fold-control-flow turns `if (false) {...}` into an empty `;`;
//        dce then sweeps that empty statement out of the block.
//   - `return 1;`                -- the block terminator: retained.
//   - `alsoDead();`              -- after the return: dce drops it
//        (dead-after-terminator).
//
// `f()` is called so treeshake (the last SIMPLE pass) keeps the
// function; otherwise the whole unused declaration would be removed and
// the dce-inside-the-body effect wouldn't be observable.
//
// Under WHITESPACE_ONLY none of this happens -- every statement survives.
function f() {
  keep();
  if (4 > 5) { neverRuns(); }
  return 1;
  alsoDead();
}
f();
