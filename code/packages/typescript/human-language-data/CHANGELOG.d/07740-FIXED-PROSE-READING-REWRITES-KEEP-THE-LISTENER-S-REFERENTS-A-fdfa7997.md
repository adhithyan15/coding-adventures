### Fixed — prose-reading rewrites keep the listener's referents and the book's punctuation

- Review follow-up to the drivable-prose entry below (issue #12070, tenth
  pass). Its rewrites cleared the prose check but lost things on the way: a
  deferred cue is still spoken, after "once you have stopped driving", yet a
  listener who hears a comment without the notice it comments on has lost the
  referent. The book printed some of the new cues badly too. 392 drivable
  lessons in all 23 tracks change again; every one stays `drivable: true`, and
  only the `core/lesson-modality` source hashes move.
- **The listener keeps the meaning.** 65 notice lessons in seventeen tracks
  had the whole notice inside the cue (`[YOU READ: the bill — **…** — …]`).
  The notice now stays in prose and only the look is deferred:
  `[YOU READ: the bill]`, then "The bill says **…** — …" (RU-C139-vkhod moves
  "you go in here" out of the cue the same way). 26 more lessons get back a
  premise, gloss or answer the first pass had put inside a cue, among them
  JA-C17-kokonotsu ("The first two signs of **ここのつ** spell **ここ** …"),
  KA-C75-on-a-sign, ML-C68-seventh and the Arabic, Persian, Urdu and Punjabi
  letter-by-letter readings ("Right to left: **ک** *k*, … [YOU READ: the word
  right to left]").
- **Spoken steps come back out of deferred cues.** TA-C66..C73 (eight
  lessons) again say "Before the new one, say what **வண்டி** means." before
  `[YOU READ: **வண்டி**]`, and FA-C02-esm-e-man's speaking step stands apart
  from its reading cue. HI-C06-numbers-1-5 and the two RU-C02 letter lessons
  drop "now" from a deferred cue and say "You'll draw them later" in prose.
  JA-R131..R137, where the first pass had dropped the reading itself, read
  `[PAUSE 2s each] [YOU READ: each one below aloud, then say what it means]`.
- **The book's punctuation.** 109 new cues that ran straight into prose or into
  another cue in the same paragraph now end their paragraph (105 lessons:
  "[YOU COVER: the page] The next question …" in MW-C06-hear-question, the
  31 Marwadi look-cover-write lessons, the Chinese, Japanese and Marwadi
  reviews). Three list items that held two cues become two numbered items
  (JA-C10-slower-please, MW-C35-read-ten-ticket, MW-R09-maternal-three).
- **No stutter.** 100 cues in 95 lessons that opened with *it*, *them*,
  *this* or *these*, printed as "*Read it:* them again", name their object
  ("the six lines again", "the passage again", "the notice, then answer",
  "**儿子** once without pinyin", "[YOU COVER: **人**, then wait five
  seconds]"). 27 four-skill items drop the "**Reading:**" label in front of a
  READ cue, which printed as "Reading: *Read it:* …".
- **Plainer phrasings.** "Leave the lessons before this one closed. Say …"
  → "From memory alone, say …" in 61 Malayalam, Spanish and Marathi reviews,
  and is dropped where the next sentence already carries it (ML-R70, ML-R71,
  ML-R75, MR-R69), as are TE-C06's "Leave the book closed on everything so
  far." and MR-R05's "Leave the chapter closed.". GE-C09-zahlmonate, SA-C63-first,
  FA-C01-practice ("Every model stays in view." dropped), JA-C08-hear-sayounara
  and MW-C07-hear-later ("[YOU COVER: the text]" then "Then the new expression
  begins.") and the Chinese meaning-card rewrite ("Hear four meanings in mixed
  order: … Say each. [YOU READ: each Mandarin form]") follow the review's
  wording.
- **Reading and checking left in prose.** PA-R158-food-quality-recall and
  PA-R158-truth-and-food-check "Check: **ਸੱਚਾ** …" → "Hear the answers: …";
  ZH-R18-book-reading-r1 and ZH-R18-looking-three-r1 "Check …" → `[YOU CHECK:
  …]`; MW-C35-ticket-four "Given a stall with six labelled goods, say …" →
  `[YOU READ: the labels on a stall of six goods, then say …]`; ES-C391-euro
  "Open the Academy's dictionary at *euro* and find two entries" → "The
  Academy's dictionary has two entries at *euro*".
- **The prose check** (`tests/drivable-writing-imperatives.ts`) takes " or "
  as a step link ("turn the page or cover every Arabic model"), "cover up" as
  a cover, and "cover and wait / write / say" as a bare cover step, each a
  closed literal. Over the pre-fix corpus that leaves the drivable count at 418
  spans and adds one non-drivable span (AR-W00-full-greeting-recall, now 740
  spans in 603 lessons). A bare "read," in a chain ("hear, say, read, and
  write") and "cover, then" were measured and left out: the first is as often a
  list of skills (a heading included) as a step, the second matches a stroke
  the Chinese writing lessons call "cover". The module comment records both
  gaps. Seven new cases: four positives, three controls, and three ~50k
  adversarial inputs that assert only the answer.
- **Correction.** The first pass's commit message said ML-C102-manikku and
  MR-C62-pahila-paath "were trimmed by one word each" to fit the 300-second
  duration budget. ML-C102-manikku was reworded, not trimmed: "read it **at**"
  became "it means **at**", the same length. This pass's named objects pushed
  four reading lessons one second over that budget, so their cues shed two to
  four words: MR-C62-pahila-paath "do not translate as you go" → "do not
  translate", HI-C78-pehla-paath "once without stopping; let the sentences
  arrive without translation" → "once, without stopping or translating",
  LA-C59-social-exchange "again and find the two things handled" → "again for
  the two things handled", AR-C42-first-passage "again, and this time notice
  the hinge" → "again for the hinge". No declared duration changed.
