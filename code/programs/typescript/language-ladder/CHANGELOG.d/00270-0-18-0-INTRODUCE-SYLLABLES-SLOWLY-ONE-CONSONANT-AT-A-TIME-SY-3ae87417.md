## 0.18.0 — introduce syllables slowly, one consonant at a time (syllabary, PR 2)

- **The Dravidian drill no longer dumps 350 syllables at once.** Practice on
  Telugu / Kannada / Malayalam now opens with a *single consonant's vowel row*
  (ka kā ki kī ku kū ke kē ko kō) and unlocks the next consonant only once the
  current row is mastered — the "ka, ki, ku … kha, khi, khu" build-up the app is
  meant to teach. This is recognition pattern-building, done the slow way.
- **New pure module `src/syllabary.ts`** is the gate: `consonantGroups` segments
  the consonant-major syllabary at each bare consonant (a grounded boundary — a
  base syllable has one component, a signed one has two), `unlockedConsonantCount`
  counts how many rows are open given the SRS state (a Leitner box ≥ 3 marks a
  syllable mastered; a gap holds everything after it locked), and
  `unlockedLetterIndices` returns the currently drillable subset. No DOM, no
  globals; unit-tested with a control that keeps the 2nd consonant locked until
  the 1st row is fully mastered, plus a check against the real generated Telugu
  data (35 consonants, a 10-syllable first row).
- **Practice wiring** (`main.ts`): on a syllabary in single-script scope, the
  scheduler picks the next question *only from unlocked syllables*, distractors
  are drawn *only from unlocked syllables* (a not-yet-introduced consonant never
  appears as a decoy), the mastery read-out is scoped to the unlocked rows
  (`mastered 0 / 10`, not `0 / 350`), and a cue reads **"Learning consonant N of
  M — master this vowel row to unlock the next."** The alphabets and Mixed mode
  are untouched (the gate is null for them).

