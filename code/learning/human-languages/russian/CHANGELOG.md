# Changelog — Russian track

## [Unreleased]

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
- Stroke data is the canonical `data/scripts/cyrillic.json` the companion
  `script-writing-visualizer` app renders, so the lessons and the app agree.

### Notes
- Headwords use the lowercase citation form (Cyrillic case is not yet in the
  script inventory).
- The LaTeX book is authored next (lessons-first workflow), typeset with the
  vendored `NotoSansCyrillic-Static.ttf`.
