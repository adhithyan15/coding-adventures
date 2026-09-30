---
category: Repo policy / workflow reminders
---

# A vocabulary candidate screen must use the validator's script-data coverage, not a list of taught letters

**What went wrong.** The Sanskrit A2 candidate screen accepted a headword when
every glyph was a letter the book had taught. ङ is taught (SA-W65-letter-nga),
so eight tranche headwords spelled with the conjunct ङ्क / ङ्ग / ङ्ख passed the
screen: अङ्कः, शङ्खः, दिनाङ्कः and others. The full suite then failed in
`integration.test.ts` and `cli.test.ts`: "characters not yet in
devanagari.json: ङ". The validator checks a different set. Every headword glyph
must be a row in `data/scripts/devanagari.json`, which is marked complete and
has no source-backed row for ङ.

**The fix.** The words are spelled with the anusvāra (अंकः, शंखः, दिनांकः), as the
book already spells मंगलवासरः. The screen now also rejects any glyph that is not
covered by the script data, in NFD, the same way `uncoveredGlyphs` in
`src/validate.ts` checks it.

**Do differently.** Check candidates against the validator's rule, not against
a proxy for it. When a script file is `complete`, its rows are the definitive
glyph set, and a taught-letter list can be a superset of it.
