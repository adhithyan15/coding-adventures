## 0.20.0 — flag the special consonants: retroflex ḷ, alveolar ṟ / ṉ (syllabary, PR 4)

- **The three Dravidian "special" consonants now carry a contrast hint.** To an
  outsider ల vs ళ (*la* vs *ḷa*) is the kind of near-miss that stalls reading, so
  the app now flags the **retroflex ḷ** and the **alveolar ṟ / ṉ** the same way
  it flags Latin false friends: a **★ special consonant** badge on the Browse
  detail, a *"Special letter — tell it apart from 'l/r/n'"* section with a
  grounded note on how it differs, and a tinted grid tile. No new data — these
  letters were already generated (LLA / RRA / NNNA); this only surfaces them.
- **New pure helper `specialConsonant(letter)` in `core.ts`** (mirrors
  `isFalseFriend`): it keys on the syllable's ISO-15919 romanization, which is
  script-agnostic — the leading code point ḷ (U+1E37, dot below) / ṟ (U+1E5F) /
  ṉ (U+1E49, line below) is the retroflex/alveolar marker. Those marks appear
  *only* on these consonants in our data — the vocalic-R vowel uses a different
  code point (ring-below r̥, U+0325) — so the test is exact, not heuristic.
  `LetterView` gains a `special` field. Unit-tested with a **control** that keeps
  the ordinary l / r / n and the vocalic r̥ un-flagged, plus a check that exactly
  the 26 LLA+RRA rows of the real Telugu data are marked (Telugu has no ṉ).

