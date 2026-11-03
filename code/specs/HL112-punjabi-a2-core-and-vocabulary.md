# HL112 — Punjabi A2: core chapters 211-224 and vocabulary chapters 225-364

The Punjabi track attains A1. A2 is touched but not attained. On the level gate
(`node tools/gate.mjs punjabi` over the current corpus), three of the five A2
spine nodes are unrealized:

- `SPINE-NEGATE-AND-ASK`;
- `SPINE-TALK-ABOUT-PAST`;
- `SPINE-TALK-ABOUT-FUTURE`.

The track also teaches 626 headwords against 1,200 and 82 verbs against 120,
and two script atoms are revisited only once:

- `PA-SCRIPT-RECOG-YA-01` (taught by PA-C181-yayya);
- `PA-SCRIPT-RECOG-E-206` (taught by PA-C206-e).

Chapters 176-210 already teach much of the A2 grammar: the past copula ਸੀ/ਸਨ,
the past forms ਮਿਲਿਆ, ਕੀਤਾ, ਗਏ and ਪਿਆ, the agent marker ਨੇ, and the futures
ਹੋਵੇਗਾ and ਆਵੇਗਾ. All of them sit on the A1 node READ-PRACTICAL-TEXTS or its
neighbours. Corpus tests pin those chapters to their nodes, so this spec adds
chapters rather than moving them.

## Part 1 — the A2 core, chapters 211-224 (hand-authored)

Every chapter opens on a review a learner can do by ear and closes on a payoff
that uses all four skills (listen, speak, read, write). Every lesson stays
under 300 seconds.

| chapters | node | teaches |
|---|---|---|
| 211 | SAY-WHAT-I-DO | the vowel-stem present: ਜਾਂਦਾ, ਖਾਂਦਾ, ਪੀਂਦਾ, with -ਦੀ and -ਦੇ |
| 212-215 | NEGATE-AND-ASK | ਨਹੀਂ before a present verb with the helper kept (`VERB-NEGATE`); ਨਹੀਂ ਹੈ; the polite request in -ੋ on consonant stems, and ਨਾ before it; ਅਜੇ; ਕੀ yes/no questions (`QUESTION-POLAR`), ਹਾਂ ਜੀ / ਨਹੀਂ ਜੀ |
| 216-220 | TALK-ABOUT-PAST | ਗਿਆ / ਗਈ / ਗਏ (`VERB-PAST`) and ਕੱਲ੍ਹ; ਆਇਆ / ਆਈ / ਆਏ; -ਿਆ / -ੀ and ਨਹੀਂ + past; the transitive past agreeing with its object after ਮੈਂ / ਅਸੀਂ / ਤੁਸੀਂ; a name or ਉਸ + ਨੇ; ਲਿਆ / ਦਿੱਤਾ; ਕੀ + past |
| 221-223 | TALK-ABOUT-FUTURE | ਜਾਵਾਂਗਾ / ਜਾਵੇਗਾ / ਜਾਵਾਂਗੇ (`VERB-FUTURE`); ਕਰਾਂਗਾ / ਕਰੇਗਾ / ਕਰੋਗੇ / ਕਰਨਗੇ; the future with ਕੀ and ਨਹੀਂ; a spoken plan with ਸ਼ਾਇਦ |
| 224 | READ-PRACTICAL-TEXTS | a short message read for what happened, what did not, and what will happen when; questions by ear; the final review |

**Decisions:**

- **The helper verb stays after ਨਹੀਂ** (ਮੈਂ ਚਾਹ ਨਹੀਂ ਪੀਂਦਾ ਹਾਂ). This keeps
  faith with chapter 37's rule that nothing else in the clause moves.
- **Third-person transitive pasts use only a name + ਨੇ or ਉਸ + ਨੇ.**
  ਉਨ੍ਹਾਂ ਨੇ and other noun agents wait for B1.
- **No vowel-stem request in -ਓ (ਜਾਓ).** It needs the independent letter ਓ,
  which the track does not teach. The request is taught on consonant stems
  only (ਬੈਠੋ, ਉੱਠੋ, ਪੜ੍ਹੋ beside ਕਰੋ).
- **Two copy-only letters are added: ਔ (anchor ਔਰਤ, chapter 217) and ਢ
  (anchor ਕੱਢਣਾ, chapter 219).** No stroke order is claimed for either. The
  Gurmukhi inventory grows from 36 to 38 letters, and its glyph-evidence pins
  move with it. Ductus for ਢ is a follow-up, pending a citable source.
- **The thin atoms are revisited inside the core**, so the other session's
  chapters stay untouched. Chapter 214 revisits YA through ਯਾਤਰੀ, and chapter
  216 revisits ਏ through ਗਏ.

## Part 2 — the A2 vocabulary, chapters 225-364 (five tranches)

Five tranches, a to e. Each has 28 chapters of five `word` lessons, so 140
words, of which at least 25% are verbs. Only A2, A1 and pre-A1 nodes are used,
never B1.

| tranche | chapters | nodes |
|---|---|---|
| a | 225-252 | SAY-WHAT-I-DO, NAME-EVERYDAY-ACTIONS, NAME-EVERYDAY-THINGS |
| b | 253-280 | READ-PRACTICAL-TEXTS, ASK-LOCATION, SAY-WHAT-I-WANT |
| c | 281-308 | TALK-ABOUT-FUTURE, DESCRIBE-QUALITIES, NAME-EVERYDAY-THINGS |
| d | 309-336 | TALK-ABOUT-PAST, CHECK-WELLBEING, EXCHANGE-NAMES |
| e | 337-364 | NEGATE-AND-ASK, SAY-WHAT-I-LIKE, SAY-WHAT-I-HAVE-AND-CAN-DO |

Each tranche is drafted, then reviewed independently, against the paa2
builder. The builder refuses:

- untaught letters and grammar;
- ergative errors;
- Hindi forms;
- sight phrases ("see the", "look at", "shown below", "the table", "the
  chart", "column", "written above") anywhere a recall drill could quote them.

**Etymology:**

- Every origin is a CDIAL row the tool shows, a hedged donor that reuses the
  corpus slug, or "origin remains unknown". No origin is invented.
- A Punjabi row that CDIAL marks as borrowed from Hindi is worded "goes back
  to … as a word taken from Hindi". A row found only in Lahnda is named as
  Lahnda.
- A sense CDIAL does not give is said so in the lesson.
- Lane, Steingass, Platts and Wiktionary are never cited.

## Acceptance

- `gate.mjs punjabi`:
  - attained A2, reach A2;
  - all five A2 spine nodes realized;
  - at least 1,200 headwords and at least 120 verbs at or below A2;
  - THIN 0, forward references unchanged (pinned at 2), over budget 0,
    order defects 0.
- The openers check:
  - every chapter's first lesson can be driven by ear;
  - the drivable-prefix ceiling does not rise.
- Letter anchoring:
  - cold 0, unread inventory 0, unwritten 0;
  - buildsToward does not rise.
- One session-map row per lesson, in sequence order.
- The full human-language-data, language-ladder and script-ductus vitests
  pass; every `check:*` script passes.
- The strict Punjabi book compile shows 0 warnings in every LaTeX class.
- `tests/level-gate-attainment/punjabi.json` gives attained A2 and reach A2,
  with evidence naming chapters 211-364.
