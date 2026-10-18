## [Unreleased]

### Added — project-defined pre-A1 four-skill task shapes

- Made the Russian pre-A1 bridge executable as separate reading, listening,
  writing, and speaking papers with exact items, timing, replay, aids, scoring,
  and an independent 60/100 floor for every skill.
- Kept the writing evidence honest: delayed recall, heard-word Cyrillic
  transcription, and bounded independent production are scored; visible
  tracing and copying remain lesson supports and cannot earn exam credit.
- This inventory is not a readiness claim. Two full mocks, rubrics, answer
  keys, calibration, curriculum task coverage, and book-only human validation
  remain required before even project pre-A1 readiness can be claimed.

### Added — Chapter 14, eleven letters sorted into three kinds (HL-C219)

`scriptLessons` 5 → 16, `taughtGlyphs` 18 → 30, `neverTaughtGlyphs` **37 → 25**.

**Cyrillic is an alphabet, so the abugida shape does not apply.** The five tracks
before this one were taught as *inherent vowel → mātrā → …*; none of that exists
here. The organising idea Cyrillic needs is different, and it is about the
reader's Latin habits rather than about the script:

- **true friends** — look Latin, sound Latin (к, м, о, с, т);
- **false friends** — look Latin, sound different (у, в, р, с, н);
- **new shapes** — no Latin relative at all (з, ж, ч, ш, л).

Every letter sorts into one of the three, and sorting them is most of what
learning this alphabet is. Each lesson ends by asking which kind it was.

**The eleven were chosen by how many words each unblocks**, not by alphabet order:
у alone was blocking 41 lessons, з 27, ы 25.

**Two lessons carry ideas rather than shapes.** *ь*, the soft sign, is **a letter
that makes no sound at all** — it softens the consonant before it, and a letter
that is a modifier rather than a sound is the one genuinely alien object in this
script for an English reader. And *я* is **a whole syllable in one letter**, plus
the backwards-R resemblance that means nothing and appears on every parody of
Russian ever printed.

### Also: the five older writing lessons now declare `delivery: script`

The manifest's rule is that once a track marks any writing lesson, **every**
writing lesson in that track must. `RU-W01`–`RU-W05` predate the marker. Marking
them is why `taughtGlyphs` jumps 18 → 30 rather than 18 → 29: their letters now
count for closure as well as being taught.


### Added — Chapter 1 (Greetings & courtesy)
- Track scaffold: `README.md`, `roadmap.md`, `session-map.md`,
  `pronunciation-reference.md`, and `track.json` declaring the **Cyrillic**
  script (so the data layer resolves Russian → cyrillic).
- Six word lessons, Cyrillic taught inline:
  - `RU-C01-privet` — привет (informal hi); the *-вет* "speak" root ↔ **Soviet**.
  - `RU-C01-zdravstvuyte` — здравствуйте (formal hello); "be healthy", polite `-те`.
  - `RU-C01-spasibo` — спасибо (thank you); worn-down *спаси Бог*, "God save you".
  - `RU-C01-da` — да (yes).
  - `RU-C01-net` — нет (no); *не + есть* "not-is", the PIE **\*ne** cousin of *no/not*.
  - `RU-C01-pozhaluysta` — пожалуйста (please / you're welcome); the favour root *жал-*.
- `RU-C01-practice` — Chapter 1 recap drilling the four false friends (в=v, р=r,
  с=s, н=n) and the greeting exchange.
- Uses the canonical concept taxonomy; adds `COURTESY-PLEASE` to the taxonomy for
  пожалуйста.

### Added — Writing the letters (the "break it apart and write it" strand)
- Three `writing`-type lessons (the HL02 hand-writing surface, taught inline the
  same etymology-first way; no `concept_tag`, exempt from the cross-language join).
  Each breaks a letter into its component strokes with a stroke order and reviews
  the Chapter 1 word it lives in:
  - `RU-W01-false-friends-v-r` — writing **в** (v, ← Greek beta) and **р** (r, ←
    Greek rho): the two false friends from *привет*, stroke by stroke.
  - `RU-W02-false-friends-s-n` — writing **с** (s, ← Greek sigma) and **н** (n,
    the Latin-*H* look-alike), completing the four false friends в·р·с·н.
  - `RU-W03-new-shapes-b-d` — writing **б** (b) and **д** (d, ← Greek delta), two
    shapes with no Latin disguise; contrasts б vs в (the top flag + one belly vs
    two bellies).
  - `RU-W04-privet-letters-p-i` — writing **п** (p, ← Greek pi Π) and **и** (ee,
    the quiet false friend — a *backwards* Latin N: its diagonal **rises** where
    N's falls); contrasts п (top bar) vs н (middle bar).
  - `RU-W05-privet-letters-e-t` — writing **е** (ye, an *iotated* honest vowel) and
    **т** (t, ← Greek tau); **completes every letter of привет** (п·р·и·в·е·т), so
    the learner can hand-write their first Russian word end to end.
- Stroke data is the canonical `data/scripts/cyrillic.json` the companion
  `language-ladder` app renders, so the lessons and the app agree.

### Notes
- Headwords use the lowercase citation form (Cyrillic case is not yet in the
  script inventory).
- The LaTeX book is authored next (lessons-first workflow), typeset with the
  vendored `NotoSansCyrillic-Static.ttf`.
