---
category: Compiler / VM / language pipeline
---

# A token pass that removes or inserts one token at a time is quadratic, so fold with a read cursor and a write cursor

**What went wrong.** `ruby-lexer`'s eight era-fusion passes (`..`, `0x1F`,
`1.5e-3`, `>>=`, `+=`, `->`, `2r`, `&.`) each folded a pair by overwriting
the left token and calling `tokens.remove(i + 1)`.  `Vec::remove` shifts the
whole tail, so a pass with `k` fusions over `n` tokens costs `O(n·k)`, and
a file that is one fusion per line is `O(n²)`: 80 000 lines of `a&.b`
(~400 KB) took ~153 s to lex (#18814).  Every unit test used a handful of
tokens, so nothing noticed.  #18798 had already hit the same trap with
`Vec::insert` for synthetic `;`s, and the #18814 fix itself first missed
a ninth case: `finalize_heredoc` removed each heredoc's tag name with one
`Vec::remove`, so `k` heredocs opened on one line were O(k²) (80 000
took 30 s in release).  The issue listed line numbers; the review found
the one it did not list.  Removing the tags in one sweep also exposed
#18820: that removal had never taken the tag's whitespace flag with it,
so the parallel flag array was off by one after every heredoc.

**Fix.** One engine, `fuse_adjacent`, compacts in place: a read cursor
`r` and a write cursor `w`; the last settled token `tokens[w - 1]` is the
left half; a match replaces it and skips the followers, a miss swaps
`tokens[r]` down to `w`; one `truncate` at the end.  Parallel arrays (the
whitespace flags) are compacted by the same moves.  Whether the old loop
stayed on the fused token or stepped past it is an explicit flag, so the
output is provably the old stream.  The old bodies stay in a
`#[cfg(test)]` module and a differential test compares every step on the
crate's literals, the Ruby corpora and seeded fragment soups.

**Do differently.**
- Never call `Vec::remove` / `Vec::insert` once per item, in a loop or in
  a per-event callback (a heredoc closing, a statement ending).  Collect
  the edits and rebuild once, or compact with two cursors.  Grep the whole
  crate for `.remove(` / `.insert(`, not just the sites an issue names.
- Remove from parallel arrays together, through one helper, so one cannot
  be forgotten.
- Test a token pass at scale: one construct repeated ~200 000 times with a
  generous time bound catches a quadratic pass in seconds, while the
  ten-token unit tests never will.
- When rewriting a pass for speed, keep the old body as a test oracle and
  compare per step, so a divergence names the pass that caused it.
