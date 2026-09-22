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

**A shape check is only as good as the list of mutations you thought of.** Round
three turned the same class on the guards. The `malformed` detector was a prefix
test written against the one failure that had actually occurred — a trailing
space — and was blind to a leading space, a bolded item number, a suffixed item
label, and a `\v` joining two rows, the last of which scored one item against
another's requirements with every guard reporting success.

The fix was not a longer list of shapes. It was to find the invariant the DATA
already states: the heading says `(25 items)` and the numbers run consecutively,
so a parse that lost a row is short or has a hole, whatever removed it. When a
guard needs you to enumerate the ways input can be wrong, look for a count the
input declares about itself instead.

**A guard that switches itself off is not a guard.** The count check was written
`if (count !== undefined && items.length !== count)`, so it ran only when the
heading happened to state a count — and stated nothing when it did not. The
invariant "the data declares its own size" is only as strong as "the data is
required to declare its own size". If a check has a precondition, assert the
precondition.

**Check the span, not the neighbours.** `findIndex((item, index) => index > 0 &&
…)` never examines index 0, so a sequence missing its *first* element is
perfectly adjacent. Comparing `last - first + 1` to `length` has no blind spot at
either end, and catches duplicates for free.

**Two holes that are each survivable can compose into a silent failure.** The
conditional count check alone was recoverable, because contiguity still caught a
middle drop. The index-0 blind spot alone was recoverable, because the count
still caught a first-row drop. Together they passed a key with 24 of 25 items.
When reviewing a set of guards, ask what a pair of them misses, not only what
each one misses.

**After the third hole at a join, stop adding checks and write the
specification.** This file accumulated five partial checks — zero rows,
adjacency, span, a conditional count, a shape detector — and five rounds of
review each found a hole where two of them met. The span check was even a
strict regression on the one it replaced. Each fix was locally reasonable and
the sequence was not.

The way out was to notice that the data specifies itself: the heading declares
the item count and the papers number straight through, so the expected item set
is derivable and the check is one set equality. Eighteen distinct mutations
collapse into one failure — *the items are not the items*. When the third
round of review finds the third variant of the same class, the problem is the
approach, not the coverage.

**A strip is not automatically safer than no strip.** Widening
`stripControlCharacters` to the whole U+200B–U+200F block to stop a bidi
override also removed ZWNJ and ZWJ, which are orthographically required in
Persian, Urdu and Devanagari — all tracks this repo has. Rendering two distinct
lexemes identically is the same harm as a spoofed one, pointed the other way.
Enumerate what a sanitizer removes, and check it against the scripts the project
actually handles.
