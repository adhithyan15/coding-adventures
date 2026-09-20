## Writing track W01–W04 — the first handwriting lessons for any Dravidian language

Four tracks — Tamil, Telugu, Kannada, Malayalam — had reached Chapter 6 with
vocabulary and **no way to learn to read it**, because `data/scripts/` had no
letter data for any Dravidian script. This adds **`tamil.json`** and the first
four writing lessons.

### The data

`tamil.json` ships **11 letters and 4 marks** — deliberately not the full
247-character grid. It covers exactly what Chapter 1's words need, is marked
`complete: false`, and carries `strokeOrderNote: "conventional"` throughout,
because Tamil handwriting varies by region and school and the descriptions name
**the pieces a learner can see** rather than prescribing one correct hand.

Every glyph was verified against its Unicode name, and every mark's `example`
was verified by actually concatenating base + mark and comparing to the claimed
result — so `ற` + `ி` really does produce `றி`.

**Also wires `gujarati.json` into the app**, which had existed since the Gujarati
track was authored and was never added to `SCRIPTS` — so the app rendered five
scripts while six had data. It now renders seven.

### The lessons — in book order, one piece at a time

- **`TA-W01` — வ, க.** Opens on the question the script answers: **why is Tamil
  round?** The usual account is **palm leaves**: incised with a stylus, where a
  straight stroke *along the grain* can split the leaf, so strokes bend into
  curves. Given as **the standard explanation, not a settled fact** — earliest
  Tamil-Brahmi is angular, the rounding came later via Vaṭṭeḻuttu, and Devanagari
  was written on the same leaves without going round. What survives the hedge is
  the general point: **the tool leaves fingerprints on the letters.** (Compare
  Latin's straight strokes, which suit a wax tablet and a chisel.) The lesson
  also notes that straight strokes *do* exist in Tamil — most of these letters
  have a flat bar across the top. Then the **abugida** principle,
  and that **one letter க spells *k*, *g* and *h***, decided by position — the
  reason Tamil needs 18 consonants where Devanagari needs 33.
- **`TA-W02` — ம, ண.** The **retroflex**: tongue curled back to the roof of the
  mouth, a sound English has no letter for and a beginner genuinely cannot hear
  at first — said plainly, because the alternative is a learner assuming they're
  failing. Sets up the three-n table (**ந** dental · **ன** alveolar · **ண**
  retroflex) without yet drawing the other two.
- **`TA-W03` — the puḷḷi ்**, "the dot", which removes the inherent vowel. The
  lesson's real content is a **divergence from Devanagari**: Tamil does **not**
  fuse the bare consonant into a conjunct — both letters keep their full shape
  and the dot stays visible. The consequence is arithmetic: ~247 characters
  total, against Devanagari's hundreds of ligature shapes. Tamil traded a smaller
  alphabet for slightly longer words. **Assembles வணக்கம்**, the first word of
  the course, and flags the doubled **க்க** (*vaṇak-kam*).
- **`TA-W04` — ந, ன, ற and the vowel sign ி.** Completes the three n's and adds
  the third thing a consonant can do (keep its vowel / lose it / **replace** it).
  **Assembles நன்றி** — and the payoff is that **ன் + ற** is pronounced together
  as *ndr*. That is presented as **one instance of a general Tamil rule** — a
  nasal voices the stop that follows it (ந்த *nd* · ண்ட *ṇḍ* · ன்ற *ndr*) —
  rather than as a property unique to those two letters. Each of the three n's
  produces its own cluster, and the spelling tells you which: that is the three
  n's earning their keep. It is also why *naṉṟi*, which is how Chapter 1
  romanizes it, is so often written *nandri* in English.

### Checks

- `sounds:` ids reused from the track's existing vocabulary (`tamil-inherent-a`,
  `pulli`, `retroflex-n`, `dental-vs-alveolar-vs-retroflex-n`, `matra-i`,
  `gemination-kk`, `final-m`) rather than invented.
- `writing`-type lessons carry **no `concept_tag`**, per the convention.
- Every Tamil character used in the four lessons was checked against the data
  file. Eleven appear that aren't in it — ஃ ங ட த ப ர ள ஷ ஸ ீ ு — all inside
  **quoted words and examples**, never in a `[YOU WRITE: …]` directive. W03
  carries a **standing read-now-draw-later note for the whole track** rather than
  an enumerated list, since the list grows with each new example. Verified
  mechanically: every character the lessons ask the learner to *draw*
  (க ண ந ன ம ற வ ி ்) has a real entry in `tamil.json`.
- **Letter shapes verified by rasterising the vendored font, not by eye.** A
  throwaway zero-dependency TTF reader (cmap → loca → glyf) extracted each
  glyph's true outline and scan-converted it to a text bitmap, which is what the
  `components` and `strokeOrder` descriptions were written against. This caught
  three descriptions that were confidently wrong:
  - **ற** was described as "a single arch, like a Latin n". It has **two**
    arches and three legs, with the right leg continuing below the baseline,
    sweeping left and dropping into a long descender.
  - **ண vs ன**: ண is **ன with one extra arch** — both open with the same top
    bar and loop and close with the same straight vertical, and ண carries two
    arches between them where ன carries one.

    This one is worth recording as a caution, because an intermediate revision
    of these lessons **replaced that correct statement with a false one** —
    claiming the two letters differed only in a final curving stroke. The cause
    was a rasteriser windowed to x≤1030 when **ண's outline runs to x=1631**:
    it amputated 37% of the letter, including the final vertical, and the
    description was then written from the truncated picture. A measurement
    instrument can be as confidently wrong as a memory. The window is now
    derived from the glyph's own bounding box, and the test asserting this
    fails if the window ever clips again.
- Verified in the browser: Tamil renders with 11 letters and "inventory in
  progress"; console clean.

