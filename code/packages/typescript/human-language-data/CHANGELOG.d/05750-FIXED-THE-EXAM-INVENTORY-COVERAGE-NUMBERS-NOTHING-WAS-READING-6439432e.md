### Fixed — the exam-inventory coverage numbers nothing was reading

- Twenty-five exam inventories sit in `core/`. Three of them — Hindi, Sanskrit and
  Telugu — were loaded by no test at all, so a tranche could raise their coverage,
  a retired atom could lower it, and the suite reported the same thing either way.
  Two more — Spanish A1 and German A2 — pinned the number but never checked that
  the atoms their probes name exist, and a misspelt atom resolves to "not
  introduced": the point reads uncovered, the number falls, and the finding is
  indistinguishable from a real gap.
- None of that was catchable, because nothing enumerated the inventories. Every
  assertion in the exam suites starts from a literal — `loadExamInventory("tamil",
  "A1")` — so an inventory nobody wrote a literal for is invisible to all of them.
- `exam-inventory-census.test.ts` enumerates `core/` with `readdirSync` and, for
  every file it finds, requires two things: that every atom its probes name is
  introduced somewhere in that track, and that some block in this suite measures
  its coverage and asserts on the result. Both are derived, so a twenty-sixth
  inventory is watched on the day it lands with no edit to the census.
- The enumeration is a directory listing and not a checked-in list on purpose. A
  declared list is the shape that failed here already: a check comparing a
  generated set against a declared set agreed perfectly while seventeen appendices
  were hand-authored, because both sides were empty. A list cannot notice what it
  forgot to declare.
- The census holds no number. `tests/corpus/README.md` forbids a shared suite from
  carrying a corpus-wide literal every language PR must update, and the corpus
  exam-point total was un-pinned for cause in HL-C310 — it read 529, 686, 793,
  792, 775, 774, 786 and 839 in one day, and two branches that both lowered it
  merged quietly because they agreed on a wrong value. Only a FLOOR on the number
  of inventory files is asserted, which fails on deletion and never on addition.
- The census also compares `listExamInventories` against the directory. That
  helper swallows a `LedgerParseError` and returns the file's neighbours, which is
  right for the planner and wrong for a census: a file that stops parsing would
  leave both the plan and the gate.
- Hindi, Sanskrit and Telugu A1 coverage is now pinned in each track's own corpus
  test, where a tranche on one track never touches another's line.
- Every assertion added here was falsified before being kept: a fabricated atom id
  in Sanskrit, Spanish and German A2 each fails the existence gate; nulling a
  covered Sanskrit probe drops the pinned count from 141 to 140; and a synthetic
  twenty-sixth inventory dropped into `core/` is demanded a pin immediately. The
  scanner that decides whether a pin exists is falsified against synthetic sources
  in the same file, including the false pass it exists to refuse — a block that
  loads one inventory as a derivation source and measures a different one.
- The security review of this change found the sharpest failure the census could
  have had. `pinnedBy` interpolates an inventory's declared `language` and `level`
  into a `RegExp`, so a file declaring `"language": ".*"` would compile to a
  pattern matching every other track's literal, borrow their pin, and pass the one
  gate it exists to fail; `(a+)+` would instead hang the worker, because the
  backtracking is synchronous and no test timeout can interrupt it. Both segments
  are now refused against the same allowlist `loader.ts` enforces, before they are
  a pattern, with a test for each. The census also reads inventories through
  `readLedgerFile` rather than a bare `readFileSync`, so it gets the symlink and
  dangerous-key guards every other reader in this package has.

