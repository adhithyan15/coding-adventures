---
category: Testing & coverage
---

# Windows sanitizer validation needs a compatible runtime link surface

NN50's additional local Clang 22 UBSan link attempt failed before execution.
Its default static C++ runtime disagreed with the sanitizer's dynamic runtime.
Selecting `-fms-runtime-lib=dll` removed that mismatch, but the installed
sanitizer still referenced unresolved Windows support symbols including
`CommandLineToArgvW`, `SymLoadModuleEx` and `__coe_win::ContinueOnError`.

No UBSan executable ran. Do not call this clean UBSan validation or a parser
runtime failure; the separately passing ASan suite proves a different property.
Do not fake system headers or weaken package/CI warnings. A compatible complete
sanitizer runtime/link surface is needed for actual UBSan evidence.

The failure was recorded immediately outside the already-reviewed source tree
in `C:/Users/adhit/.codex/tmp/nn50-ubsan-validation-note.md`; this next tranche
reconciles that note without manufacturing a commit on a healthy queued PR.
