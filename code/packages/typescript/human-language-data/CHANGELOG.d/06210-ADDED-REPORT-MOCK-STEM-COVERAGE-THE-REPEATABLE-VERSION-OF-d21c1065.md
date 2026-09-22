### Added — report:mock-stem-coverage, the repeatable version of a hand pass that was wrong three times (HL-C421)

`requires` rows list the words of the **audio passage**, so an item could pass
the book-bounded audit while the option a candidate has to choose read
*"el nivel era demasiado alto"* with `nivel` taught nowhere. Repairing the rows
was done by hand, and the count went **6 → 15 → 17 → 32 → 48** — thirty-eight
corrections across three review rounds, *every one in the same direction*.

That is not a near miss. It is a bias, and a fourth hand pass inherits it. This
is the tool that replaces the pass.

```
npm run report:mock-stem-coverage              # A2, both mocks
npm run report:mock-stem-coverage -- --all     # every bucket, not just the two
```

#### Report-only, and that is not a stage on the way to a gate

A gate would have to decide, per word, whether a candidate can read it — the
judgement that has now been wrong three times. Automating it would make it fast
and unreviewable rather than right. What the tool does honestly is put the same
list in front of a reader every time, so the next pass starts where the last
one finished instead of from somebody's memory of it.

#### The design rule: nothing is silently dropped

Every hand pass failed by **clearing** words. A form with a plausible taught
relative goes to `derivable`, which is **printed**, beside the word it matched,
so the reader can reject the match in a second. Only four things remove a form
from the report, and each is a list somebody wrote down in
`core/spanish-mock-stem-vocabulary.json`: a taught form, a declared
function word or auxiliary, exam apparatus, or a proper noun.

The lists live in committed data rather than in the script, because the three
failed attempts were all computed with a hand-written list that was never
published beside the number — the lesson `HL-C419` already paid for.

#### Two bugs found while building it, both in the old method

- **The prefix rule never belonged.** The hand pass cleared `espacio` against
  the taught `esperar` on a 3-character prefix. This reporter strips declared
  **suffixes**: `espacio` reduces to `espaci`, `esperar` to `esper`, so they
  never meet. A test pins it, so adding a prefix rule later has to break
  something that says why there isn't one.
- **`minStemLength: 4` was too tight.** It silently broke `dice` from the
  taught `decir`, whose stem is three letters. Since every match is printed
  beside what it matched, a loose rule costs a glance and a tight one costs an
  exam item. It is 3.

Also fixed on first run: rows are written in **citation** forms and papers
carry **inflected** ones, so a string comparison re-flagged `alumnos` in the
very item whose row had just been repaired with `alumno`. Row entries now get
the same stem expansion the taught set does.

#### Changed — spanishTaughtForms is exported

The taught set was a local inside `buildSpanishA1MockAudit`. The reporter needs
the same one, and a reporter that rebuilds it from its own reading of the corpus
is measuring something other than the gate it reports on — the trap
`root-slug-splits.ts` records under *"one notion of a slug"*. Behaviour is
unchanged; all three mock audits regenerate byte-identically.

#### What it says today

After all 47 repairs, the reporter still finds **39 unaccounted forms in each
mock** — words in a stem or option with no taught relative, no declared
exemption, and no mention in the item's row. `ayuntamiento`, `cuota`,
`decisión`, `participar`, `presentarse`, `reparación`, `propuesta`,
`condiciones`, `universitario`, `huerto`.

So `objectiveFailed: 48` is a floor, and now demonstrably rather than as a
disclaimer. **Those 78 were deliberately not added by hand** — doing so would
be the fourth biased pass this tool exists to retire.

#### Fixed — three silent drops that security review found in the parsers

CodeQL flagged two high-severity ReDoS patterns in the new `parseMockPaper`,
and it was right where my own review round was not: I had cleared them as
"polynomial at worst, not exploitable" because this loop splits on newlines
first. That is true of *this* caller, and `parseMockPaper` is exported.

