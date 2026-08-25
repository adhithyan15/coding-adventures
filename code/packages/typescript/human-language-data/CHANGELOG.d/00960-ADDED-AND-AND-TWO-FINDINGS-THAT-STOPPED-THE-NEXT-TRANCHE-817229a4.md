### Added — உ and ஊ, and two findings that stopped the next tranche

The plan after chapters 4-5 was to keep going: 22 lessons in the generated chapters
33-38 still carry a `## The letters in this word` section, and the obvious next step was
to strip them the way chapters 2-5 were stripped. Measuring first says otherwise, and
the measurement is the substance of this entry.

**Chapters 33-38 are not the same problem.** Running the strict taught-test over all 22
sections: **every glyph in them is already taught by the strand, except two** — **உ**
and **ஊ**. That is the opposite of chapters 2-5, where the sections were the *only*
place their glyphs were explained. Most of these sections review letters the reader has
already been taught, thirty-odd chapters into a track whose script strand starts at
chapter 4.

With one exception this tranche does **not** fix. `TA-C34-utavu` (sequence 1000) says
"**உ** is the standing vowel *u*, used when a word begins with it" — introducing the
glyph 85 sequence units *ahead* of `TA-W17`, and `TA-C36-unavu` and `TA-C37-uur` do the
same a little later. The strand cannot get there earlier without spelling a word the
learner has not been given, which is the rule it exists to keep. Nothing detects this,
either: those sections declare no atom for **உ**, so `forwardReferences` holding at 423
is not evidence against it.

**They also already cost the speaking learner nothing.** All 22 record
`detachableSegments: ["The letters in this word"]` with `coreModality: voice` and
`coreDrivable: true` in the generated manifest. The spoken-only edition those markers
exist for can already drop them. They are 22 of the 414 lessons corpus-wide where
`drivable` and `coreDrivable` disagree — a small share of a seam that exists across many
tracks, and working as designed in all of them.

**What they carry is mostly reinforcement, and one genuinely new thing.** An earlier
draft of this entry claimed the strand does not teach the no-fusion rule or positional
softening. That was wrong, and the fix below proves it wrong: `TA-W03-pulli-vanakkam`
has a whole "What Tamil does *not* do" section with the Devanagari contrast
(`TA-SCRIPT-PULLI-VANAKKAM-02`), and `TA-W01-abugida-va-ka` states the positional rule
outright (`TA-SOUND-ABUGIDA-VA-KA-04`). The chapter 33-38 sections *declare* those very
atoms in their own `assesses` lists — the corpus already says they are reinforcing the
strand, not replacing it.

What they add is thinner than that, and worth stating exactly. **ட**'s softening is
already in the strand too — `TA-W12-read-eppadi` says it outright, and both chapter 33-34
lessons that repeat it come later. What is genuinely new is **த**'s softening (the strand
teaches **த** in `TA-W16` without it), **ற்கா** as a second place two consonants refuse
to fuse, and one framing the strand does not have anywhere: `TA-C36-paal`'s minimal pair,
that a single dot is the whole difference between **பால்** and a non-word.

So the case for keeping them is narrower than the first draft claimed, but it holds:
they cost the spoken edition nothing, they reinforce atoms they correctly declare, and
they extend rules to letters the strand introduces later. Deleting all 22 would move
`sight` down by 22 and `voice` up by 22 — which reads as progress on the headline
modality numbers while losing that. That is the "a net that matches does not mean the
story is right" trap this changelog keeps recording, so the sections stay and this entry
records what was measured instead.

What was genuinely wrong is smaller, and is fixed here.

- **`TA-W17-read-unavu`** (chapter 36) — **உ**, the standing short *u*, spelling
  **உணவு**. The lesson's whole point is that the word holds the same vowel twice in its
  two forms: the letter that opens a word, and the **ு** sign that rides **வ**.
- **`TA-W18-read-uur`** (chapter 37) — **ஊ**, its long partner, spelling **ஊர்**.

**The second finding: those were the last two glyphs *in chapters 33-38*, not in the
corpus.** A census of every Tamil codepoint in every Tamil lesson against the strand's
taught set leaves **19** glyphs still used but never taught, and the cluster is nowhere
near chapter 33:

| glyph | lessons using it | earliest |
|---|---|---|
| **ூ** (long-*ū* sign) | 5 | ch7, **மூன்று** |
| **ஏ** | 4 | ch5 |
| **ஞ** | 4 | ch10, **ஞாயிறு** |
| **ஐ**, **ஒ** | 3 each | ch7, **ஐந்து** / **ஒன்று** |
| **ொ** | 2 | ch8 |
| **ஸ** | 2 | ch1 |
| **ஃ**, **ஷ** | 1 each | ch7 |
| Tamil digits **௧**-**௰** | 1 each | ch7 |

Chapter 7's numbers alone account for thirteen of them. That, not chapters 33-38, is
where the strand's remaining debt actually is.

Neither **உ** nor **ஊ** has an entry in `tamil.json`, so neither gets a stroke order,
and the note that the shorter letter sits inside the longer one is marked as what the
page shows rather than something the data states.

Both new lessons are placed **after** the speaking lesson that teaches their word — `TA-C36-unavu`
at sequence 1080, `TA-C37-uur` at 1100 — because spelling a word the learner has not yet
been given inverts the strand's own rule. A first draft put a single combined lesson at
1055, which would have spelled both words before either was spoken and pointed
`reviews_of` at a lesson 45 sequence units ahead; `forwardReviews` caught the second
half of that. Splitting in two costs one six-lesson gap in the 3:1 cadence, which is
what waiting for the words costs.

