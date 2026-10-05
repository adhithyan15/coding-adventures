### Added — Gujarati vowel signs compose after their consonant, so sign lessons and words with signs print filmstrips

- **A second written-order table.** `WRITTEN_SIGN_SIDES` gains a `gujarati`
  table with eleven rows, all "after": ા િ ી ુ ૂ ે ૈ ો ૌ ં ઃ are written after
  their consonant, even િ, which sits to its left. The table's value is now
  documented as the written ORDER, which in Tamil is also the side. Gujarati
  words are therefore drawn in typed order (`કેમકે` → ક, ે, મ, ક, ે); ો and ૌ
  have no decomposition and stay one sign each.
- **Every row is cited.** The Gujarati mark records gain `compositionOrder`
  and a `compositionSource` citing KanoAI's barakhadi templates, which draw
  the consonant's group before the sign's in every row whose consonant keeps
  its bare outline (33 of 34 for િ; ઢિ is the exception). A new case,
  `gujarati-signs-in-written-order`, holds the table to the records, and a new
  `script-inventories/gujarati.evidence.ts` pins every Gujarati mark record
  (digest, `strokeOrder`, `penLifts`, sources) and that ્ and ૃ claim neither
  a ductus nor a place.
- **What stays refused.** ્ and ૃ (no Gujarati source; the Devanagari analogy
  is not used); two signs written on the same side of one consonant (ાં, ેં:
  no source orders them, and the rule changes nothing in Tamil); and the pairs
  the bundled Noto Sans Gujarati reshapes, read from its GSUB table and listed
  in `FUSED_SIGN_PAIRS.gujarati` (54 pairs): ણુ, રુ, રૂ (ligatures); the 22
  consonants that take a stem form before ુ and ૂ (નુ, મૂ, ...); and જ and ૹ
  with ા, ી, ો and ૌ, whose ā bar joins the consonant. So `બજાર` and `જો`
  stay undrawn.
- **The figure.** A strip of parts takes its order sentence from
  `writtenOrderNote(script)`: Tamil's `<desc>` is byte for byte what it was;
  Gujarati's says its parts are in typed order, each sign after its consonant,
  even the i sign.
- **22 Gujarati lessons gained a filmstrip** (Gujarati target count 41 → 63):
  nine sign lessons (ા ે ં ી ો ુ િ ૂ ૈ), seven word lessons (અને, કે, કેમકે,
  તે, and હા three times) and six glyph lists that include a sign.
  Regenerated: the Gujarati filmstrip ledger owner, 22 SVGs, the Gujarati
  figure-hash owner and 14 Gujarati book chapters. Narration and modality do
  not change. The real-corpus case pins the 22 lessons and the seven that stay
  refused (virama, vocalic r, નમસ્તે, ક્યાં, જો and two lists with રસ્તો).
