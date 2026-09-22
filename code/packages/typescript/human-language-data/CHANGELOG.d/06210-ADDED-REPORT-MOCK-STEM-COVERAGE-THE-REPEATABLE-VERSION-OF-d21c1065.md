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
