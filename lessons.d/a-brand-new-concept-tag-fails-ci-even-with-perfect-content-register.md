# A brand-new `concept_tag` fails CI even with perfect content — register it in `concepts/taxonomy.json` first

Adding lessons with a fresh `concept_tag` (e.g. `COURTESY-SORRY`, introducing a new
courtesy concept alongside existing `COURTESY-PLEASE`/`COURTESY-THANKS`) failed both
`human-language-data` integration tests — "every concept id is canonical or
namespaced" and "has zero validation errors" — with `concept_tag 'COURTESY-SORRY' is
neither canonical nor namespaced`. `COURTESY-PLEASE` needed no such step because it
was already a registered concept from an earlier PR; a genuinely new concept has no
such precedent.

`code/learning/human-languages/concepts/taxonomy.json` is the canonical concept
registry (`code/packages/typescript/human-language-data/src/loader.ts` loads it;
`validate.ts` rejects any `concept_tag` that is neither `in taxonomy.concepts` nor
matching the language-local `NAMESPACED_TAG` pattern like `TA-NUMBERS-1-5`). Before
writing the first lesson for a brand-new cross-language concept, add its entry to
`taxonomy.json` (`family`, `gloss`, `core`, optional `retires`/`notes` — copy the
shape of a neighboring entry, e.g. `COURTESY-PLEASE`) and validate it's still parseable
JSON, THEN write the lessons. Re-run both suites
(`code/packages/typescript/human-language-data` + the app suite) after any new
concept_tag lands — a green run before is not evidence the new tag is registered.