Fixing them removed the `\s*` that was doing the backtracking — and removing
`\s*` is what introduced the next three bugs, each in the direction this module
exists to prevent.

- **Line terminators, now shared.** `.` cannot match CR, U+2028 or U+2029, so
  once `\s*` was gone a chunk containing one made `$` unreachable and the match
  simply failed. `parseMockPaper` was hardened to split on all four; the two
  answer-key readers were left on `/\r?\n/`. The asymmetry was the bug, and the
  half left behind was the **fail-open** one: a key with lone-CR endings parses
  to zero rows, from which the audit reports `objectiveFailed: 0`, `reading: 0`,
  `listening: 0` — a clean bill of health for items it never read, which
  `--write` would persist. One `LINE_TERMINATORS` in `constants.ts` now, used by
  all three. It lives there rather than beside its first caller because
  `mock-stem-coverage` already imports from `spanish-a1-mock-audit-cli`, so
  exporting it from either would close an import cycle.
- **A gate that read nothing must not report success.** `parseAnswerKey` throws
  on zero rows. Every failure mode in that parser is silent by construction, and
  each of them lands on the same zero.
- **`+` → `*` is not inert in the gate.** The widening that killed the cubic row
  regex also made `| 7 | b |  |` match for the first time, capturing `""` —
  which `taught` never contains, so the item would have failed on a requirement
  nobody wrote. Inert in the reporter, as its comment says; the comment was
  copied to a file where it was not true.
- **The fix for that landed on the opposite error.** Filtering the empty entry
  left `requires: []`, and `[].every(...)` is `true` — so the unusable row
  stopped failing and started *passing unconditionally*, counting toward
  `reading`/`listening`. Round two of review caught it. Both scorings hide
  something, so the row is now neither: it is reported as `unscored` and the
  gate refuses the file.

All three Spanish mock audits regenerate byte-identically and the reporter's
output is unchanged — `unaccounted (39)` / `derivable (76)` and `(39)` / `(58)`,
as above. The row parse is now flat where it measured **cubic**: 1.1 s at a
2000-character line, 8.7 s at 4000, 67 s at 8000.

#### Changed — the answer-key parse returns what it REJECTED, not just what it read

`parseAnswerKeyRows(text)` is split out of `parseAnswerKey(path)` so it can be
tested the way `parseMockPaper` already was — every drop above was found by
*reading*, in two rounds of review, because the only way into that parser was a
run over the real corpus.

It returns `{ rows, unscored, malformed }` rather than an array, and that is the
substantive change. Every way this parse goes wrong is silent, each one removes
a row, and a removed row is an item the gate never scores — which downstream is
indistinguishable from an item that passed. So `parseAnswerKey` refuses a file
with any malformed row, any unscored item, no rows at all, or no rows under
Prueba 1 or Prueba 2.

The first version of that guard checked only "zero rows", which fires only when
*every* row is lost. One trailing space after a closing pipe drops exactly one
row and sails past it — an all-or-nothing check on the one failure shape that
was never the risk. All six Spanish answer keys pass all four guards unchanged.

Eleven tests pin the three terminators, the CRLF pair, the empty column, the
trailing comma, the rejected row, the header/separator non-match, the heading
scope, the empty input and the complexity class.

#### Changed — `LINE_TERMINATORS` is frozen

`constants.ts` is re-exported wholesale by `index.ts`, so the shared regex is
public, and a `LINE_TERMINATORS[Symbol.split] = …` from anywhere in the realm
makes all three parsers see one line and return nothing — including the gate,
whose nothing reads as a clean bill of health. Same-realm code could patch
`RegExp.prototype` just as easily, so this is depth rather than a boundary, but
it is free: `split` works on a frozen regex and the hijack becomes a
`TypeError`. `AUDIT_DIR` one module over is frozen for the same reason.

