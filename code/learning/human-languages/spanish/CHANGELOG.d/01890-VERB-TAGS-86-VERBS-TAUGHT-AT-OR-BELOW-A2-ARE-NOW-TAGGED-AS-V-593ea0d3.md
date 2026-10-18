## Verb tags — 86 verbs taught at or below A2 are now tagged as verbs

```
level gate, A2 verb vocabulary         91 -> 177 (target 120)
level gate, A2 blockers left           vocabulary 180
```

The level gate counts a headword as a verb when its concept tag names a verb
(`VERB-*` or `ES-VERB-*`). HL23 had already called Spanish's low count
"genuine tagging misses": many verbs were tagged by the chapter's topic rather
than by what the word is. Eighty-six infinitive headwords taught at or below
A2 carried tags such as `ES-ACT-CENAR`, `ES-ASK-ENCONTRAR`, `ES-PLAN-PREPARAR`
or `ES-VER`. Each is now `ES-VERB-<HEADWORD>` (for example `ES-VERB-CENAR`,
`ES-VERB-ENCONTRAR`, `ES-VERB-VER`).

- No lesson text, headword, gloss or chapter changed, and no lesson moved to a
  different spine node. Only the `concept_tag` line changed.
- None was moved onto a canonical `VERB-*` concept, so no other track gains a
  concept it has to answer for.
- No other file referred to the old tags.
- `regular` ("so-so", chapter 9) has an infinitive-like ending but is an
  adjective, so it keeps its tag.
- The generated book, narration and modality hashes for the 44 chapters
  involved were regenerated.

