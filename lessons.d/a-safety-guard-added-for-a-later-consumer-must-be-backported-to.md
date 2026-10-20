---
category: Security boundaries
---

# A safety guard added for a later consumer must be backported to the earlier ones in the same audit

**What went wrong.** PREP01's MacroNib slice found that the generic lexer
builds a whole file's tokens before the shared engine can apply its token
budget. It fixed that for MacroNib with per-file and aggregate pre-lex byte
caps, and the C dialect copied the fix. MacroOct, the first consumer, was
never revisited. An 8 MB dense MacroOct file still used about 2 GB and 12 s
before being rejected, and a 17 MB primary file over `bytes_per_file`
compiled, because the primary is lexed by the frontend rather than read
through the engine's filesystem. The spec even stated the rule, but only in
MacroNib's slice.

**The fix.** MacroOct charges every source byte (primary, includes and nested
include frames) against one per-unit `PrelexBudget` before lexing, and checks
its in-memory bundle before building `MemoryFs`. The dialect stays a stateless
public `Copy` value; a crate-private wrapper carries the budget.

**Next time.** When a slice discovers that an engine boundary leaks (here, a
guard that fires only after the expensive work), list every existing consumer
of that boundary in the same PR or audit, and either fix them or open a
tracked item for each. Write the contract into the engine-level section of the
spec, not the slice that discovered it, so the next consumer inherits it.
