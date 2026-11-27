---
category: Compiler / VM / language pipeline
---

# PEG depth caps must fail the whole parse, not just the alternative

`GrammarParser`'s depth cap returned `None` (an ordinary no-match) and set a
sticky flag that `parse()` read only when the entry rule itself returned
`None`. With PEG ordered choice, one alternative hitting the cap and a later
lenient alternative (or an optional, repetition or negative lookahead)
matching meant `parse()` returned `Ok` with a different tree. An attacker who
controls nesting depth can steer the parse into the lenient reading.

Fix (LGE-PARSER-DEPTH): check the sticky flag unconditionally after the entry
rule and fail the whole parse with a distinct `DEPTH_EXCEEDED_MESSAGE` error
(`GrammarParseError::is_depth_exceeded`).

What to do: any resource cap inside a backtracking engine must be a sticky,
whole-parse failure, never "this alternative failed". Test it with a grammar
whose first alternative recurses and whose second is a lenient catch-all.