#### Fixed — the guards only caught the damage somebody had anticipated

Round three of review pointed the same class at the guards themselves. Every
check so far tested a *shape*, and a shape check is only as good as the list of
mutations whoever wrote it thought of. Measured against the real A1 key, these
each dropped a row into no bucket while every guard reported success:

| edit to one row | before |
|---|---|
| one trailing space | caught — the only one it was written for |
| one **leading** space (legal in GFM) | silently dropped |
| a bolded item number, already house style in `pre-a1/mock-1` | silently dropped |
| an item label with a suffix (`2a`) | silently dropped |
| `\v`, `\f` or U+0085 joining two rows | **item 3 scored against item 4's requirements** |

Two changes close all of them.

- **The heading now resets the paper.** `if (heading) paper = …` could only ever
  *set*, never clear — and that was live: `a2/mock-{1,2}-answer-key.md` write
  `## Pruebas 3 and 4`, **plural**, so `(\d)` cannot follow the `s`. `paper`
  stayed `2` through a section whose own text says it is "not read by the
  audit", and any numbered table added there would have been scored as
  listening. The A1 keys were safe only by luck — they spell `## Prueba 3`,
  which matches and resets.
- **The headings already declare their own size,** and now the parse is checked
  against it: `## Prueba 1 · Comprensión de lectura (25 items)`. Together with a
  contiguity check on the item numbers, this does not care *how* a row went
  missing. All ten mutations above are caught; all six real keys pass untouched.

The `malformed` detector is also shape-matched now rather than prefix-matched,
and it names the offending line instead of counting it — "1 table row rejected"
tells a maintainer that something is wrong and nothing about where, in a file of
160 lines.

`assertAnswerKeyParse` is exported and takes a parse rather than a path, for the
reason `parseAnswerKeyRows` is: every hole in these guards, across three review
rounds, was found by *reading*, because the only way in was a run over the real
corpus. Nineteen more tests.

All three mock audits still regenerate byte-identically and the reporter's
output is unchanged.

#### Fixed — the shape-independent guard was itself conditional

Round four found the round-four version of the same bug. The declared-count
check — the one described above as not depending on anticipating the damage —
read `count !== undefined && …`, so it **switched itself off** whenever a
heading stopped saying `(25 items)`, silently. And the contiguity fallback used
`findIndex((item, index) => index > 0 && …)`, which never examines index 0, so a
paper whose **first** row was lost read as perfectly contiguous.

The two holes line up. Either heading edit is ordinary rather than exotic —
`## Prueba 3 · Expresión e interacción escritas` in the same file already
carries no count, and `ítems` is the correct Spanish spelling — and after it,
losing the first row by any mechanism gave a clean bill of health for 24 of 25
scored items.

The count is mandatory now, and contiguity compares the **span** to the length,
which has no blind spot at either end and catches duplicates as well.

Two more, both latent and both fail-open:

- **A pipe inside the requirement cell truncated the list.** `([^|]*)` takes
  the last pipe-delimited run, so `` `a|b`, casa `` parsed as `["b`", "casa"]`
  and `casa \| ayuntamiento` as `["ayuntamiento"]` — a *shorter* requirement
  list, which is less for `taught` to miss. Rejected now, by two checks:
  cell-count arity against the first row of the same paper, and a flat refusal
  of escaped pipes. Neither catches the other's case — a single-check version
  compared the regex capture to the last split piece, which is worthless since
  both take the last run.
- **Bidi overrides survived into the error message.** This commit series was the
  first to print a line of *file content* through `reportableFilename`, and
  U+202E reverses the rendering of everything after it, so a crafted row could
  make the gate's own failure message read as though nothing had failed.
  `stripControlCharacters` now removes the invisible formatting ranges too.

Fourteen mutation shapes verified caught against the real A1 key; all six keys
pass untouched and the three audits stay byte-identical.

