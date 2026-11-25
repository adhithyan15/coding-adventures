## 0.2.2 - fuzz security review, round 2 (VER06i)

### Fixed
- F1/F2/F11: removed the `return` in the executor's `ensure`. A `break`, `return` or
  `throw` out of the target now propagates to the caller as Ruby defines (the handle is
  still retired), and `Thread#kill` / `Thread.exit` of a campaign thread are honoured.
- F4/F5: sentinel provenance is a Mutex-guarded `WeakMap` (bounded memory).
- F7/F8: the coverage tier is judged by the latest 16 executions; a result missing a
  loaded file or its branch data degrades the source; a run the source cannot prove is
  its own is never stopped.
- F9/F10: fallback loading skips already-loaded (including symlinked) and
  already-instrumented files and `load`s non-`.rb` names.
- Docs: Thread#raise stop exceptions, ObjectSpace trust level, POSIX separators.
