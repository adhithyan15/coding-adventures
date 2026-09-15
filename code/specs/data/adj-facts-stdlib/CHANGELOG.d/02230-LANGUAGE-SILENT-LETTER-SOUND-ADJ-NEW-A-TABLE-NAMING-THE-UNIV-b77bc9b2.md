- `language/silent-letter-sound.adj` (new) — a `table` naming the University of Florida Literacy
  Institute (UFLI) Foundations Toolbox's sole "Silent Letters Unit" lesson (lesson 98) and the
  single speech sound each of its three named silent-letter consonant-cluster spellings actually
  represents: `silent_letter_sound(spelling, sound)`, `kn` → `n_sound`, `wr` → `r_sound`, `mb` →
  `m_sound`. The FOURTH UFLI phonics unit shipped in this stdlib (after `digraph-sound.adj`,
  `diphthong-sound.adj`, `long-vowel-team-sound.adj`), genuinely distinct from all of them: a
  digraph blends two letters into one consonant sound, a diphthong glides between two vowel
  positions, a long-vowel team spells one un-glided long vowel sound — a silent-letter pattern
  instead DROPS one letter's sound entirely, the surviving letter alone carrying the whole
  pronunciation. Sourced from the SAME UFLI page `diphthong-sound.adj` already cites
  ("Diphthongs and Silent Letters Units (Lessons 95-98)") — that file's own header already
  documented, when it shipped, exactly why lesson 98 was deliberately NOT one of ITS rows ("the
  page's own heading splits it out as a separate 'Silent Letters Unit'... a silent
  CONSONANT-cluster spelling convention, not a vowel diphthong"); this table is the natural
  completion of that already-flagged exclusion, so it required zero new WebFetch beyond a
  byte-for-byte re-verification of the already-cited page (confirmed the page's own prose:
  "The Silent Letters Unit only consists of one lesson, but this lesson instructs students on
  three common silent letter patterns (e.g., kn-, wr-, and -mb)."; lesson table: "98 kn /n/, wr
  /r/, mb /m/"). Also independently confirmed against UFLI's own official Scope & Sequence PDF
  that "Silent Letters" is its own named unit line, distinct from "Diphthongs" — not an
  ad hoc grouping invented by this stdlib. A full-tree grep for `row (kn,`/`row (wr,`/`row (mb,`
  and for `n_sound`/`r_sound`/`m_sound` as atom labels found zero prior collisions anywhere in
  this stdlib (the two whole-word hits for "know"/"write" in `dolch-sight-word-level.adj` are
  unrelated — that table names whole WORDS, never the bare two-letter spellings themselves).
  Empirically verified all three rows (forward and reverse) plus the honest abstention on `gh`
  (a real silent-letter pattern, e.g. "night", but not one of this UFLI lesson's three named
  patterns) against the real built `adj-lang-cli` binary in a scratch table before writing the
  shipped file. New `silent-letter-sound.query.adj` and `facts_silentlettersound_e2e.rs` (4
  tests); new manifest objective `adj.literacy.k2.silent_letter_sound` added via a surgical text
  edit (verified with `git diff --stat`, JSON validated before write). Also resolves a genuine,
  long-flagged stdlib-tracking staleness: the `r-controlled-vowel-word.adj` UFLI unit (shipped
  2026-08-10, PR #10508, predating this session's Thread-numbered log) had been mistakenly
  treated by recent session notes as an "open" candidate 4th/5th UFLI unit — it is not; "R-
  Controlled Vowels" was already shipped. This round confirmed that via `git log
  --diff-filter=A` on the file itself and picked "Silent Letters" instead, the genuinely open
  UFLI unit.