#### Fixed — six partial checks became one equality

Round five found that the span check was a **strict regression**: `[1, 1, 3]` has
span 3 and length 3, so a duplicated row filling the gap a dropped row left
passed clean — and the adjacency check it replaced had caught exactly that. The
comment claiming it had "no blind spot at either end" was also simply false;
`[2, …, 25]` has span 24 and length 24.

That is five rounds and five fail-open holes, each one in the fix for the last:

| check | hole |
|---|---|
| zero rows | missed a single dropped row |
| adjacency | blind at index 0 — a lost first row read as contiguous |
| span, replacing it | strictly weaker; missed duplicate-plus-gap |
| declared count | switched itself off when a heading lost `(25 items)` |

The sixth partial check would not have been the right one either. A set of
partial checks has holes *at the joins*, and the way out is to stop enumerating
what can go wrong and state what right looks like.

**The file already specifies itself completely.** Each scored heading declares
its size, and the papers number straight through — Prueba 1 is `1..n`, Prueba 2
is `n+1..n+m`. So the expected item set is derivable, and the check is one set
equality. A drop, a duplicate, a lost first or last row, a renumbering, a merge
are now all the same failure: *the items are not the items*. The message names
which items are missing, unexpected or duplicated.

Eighteen mutation shapes verified caught against the real A1 key. All six keys
pass untouched; the three audits stay byte-identical.

#### Fixed — the character strip no longer corrupts Perso-Arabic and Devanagari

The previous entry's widening stripped U+200B–U+200F wholesale, which includes
**ZWNJ and ZWJ** — orthographically required in Persian and Urdu (`می‌رود`
strips to `میرود`, a different word) and in Devanagari conjuncts. This repo has
`persian`, `urdu`, `hindi`, `marathi`, `marwadi` and `sanskrit` tracks. Every
caller is display-only and the corpus contains none of these characters today,
so nothing was corrupted — but a strip that renders two distinct lexemes
identically is the same class of harm as a bidi override, pointed the other way.
Both are excluded now.

Added in their place: U+2028 and U+2029, which are **line terminators** that
`JSON.stringify` does not escape — so the quoting layer this function's
docstring leans on to prevent a forged second log line did not stop the two
characters most able to forge one. Also U+061C, U+00AD, U+180E and U+FEFF.

#### Fixed — the equality needed a floor and a ceiling

Round six found that replacing the per-paper checks had deleted one the set
equality does not cover. A heading declaring `(0 items)` makes the expected set
**empty**, an empty expectation matches an empty parse, and the paper is skipped
entirely — its items never scored, `--write` persisting the result. Asymmetric,
too: `(0 items)` on Prueba 1 is caught incidentally because the anchor never
advances, so only the **last** paper failed open, which is the one an author
would stub out.

The same bound closes a second problem. `Array.from({ length: count })` does its
work before anything caps the message, so `(999999999 items)` aborted the
process outright — `FATAL ERROR … heap out of memory`, exit 134, uncatchable, no
gate message at all. `(99999999999 items)` was *safe*, because `ArrayCreate`
rejects a length at or above 2³² with a plain `RangeError`; the merely enormous
number was the dangerous one. Counts must now be 1–1000; the real keys declare
10 or 25.

Also: the set difference used `includes`/`indexOf`, quadratic in the row count
(10.1 s at n=100 000) on the passing path as well as the failing one — Sets now.
And `assertAnswerKeyParse` sanitises the name it is handed rather than trusting
the caller, on the same deep-import argument `spanishMockDir` already carries.

**One claim narrowed rather than defended.** The comment called the item-set
equality "a complete specification". It is not: transposing the requirement
cells of two items while leaving their numbers alone still passes, and item 3 is
then scored against item 4's requirements. No invariant over item *numbers* can
see that, and there is no second source to check the requirements against —
which is why `report:mock-stem-coverage` exists at all.
