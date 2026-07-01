# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `rename_properties_test.rs`
    - upstream: `test/com/google/javascript/jscomp/RenamePropertiesTest.java`
    - tracked commit: see `UPSTREAM_SHA`

## Translation notes

CLOC12 port for the `rename-properties` pass (seventh port under CLOC12, after
constant-fold, dce, the emitter / source-map ports, remove-unused-vars, inline,
fold-control-flow, and rename-globals). Per CLOC12 §6, each upstream Java test
file maps to one Rust file in the matching pass crate's `tests/upstream/`
directory.

- Like the `rename-globals` port (and unlike the AST-builder ports), the pass
  exposes a source-string surface through public crate APIs, so this port drives
  the real `source → bridge → RenamePropertiesPass → emit` chain and asserts on
  the emitted string, the same surface upstream `RenamePropertiesTest` uses
  (`test(js, expected)`).

- **What our pass does today:** it renames **dotted, unquoted** property names
  (member accesses `o.prop` and object-literal keys `{prop: v}`) to the shortest
  fresh names `a`, `b`, `c`, … in first-appearance order, applied consistently to
  every occurrence of the same name. It leaves untouched: names accessed via a
  computed / quoted subscript anywhere in the program (`o["prop"]` — renaming the
  dotted form would desync from the string form), names already one character
  long, a curated set of built-in / DOM property names (`length`, `push`,
  `toString`, `innerHTML`, `addEventListener`, …), and any name in the externs
  do-not-rename set.

- **What upstream `RenameProperties` additionally does** — type-/heap-aware
  renaming that can distinguish same-named properties on unrelated objects,
  aggressive cross-module renaming, and the affinity/frequency-ordered
  short-name assignment that packs the hottest properties into the shortest
  names — our name-based pass does not do. Those are ported as
  `#[ignore = "blocked on gap-NNN"]` placeholders pinned to
  `code/specs/CLOC12-gaps.md` (gap-138 … gap-140).

Every active test that *disagrees* with our pass is a real closurec defect, not
a translation artifact — that is the entire point of the port.
