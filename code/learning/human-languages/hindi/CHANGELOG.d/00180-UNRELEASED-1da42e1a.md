## [Unreleased]

### Added - 33 more pre-A1 words, chosen against the A1 exam inventory: 176/282 -> 192/282

Seven chapters (**75-81**), one new headword per lesson. Thirty-five words were
authored against chapters 68-74; #14113's joining tranche merged into those same
seven numbers first, so this tranche moved to 75-81 and two of its words were cut
rather than taught twice. A1 exam coverage moves **176/282 (62%) ->
192/282 (68%)**, measured with `measureExamCoverage` on the MERGED tree, before
and after, never composed from the two branches' separate claims.

TWO WORDS CUT, NOT RENUMBERED

Both tranches were chosen against the same list of uncovered inventory points,
and both closed HI-A1-Q-07 (`kab`) and HI-A1-Q-08 (`kyon`/`kyonki`). Introducing
a headword twice is a hard error, so `HI-C71-when` (कब) and `HI-C71-why` (क्यों)
are deleted and those two points stay #14113's:

  - it teaches `kab` inside the ka-/ja-/ta- deixis system beside `jab...tab`
  - it gives `kyonki` a grammar lesson of its own rather than a paragraph

Chapter 78 therefore teaches three words, not five, and its payoff is
`HI-C71-very`. `HI-C72-money` and `HI-C72-rupee` reviewed the two cut lessons;
they now reach back to आज, भी and बहुत, which is where the tranche's own
cross-chapter rule points anyway and makes "two words back" true again.

There is no atom-id collision between the two tranches. The clash was at the
headword, which is the one that matters.

LESSON IDS DID NOT MOVE

