## Unreleased — nine more Devanagari characters, one per lesson

Sanskrit's script closure moves for the first time since the ladder stalled at
fourteen characters. Measured with `report-cli` before and after:

| | before | after |
|---|---|---|
| characters taught by a script lesson | 35 | 44 |
| characters SHOWN but never taught | **18** | **9** |
| lessons decoding an untaught character | **56** | **46** |
| script lessons | 14 | 23 |

Corpus-wide the closure figure falls 643 → 633; every one of those ten is
Sanskrit. Nothing else in the gap report moves: the script ramp still reports 39
lessons above three new glyphs (none of them these), the representativeness
floor still reports 80 payoffs, and the 300-second and drivable-prefix counts
are untouched.

`SA-S115`–`SA-S123`, one character each, placed **last in their chapter** so the
words that contain them are met glossed first and the character arrives after —
the HL08 placement measurement that chose the earlier segments' position applies
unchanged, and end-of-chapter costs no drivable prefix.

- **◌ौ** (ch. 15) — anchored on द्वौ and कर्णौ, which are the two **dual** forms
  the reader already says. The sign is introduced as the mark that has been on
  the page every time the book counted to exactly two.
- **ट** (ch. 16) — कुटुम्बम्. The retroflex tongue position is named against त,
  and the surviving ट of Hindi कुटुंब is pointed at.
- **ठ** (ch. 16) — पठति, पाठ. Taught immediately after ट and set beside it: same
  two pieces, one unbroken run in ट and a closed body with a lift in ठ.
- **ष** (ch. 17) — क्षीरम्, अक्षि, अक्षिणी. All three carry ष inside the
  conjunct क्ष, and the lesson says so: **owning both halves is not owning the
  join**. क्ष is left for its own lesson rather than assumed free.
- **ख** (ch. 18) — खादति, लिखति, मुखम्.
- **ज** (ch. 19) — जानाति, ज्ञान, with the ज्ञ join named as *jña* in some
  regions and *gya* in others.
- **उ** (ch. 20) — उत्तम, उदक, उपसर्ग; the independent shape set against the
  hook the same vowel becomes inside a word.
- **फ** (ch. 24) — फलम्; the aspirate related to प the way ठ is to ट.
- **ओ** (ch. 24) — ओदनः; the whole of आ is inside ओ with one arc added, which is
  how the vowel letters are built.

Every pen path, pen-lift count, stroke note and citation is READ from the
committed `data/scripts/devanagari.json` rather than typed. ◌ौ is a mark and
that file records no stroke source for marks, so it gets the existing
copy-what-you-see block and says plainly that the direction is not written down
here yet.

**Each lesson introduces exactly one new Devanagari character and no other.**
That was checked mechanically rather than by eye: every anchor word, worked
example and practice line was audited against the set of characters already
taught at that point in reading order, and three candidates were replaced
because they would have credited a character the reader had not met — मुखम् in
the ट and ष lessons, which carries ख. Without that audit the improvement would
have been laundered through example words instead of taught.

**What was deliberately not done, and why** (backlog `HL-C217`): the nine
characters still untaught are ऋ and ङ, which have **no cited stroke order** in
the shared Devanagari file, and इ ई घ ड ँ ू ◌ै, which have data but **no Sanskrit
headword containing them** — they appear only in metalinguistic notes, so a
"you already say these" list would be empty. The first group needs a shared-file
branch; the second needs vocabulary before script.

