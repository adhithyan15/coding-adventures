## 0.19.0 — the full vowel row: ai, au & vocalic R (syllabary, PR 3)

- **Each consonant now carries three more syllables.** The generator's core
  vowel row was the ten short/long vowels (a ā i ī u ū e ē o ō); it now also
  composes the two **diphthongs** (ai, au) and the **vocalic R** that
  Sanskrit-derived words carry — కృ = *kr̥*, as in కృష్ణ *kr̥ṣṇa* "Krishna". So a
  consonant's row grows from 10 to 13, and the regenerated data goes **Telugu
  350 → 455, Kannada 350 → 455, Malayalam 360 → 468** syllables.
- **Still Unicode-grounded.** The three new syllables are composed from the
  `VOWEL SIGN AI` / `AU` / `VOCALIC R` code points of each block, verified to
  exist by their official Unicode names before use — nothing hand-typed. The
  vocalic-R romanization is **ISO-15919 `r̥`** (a plain *r* with a combining ring
  below), deliberately *not* IAST's dot-below `ṛ` — in ISO-15919 that dot-below
  form is the *retroflex* ṛ, a different sound, so using it would be wrong.
- **Flows through the slow-unlock gate unchanged.** The new syllables are signed
  (two components), so `consonantGroups` keeps them inside their consonant's
  group automatically: Practice on Telugu now reads *"mastered 0 / 13"* and the
  first row is `ka kā ki kī ku kū ke kē ko kō kai kau kr̥`. No app code changed —
  only the generator and the regenerated JSON (plus the one data-dependent test
  assertion, 10 → 13). Still recognition only (`strokeOrder: []`).

