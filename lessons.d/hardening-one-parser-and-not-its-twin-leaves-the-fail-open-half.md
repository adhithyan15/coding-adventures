---
category: Cryptography & security review
---

# Hardening one parser and not its twin leaves the fail-open half behind

CodeQL flagged two high-severity ReDoS patterns in `parseMockPaper`
(`code/packages/typescript/human-language-data/src/mock-stem-coverage.ts`).
Both came from `\s*(.*)$` — the two halves both match a tab, so a failing
match tries every split point and the cost is quadratic in the whitespace run.

Removing `\s*` fixed the backtracking and created a new bug, because `.` cannot
match CR, LF, U+2028 or U+2029. With `\s*` gone, a chunk still containing one of
those made `$` unreachable and the match failed outright. The fix was to split
on every line terminator first — applied to `parseMockPaper`, and **not** to the
two answer-key readers, which stayed on `/\r?\n/`.

That asymmetry was the real defect, and the copy left un-hardened was the
dangerous one:

- In the **reporter**, a dropped row leaves `requiresByItem` empty, so words
  print as `unaccounted`. Noisy. Fail-closed.
- In the **gate** (`parseAnswerKey`), a dropped row is an item never scored. A
  key with lone-CR endings parses to zero rows, and the audit then reports
  `objectiveFailed: 0`, `reading: 0`, `listening: 0` — a clean bill of health
  for items it never read, which `--write` persists. Fail-open.

Three things to carry forward, and a corollary.

**Hardening is per behaviour, not per call site.** When a fix changes what a
parser treats as a line, a token, a path or a name, grep for every other place
that makes the same decision and change them together — or better, delete the
duplicate and share one definition. Here the sharing was blocked by an import
cycle (`mock-stem-coverage` already imports from `spanish-a1-mock-audit-cli`),
which is a reason to put the constant in a leaf module, not a reason to write it
out twice.

**Ask which direction each copy fails in, and fix that one first.** I hardened
the harmless copy and left the one whose silence reads as success. Sort the call
sites by failure direction before fixing any of them.

**A justification does not travel with a copy-pasted line.** Widening
`([^|]+)` to `([^|]*)` is genuinely inert in the reporter — an entry of `""`
matches no form. The same line in the gate makes an item fail on a requirement
nobody wrote. The comment saying "the widening is inert" was copied across with
the code and was false at the destination. Re-derive the claim against the new
caller, or do not carry the sentence.

**Corollary: a gate that read nothing must not report success.** Every silent
failure mode in a parser — a missed terminator, a changed heading, a rewritten
table — converges on the same empty result, and an empty result makes every
downstream number zero. `parseAnswerKey` now throws on zero rows.

**And the fix for a fail-closed bug can land fail-open.** Round two caught the
next move. The phantom requirement was `requires: [""]`, which made the item
FAIL on something nobody wrote; filtering the empty entry gave `requires: []`,
and `[].every(...)` is `true`, so the same unusable row began PASSING
unconditionally. Both scorings are wrong and only one of them is loud. When a
guard's input is unusable, "treat it as satisfied" and "treat it as violated"
are both answers to a question that should not have been asked — report it and
refuse, rather than picking a side.

**An all-or-nothing guard does not catch a partial drop.** The first version of
"a gate that read nothing must not report success" tested `rows.length === 0`,
which fires only when EVERY row is lost. One trailing space after a closing pipe
drops a single row, sails past that check, and leaves one item unscored — which
downstream looks exactly like an item that passed. Guard the per-unit invariant,
not just the aggregate: the parser now returns what it rejected, and the caller
refuses the file if anything was.
