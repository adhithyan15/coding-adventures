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
  copied to a file where it was not true. The gate now filters empty entries, so
  a trailing comma in a hand-written row is dropped too.

All three Spanish mock audits regenerate byte-identically and the reporter's
output is unchanged — `unaccounted (39)` / `derivable (76)` and `(39)` / `(58)`,
as above. The row parse is now flat where it measured **cubic**: 1.1 s at a
2000-character line, 8.7 s at 4000, 67 s at 8000.

#### Changed — the answer-key parse is testable on strings

`parseAnswerKeyRows(text)` is split out of `parseAnswerKey(path)`, so it can be
tested the way `parseMockPaper` already was. Both of the silent drops above were
found by *reading*, not by a failing test, because the only way into that parser
was a run over the real corpus. Eight tests now pin the terminators, the CRLF
pair, the empty column, the trailing comma, the heading scope and the
complexity class.
