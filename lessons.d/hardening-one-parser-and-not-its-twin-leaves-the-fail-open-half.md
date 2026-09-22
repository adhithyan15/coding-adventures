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