`HI-C68-*` through `HI-C74-*` keep their ids in chapters 75-81, the way
`ES-C03-*` lives across Spanish chapters 4-6 and `GE-C15-*` across three German
chapters. What moved: the chapter assignment, the sequences (2610-2950 ->
2830-3150, after #14113's 2820) and the curriculum shard prefixes. Every derived
artifact -- chapter `.tex`, narration, both hash ledgers, the modality shards and
the gentle-ramp snapshots -- was REGENERATED, none hand-merged.

WHAT IS TAUGHT

    75  MEET-GREET              desh, Bharat, shahar, bhasha, angrezi
    76  POLITE-REQUEST-REPAIR   gari, rel, steshan, tikat, jana
    77  EXCHANGE-NAMES          khel, khelna, kriket, gana, aram
    78  CHECK-WELLBEING         aaj, bhi, bahut
    79  RESPOND-BASIC           paisa, rupaya, dam, mahanga, kharidna
    80  COURTESY-THANK          hotal, nashta, bil, vetar, pina
    81  TAKE-LEAVE              khula, band, pravesh, nikas, der

SIXTEEN EXAM POINTS CLOSED, NOT EIGHTEEN

The pre-merge branch claimed eighteen. Two of them are now #14113's, and the
honest figure is sixteen. Core lexis 35/65 -> 43/65, adverbs 7/10 -> 9/10, the
verb 14/27 -> 16/27, communicative functions 49/65 -> 51/65, the noun 6/9 -> 7/9,
the adjective 5/9 -> 6/9. The denominator did not move.

Still closed, and named in an earlier entry as the track's worst holes: **`jana`
is taught** (HI-A1-V-25, "the single highest-traffic missing verb in the level"),
**`aaj` is taught** (HI-A1-ADV-03), and **the public-sign vocabulary exists** --
`khula`, `band`, `pravesh`, `nikas` are items 1 to 4 of BOTH mocks' reading
papers (HI-A1-F-60, HI-A1-LEX-62).

The verb criterion closed. Hindi taught **2** distinct verb headwords at or below
pre-A1 against a target of 5; `jana`, `khelna`, `kharidna` and `pina` bring it to
**6**, and `verb-vocabulary/hindi/pre-A1` has left the completion plan. Those four
are canonical `VERB-GO`, `VERB-PLAY`, `VERB-BUY` and `VERB-DRINK`, so the
cross-language verb join sees Hindi go 13 -> 17 of 40 covered and the corpus mean
39% -> 40%.

REINFORCEMENT WENT DOWN WHILE 33 LESSONS WENT IN

Measured against `origin/main`'s data with the same CLI, not derived:

    hindi pre-A1, completion plan       main    now
      exam-point (uncovered)             106     90
      vocabulary (shortfall)             145    112
      verb-vocabulary                      3   closed
      reinforcement (thin atoms)          54     51
      script-closure                      11     11

Whole-track atoms revisited fewer than twice: **132 -> 129**. Not one is ours;
three pre-existing atoms were rescued, and the two of #14113's that a longer book
made judgeable for the first time -- `HI-JOIN-TAG-NA-01` and
`HI-SOUND-QUESTION-RISE-01`, the last two lessons before this tranche -- are
closed by chapter 75's opening two lessons reaching back across the boundary, the
same rule this tranche applies at every other chapter boundary. `atomsTaught`
376 -> 409, `atomsNeverRevisited` 79 -> **78**.

`reinforcementWindowMisses` rises 1071 -> 1155, and the +84 splits **41 this
tranche's own atoms, 43 pre-existing atoms newly judgeable** -- a longer track
makes windows fit that the end of the book had previously cut off. All 41 are R2
and R3; **R1 does not move at all (145 -> 145)**, because a one-word-per-lesson
chapter retrieves at distances 1-4 and never at 5-15. That is a property of the
chapter shape rather than of these lessons, it applies equally to the Tamil,
Kannada and Marathi tranches, and HL-C313 records it with the fix and the reason
it was not made here.

ONE FORWARD REFERENCE FOUND AND CUT

Teaching `dam` made `HI-A1F01-name-label` a forward reference: its shape drill
"find नाम among काम · नाम · दाम" used a word no lesson taught, and this tranche
teaches it 48 lessons later. The distractor is now **शाम**, taught in chapter 29,
which is a better drill -- the reader discriminates among three words they hold
rather than two and a stranger. Forward references stay at **11**, unchanged from
main.

Script closure, never-taught glyphs, writing-practice lessons, the atom and glyph
step findings and the payoff-surprise finding are all byte-identical to main.

PINS MOVED

    tests/corpus/hindi.test.ts   lessons 310 -> 343
    tests/verbs.test.ts          meanCoveredPercent 39 -> 40

The Hindi book compiles under XeLaTeX at **545 pages** with errors, overfull,
underfull and missing-character counts all zero, and the changed pages were read
as rendered PDF.

### Added - the joining column: 155 -> 176 of 282 A1 points

**Chapters 68-74 teach Hindi's joining words, and A1 exam coverage moves
155/282 (55%) -> 176/282 (62%).** Measured with `measureExamCoverage` against
`core/exam-inventory-hindi-a1.json`, before and after, on the tree - not
arithmetic on the deltas.

- **`Samuchchay (joining clauses)` goes 0/6 to 6/6, and `Prashn (asking
  questions)` 5/10 to 10/10.** Fifteen items closed twenty-one points, because
  they were chosen against the inventory rather than by topic: six of the
  twenty-one fell in the communicative-function column the tranche was not
  aimed at (asking a preference, saying what you want, asking about ability,
  saying why you cannot do something, asking when something happens), and one in
  pronunciation.
- **What is taught, one item per lesson:**

  | ch. | items |
  |---|---|
  | 68 | *aur* and - **या** or - **लेकिन** but |
  | 69 | the verb comes last - **कि** that |
  | 70 | **सकना** can - **चाहना** want - the -ना form as an object |
  | 71 | **क्यों** why - **क्योंकि** because |
  | 72 | **कब** when? - **जब … तब** when/then |
  | 73 | **-कर** having done |
  | 74 | **क्या …?** the polar particle - the rising voice - **… ना?** the tag |

- **`aur` is taught GLOSS-FIRST, and the letter debt is written into the
  lesson.** Its opening vowel is the independent *au*, which no Hindi writing
  lesson has ever drawn and which the corpus shows exactly once, inside
  *aurat*. Its Devanagari therefore appears only in the headword, where the
  romanization exemption makes it exposure; the body is entirely romanized and
  says in as many words that the letter is owed. The word carries a clause join
  in five of the six mock profile texts and is far too load-bearing to wait for
  a script lesson. This is the shape the Marathi tranche used for **सांगणे**.
- **Every other item is spelled from Devanagari the corpus already teaches.**
  The candidate list was filtered against the track's taught-glyph set before a
  word of prose was written, and the first draft still had seventeen closure
  violations - **झ** inside *mujhe*, **छ** inside *acchā*, **ौ** inside *kaun*
  - every one of which was rewritten around rather than waved through.
  `scriptClosureViolations` and `neverTaughtGlyphs` are unchanged at 38 and 11.
- **Four spine omissions close.** `VERB-CAN` (HI-C70-sakna), `VERB-WANT`
  (HI-C70-chahna), `CONNECTIVE-BECAUSE` (HI-C71-kyonki) and `QUESTION-POLAR`
  (HI-C74-kya-polar). `SPINE-SAY-WHY` and `SPINE-SAY-WHAT-I-WANT` now omit
  nothing at all.
- **The tranche's own reinforcement debt is zero**, and `atomsNeverRevisited`
  FALLS 81 -> 79: the two atoms rescued are ones the older track had nothing
  later to revisit them with. `reinforcementWindowMisses` rises 1035 -> 1071,
  and every one of the 36 is pre-existing debt this tranche EXPOSED rather than
  created - twenty-three more lessons made reinforcement windows fit that the
  end of the track had previously cut off. Not one of the 36 is on a lesson in
  this tranche.
- **Chapter 74 was split rather than crammed.** The intonation question started
  as a section of the **क्या** lesson and pushed its COMPUTED duration to 308
  seconds against a 300-second ceiling. Trimming prose moved it by four seconds
  and made the lesson worse; lifting it into its own lesson - one new item per
  lesson - fixed it and is the better book.
- **Left uncovered, with the reason written into the inventory:**
  `HI-A1-V-21` (*chahiye* takes a noun where *chahna* takes a verb, and it needs
  the independent vowel **ए**, which no writing lesson has drawn - a script
  lesson as much as a vocabulary one) and `HI-A1-F-39` (the asking half now
  exists; *janna* still has no atom, and half a pair is not the point).

### Added - an A1 exam inventory, measured against a proxy with a real syllabus behind it

`core/exam-inventory-hindi-a1.json` enumerates **282 things an A1 Hindi
candidate must be able to do**, each probed by the knowledge atoms whose
presence would demonstrate the track teaches it. Hindi is the first track
outside Spanish, French and German to have one, and the first Indic track whose
exam gap is a measurement rather than a proxy.

**The number: Hindi covers 155 of 282 points (55%).** Zero points are partly
taught - every probe names atoms that actually exist - so the shortfall is 127
points with nothing in the corpus corresponding to them. By category:

    44/65  Sanvad-karya (communicative functions)
    35/65  Shabd-bhandar (core lexis)
    14/22  Devanagari lipi (script and orthography)
     7/10  Kriya-visheshan (adverbs)
     6/9   Samay aur tarikh (time and date)
    11/27  Kriya (the verb)
     5/10  Sarvanam (pronouns)
     3/10  Parsarg (postpositions and case)
     1/5   Vakya (the sentence)
     0/6   Samuchchay (joining clauses)

The shape is the finding. Hindi greets, thanks, apologises, introduces itself
and names a great deal of the world. It cannot join two clauses at all - `aur`,
`ya`, `lekin`, `ki`, `jab` and `jo` are none of them taught - and it cannot say
where it is from (`se`), where it will be (`par`), when anything happens
(`kab`, `X baje`), what anything costs, what day it is (`aaj` is untaught while
`kal` is), or how it gets anywhere: **`jana` is introduced nowhere**, and every
personal-account item in both A1 mocks uses it.

**Why the target list is 282 and not 172.** A first draft of this file built
its point set from the CEFR Companion Volume's A1 descriptors and reached 172
points, measuring 67%. Descriptors are deliberately abstract where a syllabus
enumerates, so that ruler was short and the number flattered the corpus. The
point set is now derived structurally from `core/exam-inventory-es-a1.json`,
which restates the inventory the Instituto Cervantes publishes behind DELE - a
real awarding body with a published, finite, A1/A2-split syllabus - used as a
**proxy for level**: it is asked what an A1 learner must *handle*, not what
Spanish grammar it names. All 273 proxy points are accounted for, 264 mapped
and 9 named as Spanish typography with no Devanagari counterpart, and the
generator refuses to emit the file if any is left unaccounted.

The proxy earned its keep. It surfaced demands no descriptor and no
mock-reading had named: that nothing tells a learner Hindi **has no article**;
the reflexive possessive **`apna`**, which both mocks use; object-marking
**`ko`**, the exact counterpart of Spanish's `a` before a human object; and
whole lexical domains the earlier draft never enumerated - transport, payment,
free time, media, written correspondence, documents, educational institutions.
It added 110 points and removed none.

**Nothing is attributed to anybody.** DBHPS publishes the names of its
examinations and the prescribed readers, and no content syllabus - no grammar
inventory, no word list, no can-do descriptors, no A1/A2 split. Kendriya Hindi
Sansthan and the Central Hindi Training Institute publish none either, and no
Council of Europe Reference Level Description exists for Hindi. The file says
**NOTHING IN THIS FILE MAY BE ATTRIBUTED TO DBHPS** and, equally,
**ATTRIBUTE NOTHING TO DELE OR THE INSTITUTO CERVANTES ABOUT HINDI**: they
published a Spanish syllabus and have said nothing whatever about this
language. Every Hindi exponent is an editorial judgement, all four `scope`
dimensions are `partial`, and the inventory is deliberately NOT complete.

A proxy is a scaffold, not a template to translate. **22 points are
`hindi-specific`** and answer no Spanish demand at all: the Devanagari script
points, the postposition and oblique-case system, the nuqta, the
Sanskritic/Perso-Arabic register split, the `karna` compound verbs, and the
abstract courtesy nouns the track teaches a whole chapter of. Every point
carries a `derivedFrom` field naming the proxy points it answers or marking it
Hindi-specific, so the derivation is auditable rather than asserted.

Two gaps are worth naming here because they are cheap to close:

- **Eight Hindi writing lessons teach something and declare an empty
  `introduces` list**, so they contribute no atom and cannot be probed.
  `HI-W06-name-sentence-stop` teaches the danda and `HI-W06-two-sentence-card`
  teaches the 30-40 word message to a named reader, which is 60 of the A1
  writing paper's 100 points. Fixing one frontmatter field per lesson would
  make both points measurable without authoring any content.
- **The nuqta is never taught, and the corpus's own headwords are full of it** -
  shukriya, khushi, zarur, safed, darvaza, mez, sabzi, zyada, safar, tohfa,
  bukhar, fasal, khushbu, mulaqat. Every corpus-internal metric counts those
  lessons as taught; only an external target list surfaces the mark itself.

`npm run plan` now names Hindi exam-point work - *cover 127 of 282 A1 exam
point(s) hindi does not teach* - where it previously reported Hindi among the
tracks that could not be measured at all. The plan's exam-inventory line moves
to *0 complete and 5 partial of 138*, and the unmeasured remainder falls from
twenty tracks to nineteen.

### Changed — 70 Hindi headwords stop being load-bearing script

Seventy lessons printed their headword in Devanagari and declared no
`romanization`. Every one of them already *said* the word in romanization —
in its own title line, its gloss, or its body prose — so the reader was never
actually stranded. The structured field was simply empty, and the field is the
contract: `script-closure` reads it, the narration exporter reads it, and the
book's glossary and running heads read it. An empty field made **नमस्कार**
count as a glyph the reader must decode unaided, when the page beside it said
*namaskār* all along.

Each lesson now declares the romanization **its own prose already teaches** —
transcribed, not invented, and matching that lesson's chosen orthography rather
than a house style imposed over it (so Chapter 26 keeps *raat* and Chapter 29
keeps *shaam*, the forms those lessons print). Multi-word headwords keep their
own separators: **आप / तुम** → `āp / tum`, **मुझे … पसंद है** → `mujhe …
pasand hai`.

Three downstream artefacts move with it, all in the reader's favour:

- **Narration.** A voice-only learner previously heard `read "नमस्कार"` — an
  instruction naming a string they cannot pronounce. It now reads
  `read "नमस्कार (namaskār)"`. This is HL08's point, and it was silently
  failing for seventy lessons.
- **The book's table of contents.** Section short-titles were bare Devanagari,
  so the ToC was unnavigable to a reader still learning the script. They are
  now the romanization.
- **The glossary.** Those seventy entries gain their romanization and sort into
  Latin order beside the rest, instead of clumping by Devanagari codepoint.

Measured on the corpus report: Hindi's load-bearing headwords fall **71 → 1**,
and Hindi's script-closure violations fall **69 → 40** — the single largest
exposure seam in the corpus, and Hindi was the worst track in it. Corpus-wide
the exposure count falls 283 → 213 and closure 643 → 614; every point of both
deltas is Hindi.

**One lesson is deliberately left load-bearing.** `HI-A1F01-name-no-model` is
a no-model writing checkpoint whose own prompt reads "With no bank,
romanization, or copyable answer, complete the Hindi name field." Its
withheld romanization *is* the assessment condition. Declaring the field to
drive the count to zero would have leaked support into the narration of a
lesson built to remove it, so the remaining 1 is intended and should stay.

Declaring the field also exposed two places where a lesson glossed the same
word twice, in two spellings, because the narration exporter's de-duplication
is an exact-substring test and the new field near-missed the body's existing
gloss. Both were repaired at the source rather than by loosening the field:

- **Chapter 4** was the track's lone romanization outlier, writing च as *c*
  (*caltā*, *calnā*) where every other lesson writes *ch* (*chār*, *chāy*,
  *chammach*, *sochnā*) — and where its own title, slug, roadmap and
  session-map entry all said *chaltā*. Its Hindi word forms are now *chaltā /
  chaltī / chalnā*. The Sanskrit root (*cal-*, *calan*, *cañcal*) stays in
  IAST because it is a Sanskrit citation, and the letter name **च** (*ca*)
  stays as `HI-S117-letter-ca` teaches it: the track romanises *letters* in
  IAST and *words* for the reader, and that split is deliberate.
- **Chapter 2's practice table** glossed **आपका नाम क्या है?** as
  *āpkā nām kyā hai*, dropping the question mark the Hindi cell carries. It
  now matches.

The 164-page figure in this track's README was also stale: the book now
compiles at **454 pages**, still with zero missing characters and zero errors.

### Changed — declare both Hindi apology registers

- Chapter 9 now declares **माफ़ कीजिए / क्षमा करें** instead of naming only
  the everyday apology. The lesson already defines **क्षमा करें** as the formal,
  Sanskritic cousin, contrasts its register with **माफ़ कीजिए**, practises the
  complete phrase, and asks learners to recall it; its metadata now exposes both
  assessed apologies.

### Changed — declare the Hindi weekday building block

- Chapter 10 now declares **वार** alongside **सोमवार** through **शुक्रवार**.
  The lesson already defines **वार** as “day,” uses it as the productive second
  half of every weekday, practises the `[deity] + वार` pattern, and asks learners
  to recall both the form and its meaning; its metadata now exposes that assessed
  standalone word.

### Changed — declare the Hindi gain noun

- Chapter 35 now declares **लेना / लाभ** as its headword instead of naming only
  the verb “to take.” The lesson already defines **लाभ** as “profit, gain,
  benefit,” uses it to show the consonant lost by **लेना**, and asks learners to
  recall both the noun and why it preserves **भ**; its metadata now exposes that
  assessed standalone noun.

### Changed — declare the complete Hindi woman-register set

- Chapter 39 now declares **औरत / महिला / स्त्री** as its headword instead of
  naming only everyday **औरत**. The lesson already defines **महिला** as the
  respectful public form and **स्त्री** as the older literary form, contrasts
  all three registers, and assesses which word belongs on a public sign; its
  metadata now exposes the complete set.

### Changed — declare both Hindi parent-register pairs

- Chapter 12 now declares **पिता / माता / बाप / माँ** as its headword instead
  of naming only the respectable **पिता / माता** pair. The lesson already
  defines **बाप / माँ** as the everyday spoken pair, practises both registers,
  and asks learners to recall the everyday forms; its metadata now exposes the
  complete assessed contrast.

### Changed — declare Hindi's time-telling verb

- Chapter 18 now declares **घंटा / बजना** as its headword instead of naming
  only “hour / bell.” The lesson already defines **बजना** as “to strike, to
  toll,” teaches it as Hindi's time-telling verb, practises singular and plural
  clock sentences, and asks learners to recall its literal meaning; its
  metadata now exposes that assessed verb.

### Changed — declare the complete Hindi book-register set

- Chapter 37 now declares **किताब / पुस्तक / पोथा** as its headword instead of
  naming only the ordinary **किताब**. The lesson already defines all three as
  the ordinary, formal, and old or homely words for “book,” respectively, and
  assesses the register contrast; its metadata now exposes the complete set.

### Changed — declare both everyday Hindi words for foot

- Chapter 38 now declares **पैर / पाँव** as its headword instead of naming only
  **पैर**. The lesson already defines **पाँव** as Hindi's other everyday word
  for “foot,” contrasts the two words' historical stems, and asks learners to
  recall which one descends from Sanskrit **पाद**; its metadata now exposes
  both assessed forms.

### Changed — declare the Hindi location postposition

- Chapter 5 now declares **रहना / में** as its headword instead of naming only
  the infinitive. The lesson already defines **में** as “in,” contrasts its
  post-noun position with English, practises that placement, and asks learners
  to recall it in the complete Delhi sentence; its metadata now matches that
  assessed grammar target.

### Changed — declare the Hindi thought-and-worry noun

- Chapter 34 now declares **सोचना / सोच** as its headword instead of naming
  only the infinitive. The lesson already defines **सोच** as the verb stem and
  as the standalone noun “thought” or “worry,” then asks learners to recall
  both meanings; its metadata now matches that assessed target.

### Changed — declare the full Hindi weather set

- Chapter 20 now declares **मौसम / गर्मी है / ठंड है / बारिश हो रही है** as
  its phrase headword instead of naming only **मौसम**. The lesson already
  defines, practises, and assesses the hot, cold, and everyday rain forms;
  formal **वर्षा** remains declared by the earlier season-set lesson.

### Changed — declare the spoken Hindi tea request

- Chapter 37 now declares **एक चाय दीजिए** as a phrase headword instead of
  declaring only **चाय** as a word. The lesson already defines the polite
  **दीजिए** request, contrasts it with written **कृपया**, and repeatedly asks
  learners to produce the complete counter request; its metadata now matches
  that outcome.

### Changed — declare both sides of the Hindi age frame

- Chapter 19 now declares **कितने साल के हो? / मैं … साल का हूँ।** as its
  grammar headword instead of only the question. The lesson already defines,
  practises, and assesses both **के** in the question and **का** in the answer;
  its metadata now matches the complete exchange.

### Changed — declare the Hindi work conjunct verb

- Chapter 5 now declares **करना / काम करना** as its headword instead of only
  **करना**. The lesson already defines **काम** as “work,” explains the full
  conjunct verb, and assesses learners producing “I work”; its metadata now
  matches that learner task.

### Changed — declare the full Hindi liking frame

- Chapter 35 now declares **मुझे … पसंद है** as its phrase headword instead of
  presenting only **पसंद** as a standalone word. The lesson already introduces,
  practises, and assesses **मुझे** as the required dative experiencer; its type
  and headword metadata now match that learner task.

### Changed — declare the evening doublet the lesson already teaches

- Expanded the chapter-32 headword declaration from **शुभ संध्या** to **शुभ
  संध्या / साँझ**. The lesson already defines, contrasts, practises, and assesses
  **साँझ**; the metadata now tells reports and downstream cards that it is taught
  rather than merely mentioned.

### Added — sentence-to-connected pre-A1 writing bridge

Six new Chapter 2 micro-lessons reuse only previously practised language to
move from a visible name-sentence choice through punctuation, delayed recall,
and heard-cue transcription into a two-sentence card for a named reader and
purpose. The final 180-second checkpoint removes the model, waits ten seconds,
and separates meaning-order, spacing, spelling, and punctuation repair.

This closes the short connected writing slice in #13446. It does not claim A1
or exam readiness; the complete four-skill assessment bridge remains #13424.

### Added — a cumulative first Hindi writing-stage runway

Hindi's existing writing lessons now expose an honest, machine-checkable first
support-removal sequence: visible head-line trace, visible guided copy of **न**
and **म**, delayed copy of the already-known **नमस्ते**, and transcription of
that same word from a heard cue. The two new lessons are capped at 120 seconds,
include compare-and-repair steps, and introduce no new language or script form.

This closes only the glyph-to-known-word stage slice in #13445. It does not
claim that Hindi writing, pre-A1, or any exam level is complete; phrase and
short connected-text work remains explicit in #13446.

### Fixed — one false forward-review claim

`HI-C05-rahna` now records only the earlier `HI-C05-bolna` lesson that its
warm-up, knowledge ledger, and practice actually rehearse. The removed
`HI-C05-main-hindi-bolta-hun` claim pointed three lessons into the future.
Hindi's authored order and the lesson's 240-second cap do not change; its
order-integrity debt falls from one to zero.

### Added — Chapters 60-66: Thirty-five more everyday words, round three

The third HL-C198 tranche for Hindi. The track stood at **120** distinct pre-A1
headwords against the 300 the vocabulary floor asks for — last of the seven
Indic tracks; these seven chapters answer with thirty-five more, one new word
per lesson, reusing everything already taught. The pre-A1 vocabulary shortfall
falls **180 → 145** (120 → 155 headwords).

  60 Animals in the Yard      बंदर साँप मुर्गी भैंस मोर
  61 At the Cooking Fire      बर्तन चम्मच गिलास तवा चूल्हा
  62 What Hands Work With     रस्सी सूई टोकरी मिट्टी लकड़ी
  63 What the Body Tells You  भूख प्यास नींद बुख़ार दर्द
  64 How Much and How Fast    अब जल्दी धीरे कम ज़्यादा
  65 Asking Nicely            मेहरबानी विनती अनुमति भरोसा अदब
  66 The Field and the Well   बीज फ़सल अनाज घास कुआँ

All seven pre-A1 spine nodes are used, one per chapter, each for the third time:
MEET-GREET, POLITE-REQUEST-REPAIR, EXCHANGE-NAMES, CHECK-WELLBEING,
RESPOND-BASIC, COURTESY-THANK, TAKE-LEAVE.

**Every headword is spelled from the 45 glyphs Hindi's own writing lessons
teach**, and so is every Devanagari citation in every body. The tranche adds
**zero** script-closure violations and **zero** exposure-exempted glyphs — the
first Hindi vocabulary wave for which that is true, and it is what chapter 59
bought. The chapter chains onto that chapter's last lesson, the retroflex ड.

Words worth naming. **साँप** and Latin *serpēns* are the same ancient word for
"the creeping one", neither borrowed from the other. **भैंस** is a feminine
Sanskrit form that outlived its masculine, because the female is the animal a
household talks about. **भूख** and **प्यास** were built in the same workshop:
both are Sanskrit desideratives, the wanting rather than the lack, and प्यास
shares its root with पानी. **कुआँ** is Sanskrit *kūpa*, a cousin of Latin *cūpa*
and so of English *cup*. **अदब** means courtesy and literature with one word.

Three near-collisions are taught rather than avoided: *ab* against *abhī*,
*kam* against *kamrā*, and *mehrabānī* against *mehmān* — each pair close in a
romanization, unrelated in fact, and separate on the page in Devanagari.

Counts held where they must: R1 1181, forward references 502, rule statements
30, script-closure violations 756, exposure-exempted glyphs 2220, hindi
cross-chapter prose references 20. R2 moves +35 by construction — the new atoms
sit at the end of the track with no successors yet.


### Added — Chapter 59, the vowel you write and do not say (HL-C223)

Eleven lessons. `neverTaughtGlyphs` **16 → 12**; Hindi now teaches 45 of the 57
glyphs it shows, the most of any non-Latin track.

**It opens with a rule, not a shape.** At the end of a word the inherent vowel is
not pronounced: **नाम** is *nām*, not *nāma*. Every reader who sounds a word out
letter by letter ends one vowel too long, and this is the **default** rather than
a per-word exception. Sanskrit says the final vowel; Hindi dropped it a thousand
years ago and kept the spelling — the same bargain English makes with the *k* in
*knight*.

Ten glyphs follow, chosen by how many lessons each unblocks rather than by
alphabet order. Two of them carry ideas: **ु** hangs *below* its consonant, giving
the reader a third mātrā position, and **ँ**, the chandrabindu, nasalises without
changing the vowel.

The chapter closes on the **retroflex** ड — the single most audible foreign-accent
tell in Hindi, and a tongue position rather than a sound to hear.


