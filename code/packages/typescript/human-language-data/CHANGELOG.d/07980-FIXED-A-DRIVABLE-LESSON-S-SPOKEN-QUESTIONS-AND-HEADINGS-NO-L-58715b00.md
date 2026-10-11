### Fixed — a drivable lesson's spoken questions and headings no longer ask a driver to read, write, type or point

- **The defect (security review of the previous entry, driver safety).** The
  eyes-or-hands detectors read a section's body as `narratedParts` walks it,
  and that walk drops every HTML comment — which is where a scored activity
  lives (`<!-- hl-activity: … -->`). The narration does not drop it: it says
  the prompt as "[question — say your answer, then pause N seconds]" followed
  by the prompt. Headings were not read either, by design, yet the narration
  speaks every one. Measured over the drivable parts (every section not set
  aside behind a stop guard) of the 27,560 core-drivable lessons:
  - **activity prompts:** 3,103 spoken, of which **235 needed eyes or hands**
    — 222 by the existing prose checks run over the prompt text (163 open
    "Write …": Marwadi 69, Japanese 45, Chinese 27, Marathi 8, Spanish 8, …;
    "Read 汉字 without pinyin, then give its meaning"; "From the picture cue of
    a stream, write …"), 10 that typed a script ("Type 'a car' in hiragana",
    "Type the Mandarin character for person") and 4 that read the script they
    display ("Read this shuffled pair and give each meaning: આજ — રાત"). By
    track: Marwadi 109, Japanese 56, Chinese 39, Marathi 11, Spanish 9,
    Persian, Punjabi, Telugu, Gujarati, Russian, Urdu. A further **1,312**
    prompts said "Type …" with an answer a driver can say (Japanese 643,
    Marwadi 546, Persian 50, Punjabi 40, Urdu 23, Chinese 6, Latin 3,
    Bengali 1).
  - **headings:** 95 spoken with no guard in front of them that open a step
    with an eyes-or-hands verb — 50 section titles (31 Japanese "Guided
    Practice — read, copy, recall", "Guided Practice — write", "Guided
    Practice: Read them back", "Sounds you'll need: Read the cedilla") and 45
    lesson titles (34 Spanish "(el aviso de entrega) — read it, then do it
    yourself", "Practice — read the numbers off a wall", "Read two, say
    one"). The prose checks saw only two of them: they were built for
    sentences with objects, not labels. Every set-aside section's title was
    also spoken one line BEFORE its stop guard ("Script you'll notice: Read
    ன்னி." then the guard).
- **The activity schema could not tell them apart.** `kind` is always `text`
  (the app shows every prompt above a text box), so the prompt's words are the
  only signal. `src/drivable-instructions.ts` gains
  `activityPromptNeedsEyesOrHands`: every existing check on the prompt text,
  plus two prompt-only shapes — `promptTypesInAScript` (a typing step whose
  answer's form is a script: "in hiragana", "in characters", "the character
  for") and `promptReadsTheShownScript` (a reading step with a determiner or
  "and" for its object, in a prompt holding unromanized script; not "Read in
  the falling fourth tone, 好 …", not Latin-script material).
- **Deferred, like a cue, rather than counted against the core.** A prompt
  that needs eyes or hands cannot be said for the ear without grading a
  different skill under the same activity id, and the app still needs it as
  authored. So the narration defers it exactly as it defers a `[YOU WRITE: …]`
  cue: `NarrationActivity` gains `spoken: boolean`; a deferred question is
  rendered "[once you have stopped driving — question: Write son in Chinese
  characters.]" with no answer gap, and its section joins the notice's
  `waitUntilStopped` and gets the stop guard. As with deferred cues, the
  lesson's core stays `voice`: nothing is asked at the wheel. Corpus-wide 639
  questions in 589 lessons are now deferred (282 in 271 core-drivable
  lessons).
- **Typed words are said for the ear, in the renderer.** New
  `src/for-the-ear.ts`, `activityPromptForTheEar`: "type" where a step starts
  becomes "say" in the same case ("Type the Persian word for 'tea'." → "Say
  the Persian word for 'tea'."), and a closed list of typist-only spelling
  phrases is dropped (", in rōmaji", ", in pinyin with its tone mark,", ",
  with its tone mark,", "In pinyin, ", "romanized", "its romanization" → "it").
  The lessons are not reworded, on purpose: "Say …" above a text box would be
  a worse instruction in the app than "Type" is in the car — each medium gets
  the verb it can do, as `[YOU WRITE: …]` already prints "*Write it:*" in the
  book and "once you have stopped driving — write:" in the narration.
- **Headings are said for the ear.** `headingStepIndex` finds an
  eyes-or-hands verb where a heading step starts (the start, ": ", " — ",
  ", ", " then ", " and ", " or "), with or without an object, Unicode-aware
  ("tapēte" is not "tap"), and not before a copula. `headingForTheEar` keeps a
  matching heading's LABEL ("Guided Practice — read, copy, recall" →
  "Guided Practice", "(el mensaje del vecino) — read it, then answer" →
  "(el mensaje del vecino)"); a lesson title that is all step falls back to
  the headword, then the gloss. A lesson title of the shape "headword —
  gloss" keeps its gloss ("ಬರೆ (bare) — write") when the head holds the
  headword AND the tail is the lesson's gloss. The renderer judges the
  title-plus-gloss line it composes the same way ("(una oferta y una
  despedida) — the payoff"). This applies only to lessons whose core is
  `voice`; any other lesson's notice has already said it is not a driving
  lesson, and keeps its titles as authored. Modality still does not read
  headings: this is a rendering rule and moves no modality.
- **The stop guard comes before a set-aside title.** `renderLessonNarrationText`
  speaks `STOP_GUARD` and then the title, so a driver who skips ahead skips the
  section's own words too. (The notice still names set-aside sections by their
  authored titles, inside "leave these sections until you have stopped: …".)
- **Tests.** `tests/for-the-ear.test.ts` (new): both rewrites case by case,
  controls, linearity. `tests/drivable-instructions.test.ts`, new part 5: the
  activity and heading detectors' positives and controls; the narration on
  synthetic lessons (deferral, notice and guard, "Say …", label-only titles,
  non-drivable lessons untouched, guard before a set-aside title); and a gate
  over the COMMITTED narration of every track — no prompt or heading spoken in
  a drivable part of a core-drivable lesson asks to read, write, type or point,
  every activity carries a verdict, and every deferred one is said as a
  deferral. Non-vacuity is synthetic, not debt: into a fixed sample of real
  core-drivable lessons (every 250th by id), each shape is planted — a writing,
  script-typing and script-reading question, a "Type the word for …" question,
  a "Guided Practice — read, copy, recall" title — and the renderer's output
  must pass the gate while the old output (asked plainly, typed, or said
  whole) must fail it. `tests/narration.test.ts` and the classifier test now
  pin guard, then title, then content.
- **Regenerated.** Narration for every track (the guard moved in every
  lesson with a set-aside section: 1,751 narration files and 1,200 hash shards). Books, modality, figures:
  unchanged.
- **Follow-up security review: the prompt check failed OPEN, and the gate could
  not see it.** `activityPromptNeedsEyesOrHands` reused the PROSE writing
  detector, which is tuned to under-fire (a fronted phrase must be plain ASCII
  words, "after" and "give" open nothing, a verb before "is" reads as a
  subject). In a scored prompt every writing verb is a request, and 51 of them
  were still asked as live questions in kept sections of core-drivable lessons:
  Marwadi 41 ("From bachchā, write the taught word.", "After hiding the model,
  write kāĩ and give its meaning."), Chinese 5 ("After all four checks pass,
  write home, person, and family members.", "Write is that so, without
  pinyin."), Japanese 3 ("From the heard morae i | chi | do, write once in
  hiragana.", "… how many morae do you tap?", "Which kanji have you been able
  to write …"), Marathi 2 ("Hear dhanyavad; give its meaning and write it in
  Devanagari."). The corpus gate called the same detector, so it agreed.
  - **New fail-safe check, `handVerbsInPrompt`** (reason `hand-verb`): any hand
    verb — write, rewrite, jot, copy, trace, draw, sketch, circle, underline,
    tick, check off, cross out, highlight, fill in, fill out, select, point,
    look at, tap, clap — as a whole word anywhere in the prompt defers it,
    unless that occurrence is one of five closed shapes: a quoted gloss
    ("for 'to write'"), an infinitive gloss ("meant to write"), a noun (a
    determiner before it, or "of" after it), a third-person question ("Does it
    point to …", not "Do you …"), or copy + a sound word. Each excuse reads a
    fixed window, so the check is linear. "type", "read", "mark" and "label"
    stay with their own shapes (see the source header for why).
  - **Every check reads the prompt with emphasis dropped too**, so `*Type*`,
    `_write_` and `Read **this**` are the bare words; and the ear rewording now
    runs on the Markdown-stripped prompt, so an italic "Type" is said "Say".
  - **Emphasis openers in the prose and heading detectors**: the step links
    (`SENTENCE_START`, `CLAUSE_LEAD`, `PROSE_LINK`, `HEADING_LINK`) accept `*`,
    `_` and `__` as well as `**`; the mention guards accept `_Write_ is …`;
    `WRITING_VERB` no longer ends at `\b` (which never fired on `_write_`); a
    prose "read" may be followed by a closing `*` or `_`. This changed no verdict
    in the corpus; it closes the shape.
  - **Deferred now:** 792 questions in 740 lessons (was 639 in 589), 334 in 323
    core-drivable lessons (was 282 in 271). Nothing that was deferred is asked.
  - **`headingForTheEar` trims a label's trailing joiners with a linear walk**
    instead of `replace(/[\s,;:—–-]+$/u, "")`, which was quadratic on a long
    run of those characters that stops short of the end (about six seconds at
    100,000 commas).
  - **Tests.** The six review prompts as positives; every excused shape with a
    control that differs only in what the excuse keys on; emphasis cases for
    prose, headings and prompts; linearity at 200,000 characters. And part 6
    of `tests/drivable-instructions.test.ts`, a gate that shares nothing with
    the generator: a plain whole-word scan for the hand verbs over every asked
    question and every unguarded section title of every core-drivable lesson in
    the committed narration, with an exact (lesson id, text) allowlist of 25
    entries, each with its reason — 9 questions (glosses such as "Say the
    Japanese for 'to write'", "Does it point to my or your?", "a letter-for-
    letter copy of tenuī") and 16 titles (nouns such as "that's the point",
    third-person verbs such as "-able and -ible point two ways", and labels
    naming writing as a topic such as "Grammar Lens: two ways to write it").
    Stale entries fail. The six review questions are pinned `spoken: false` in
    the committed narration, and non-vacuity is a question and a title planted
    into real lessons' kept sections.
  - **Regenerated:** narration for the affected tracks (176 narration files, 88
    hash shards). Books, modality and figures unchanged.
- **Second follow-up security review: an answer can be written without a hand
  verb, and four excuses were too loose.** Eleven questions were still asked
  live in kept sections of core-drivable lessons, and one review title was
  spoken as an opening line:
  - Marwadi 3 — "Refuse an offer and then leave, in two written lines.",
    "Refuse the offer, then leave the door open — two written lines.",
    "Someone says das karo. Refuse in one word, in Devanagari." (assesses
    `MW-SCRIPT-KONI-01`); Japanese 4 — "Record the retrieval string: e, five,
    mouth, please say it, hair, eye." (accepts only kana and kanji), "Which
    hiragana sign becomes が when the dakuten is added?", "How many beats does
    san have, and what is the second one?", "Two of these five were already
    yours …" (each grades a script atom with a script answer); Punjabi 1 —
    "Which already-taught Gurmukhi consonant is na: ਨ or ਸ?"; Chinese 1 —
    "Which taught character is the Level 1 word female or woman?" (accepts
    only 女); Persian 1 — "In خواندن (khândan), which written letter is not
    pronounced?"; Urdu 1 — "Type the Urdu word for 'nose' — the same three
    letters as کان (kān), reversed."; and the Japanese title "Retrieval 6 —
    greet, repair, point", spoken unguarded because the heading check knew
    only "point to / at / and".
  - **New prompt shapes** (`activityPromptNeedsEyesOrHands`): `written-answer`
    (`promptAsksForAWrittenAnswer`: "in writing", "on paper", "written
    line(s) / form / answer(s) / sentence(s) / response(s) / reply / version",
    or in / into / using / as + an optional "the" and one word + a script's
    name — Devanagari, Gurmukhi, Shahmukhi, Nastaliq, Naskh, Perso-Arabic,
    Cyrillic, Hangul, kana, hiragana, katakana, kanji, hanzi, characters,
    script; it fires on a spoken fact such as "In Devanagari, ক is …" too, on
    purpose) and `choice-among-shown-glyphs` (`promptChoosesAmongShownGlyphs`:
    two unromanized runs of a non-Latin script joined by "or", "/" or "vs").
    The hand verbs gain `record`, `note down` and `transcribe`.
  - **New structural rules** (`activityNeedsEyesOrHands`, which the narration
    now defers on): `accepts-only-script` — every accepted response holds a
    letter of a script that is not Latin, Common or Inherited, so nothing a
    driver can say is accepted; `grades-script-glyphs` — the activity assesses
    a `-SCRIPT-` atom and its canonical answer is script. The script-atom rule
    deliberately stops there: 35 spoken questions assessed a script atom, and
    most are knowledge about a script answered by ear ("Say the four velar
    letters in order" — ka kha ga gha); their canonical answers are words.
  - **Excuses tightened** (`handVerbsInPrompt`): a quoted gloss no longer
    follows a colon ("Follow this: 'You trace …'") and its closing quote must
    end a word (the apostrophe of "letter's" is not one); an infinitive gloss
    ("means to write") must end its sentence ("… means to write it: do it in
    Devanagari" fires); a third-person question needs its auxiliary to open
    the sentence, optionally after a question word ("What does one speaker
    write …?"), and the sentence to end in "?" ("Do it again and copy the
    sign", "If you can now write it …, do so" fire), with "please" never a
    subject; copy + a sound word is no excuse when "mark(s)" follows ("Copy
    the tone marks", "the stress mark", "the pitch accent marks") — the same
    guard is added to `WRITING_VERB` and the heading verbs. The sentence-end
    lookup is one precomputed pass, so the scan stays linear.
  - **Headings**: a bare `point` ending a step ("…, point", "point, then …")
    now counts; "point every verb at somebody" and "that's the point" still
    do not, and a headword–gloss title keeps its gloss.
  - **Deferred now:** 912 questions in 857 lessons (was 792 in 740), 345 in
    333 core-drivable lessons (was 334 in 323). By track, all lessons:
    Punjabi 57, Japanese 23, Chinese 10, Marathi 10, Marwadi 8, Hindi 4,
    Telugu 3, Arabic, Gujarati, Persian, Russian, Urdu 1 each — mostly
    writing-track questions ("Add the voicing mark to て. Type the sign you
    get.", "Repair only the spacing in ਕੰਮ:ਨੌਕਰੀ.") that accept only script.
    Nothing that was deferred is asked.
  - **Tests.** Every review example as a positive, each tightened excuse
    against a control that differs only in what it keys on, the structural
    rules with romanized and word-atom controls, linearity at 100,000
    characters. Part 6, the independent gate, now also scans written answer
    forms (its own literal copy), and for each asked question its accepted
    answers and assessed atoms straight from the committed JSON (its own
    script-letter test, written as a `v`-flag set difference rather than the
    generator's lookahead), glyph choices, and the opening line of every
    lesson whose title is not a headword title (1,676 of 27,560 core-drivable
    lessons). The allowlist grows by four, each with its reason: two section
    titles that name a written form or "a record" as a topic, two review
    openings that describe ("the two letters that write the last of them",
    "somebody to point at"). Non-vacuity: each new shape planted alone into a
    clean question in real lessons, plus a review opening and a headword
    title as its control.
  - **Regenerated:** narration for the affected tracks (106 narration files,
    53 hash shards). Books, modality and figures unchanged.

The glyph-choice pattern uses a one-character lookbehind (`(?<![\p{L}\p{M}])`); the earlier variable-width `(?<!\p{L}\p{M}*)` walked back over runs of combining marks at every position and went quadratic (53 s on 40,000 bare accents). Same answers on real text; covered by 100k-character mark-run cases.
