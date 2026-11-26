# 00080 - LGE-S3b: swift lane threat list

* **Added** `lanes/swift.json` for `swift/logic_gates` (family `managed-static`): 46 threats (all 29 registry
  classes plus 17 swift-specific ones), 46 rows, 43 `considered_not_included`. Every row is `open`: no lane
  evidence is `live` yet (VER15a 5.2).
* **Swift-specific threats:** reachable runtime trap (CWE-617), smart shifts (CWE-1335), caller-side narrowing
  (CWE-197, cannot close), ignored or swallowed `throws` including the `try?` / `Int?` (triState) ambiguity
  (CWE-391, cannot close), error identity and parameter attribution, error-message echo, Objective-C runtime and
  reflection (CWE-470), unsafe pointers and C interop (CWE-119), `-Ounchecked` consumer builds (CWE-758, cannot
  close), debug-only test runs, unenforced coverage, ARC retain cycles (CWE-401), public mutable state (CWE-500),
  SwiftPM supply chain (CWE-1357), gate values computed through a floating-point simulation (CWE-682),
  short-circuit validation and operator misuse.
* **Findings recorded in rows:** `andN`/`orN`/`shiftRegister` have no length cap on main (local branch
  `claude/swift-logic-gates-caps`, `LGE-SWIFT-CAPS`); `muxN` is recursive; two force unwraps
  (`Combinational.swift:202,353`); the LG02 wide-integer rows 2^31, 2^32, 2^32+1 and 2^53+1 are missing; n-ary
  gates name `input[i]` while the blocks name `inputs[i]`; block rejections are asserted only as "something
  throws"; `swift test` runs debug only with no coverage threshold; the transitive Foundation import through
  `swift/transistors`; the path dependency that makes the package unpublishable by URL.
* **Registry note (for the coordinator):** the memory-safety subset is named `gc_runtimes` but Swift uses ARC;
  the swift file uses it and says that whether an ARC runtime belongs to it is the analyzer rule's definition.
  The same applies to `pure_zero_allocation` for CWE-400, because most blocks return arrays.
* **Backlog rows added** (in `logic-gates-everywhere.md`): `LGE-R-swift` (merged #19955), `LGE-SWIFT-CAPS`,
  `LGE-SWIFT-HARDEN`, `LGE-PUB-SWIFT`. `LGE-A2`, `LGE-W1`, `LGE-S3v`, `LGE-S3b` and the template
  `LGE-L-swift` already exist.
