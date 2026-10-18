## Chapters 28–30 — the track's first verbs (2026-08-06)

- Added six lessons, one verb each, realizing six **canonical** `VERB-*`
  concepts: `AR-C28-dhahaba` (VERB-GO), `AR-C28-jaa` (VERB-COME),
  `AR-C29-qala` (VERB-SAY), `AR-C29-raa` (VERB-SEE), `AR-C30-arafa`
  (VERB-KNOW), `AR-C30-akala` (VERB-EAT). Arabic taught **zero** verbs before
  this; it is now the first track in the corpus to realize any core verb
  concept at all — 6 of 40 — and the first to place lessons on an **A2** spine
  node (`SPINE-SAY-WHAT-I-DO`).
- **The root-and-pattern system is the subject, not a remark.** `dhahaba` opens
  on ذ-ه-ب giving "he went," *dhahab* ("gold") and *madhhab* ("a school of
  law"), then pays the *ma-* place shape off against **mosque** (*masjid*) and
  the **Maghreb** (*maghrib*) — words English already owns, so the pattern
  arrives already-known and only the root is new. A Grammar Lens states the
  load-bearing fact: Arabic has **no infinitive**, so the dictionary form is
  *dhahaba*, "he went," the فَعَلَ (*faʿala*) shape every verb here wears.
- **Weak roots taught as regularity, not exception**: middle-weak in *jāʾa*
  (ج-ي-أ, the *yāʾ* walking back out in *majīʾ*) and *qāla* (ق-و-ل, the *wāw*
  back in *qawl*); final-weak in *raʾā* (ر-أ-ي, surfacing as the dotless ى).
  Pattern families then carry the vocabulary: *qāʾil*/*maqūl*/*maqāla* reuse
  the doer, done-to and place shapes met in *shukran*; *mirʾāh* ("mirror")
  introduces the instrument shape; *raʾy* ("opinion") is a *seeing*, the same
  metaphor as English "in my view."
- **Cousins claimed only where they are real.** *ʿarafa* carries a genuine
  Al-Andalus loan — *al-ʿarīf* → Spanish *alarife*, and the **Generalife** in
  Granada, flagged as the leading reading rather than a settled one — and its
  root-sibling ع-ل-م gives *ʿālim*, whose plural English borrowed as **ulema**.
  *akala* says the opposite plainly: **no English cousin exists**, because
  Arabic and English are unrelated; what it has is the Hebrew twin *akhal*,
  beside Arabic *akl* and Hebrew *okhel*.
- **Letters taught inline, per HL00.** New: **ذ** (*dhāl*, *dāl* plus one dot),
  **ق** (*qāf*), **ى** (*alif maqṣūra*, *yāʾ* stripped of its dots) and **ف**
  (*fāʾ*) — the last two a fourth dots-family, one dot against two on one body.
  ق and ى were both flagged in Chapters 4 and 24 as letters the reader could
  read but had not been taught; this arc names them. `AR-C30-akala` needs
  **no new letters at all**, which is the inline rule paying itself off.
- **All six measure `voice`** and all three chapters are drivable end to end:
  the letter work is speakable prose, with no table, no `script` block and no
  sight cue. Track modality moves 50/5/16 → **56 `voice` / 5 `sight` / 16
  `pen`**, 70% → **73% drivable**, 42 → **48** lessons reachable in
  chapter-prefix order. Corpus totals move 1,134 → 1,140 lessons, 957 → 963
  `voice`, `sight` and `pen` unchanged, 377 → 380 chapters.
- Wired through the whole pipeline: `curriculum.json` gains `AR-PATH-027` on
  `SPINE-SAY-WHAT-I-DO` with the `AR-EXT-027-LANGUAGE-SPECIFIC` extension;
  `chapters.json` gains three HL05 ledger entries whose payoffs each assess
  three atoms, above the representativeness floor; the book gains three
  generated chapters and three `\input` lines. Verified with a real XeLaTeX
  run: 114 pages, **0 overfull, 0 underfull, 0 missing characters**.

