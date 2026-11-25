---
category: Compiler / VM / language pipeline
---

# The GrammarLexer takes the first matching token, not the longest, and the GrammarParser skips NEWLINE tokens left after the entry rule, so a spec's tie rule and end-of-input need explicit grammar help

**What went wrong.** VER32 4.2 and 4.3 (PIF) assumed two engine behaviours that
the repository's engine does not have. PIF-001 found both by running probes.

1. *Longest match.* The spec's "tie rule" said that where two tokens match the
   same length the first listed wins, so the literal binder tokens `" inst"`
   etc. come before `FIDENT`. But `try_match_token_in_group` returns the FIRST
   definition that matches at all, regardless of length: `" instance"` lexed as
   `" inst"` plus the stray text `ance`, so a valid feature name was rejected.
2. *End of input.* `GrammarParser::parse` skips trailing NEWLINE tokens after
   the entry rule succeeds, so `end 0\n\n` parsed even though the spec says
   nothing may follow the `end` line.

Also found in the same pass: the `keywords:` section takes one word per line
(a line of several words is one keyword), and a rule that names the token
listed second of two overlapping tokens (`FSTRING` before `FLSTRING`) can
never match it, so the grammar must accept either.

**Fix.** For the tie rule, list a token before the literals that takes a binder
word followed by at least one more identifier character and alias it to
`FIDENT` (`FIDENT_LONG = / (default|inst|...)[a-z0-9_.]+/ -> FIDENT`); the bare
words still reach the literals. For end of input, end the entry rule with the
`EOF` token (`trailer = "end" FNAT NEWLINE EOF ;`).

**Next time.** Before relying on "longest match" or "must be at end of input",
write the failing input and run it. Token order in a `.tokens` file is
priority order, not a tie-break.
