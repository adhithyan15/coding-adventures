# Russian Roadmap — Absolute Beginner toward B1

> The unit is one word/phrase per lesson (see
> [`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)).
> Pronunciation and the Cyrillic letters live **inline** in the lessons and, for
> reference, in [`pronunciation-reference.md`](./pronunciation-reference.md) —
> there is no gated reading course. A *chapter* is a themed cluster of many
> small lessons, authored **lessons first, then the LaTeX chapter**.

## Assessment destination

The complete book must prepare a book-only learner for the project-defined
pre-A1 bridge and the external TORFL/ТРКИ ladder at A1–C2. See
[assessment-spec.md](assessment-spec.md) for official target names, the
five-subtest readiness rule, source-drift boundary, and validation requirements,
and [assessment.json](assessment.json) for the machine-readable seven-rung
four-skill floor. Lessons remain five minutes or shorter and Cyrillic writing
grows one action at a time; full mocks retain the live provider's continuous
timing. The contract records a destination, not current pass-readiness evidence.

## Chapter 1 — Greetings & courtesy *(authored)*

привет · здравствуйте · спасибо · да · нет · пожалуйста. Establishes the four
false-friend letters (в р с н), the formal/informal split, and *politeness =
plural*.

### Writing the letters *(authored)* — the "break it apart and write it" strand

`writing`-type lessons (HL02) that teach the *hand-formation* of the letters the
Chapter 1 words introduced — component strokes + stroke order, from the canonical
`cyrillic.json` the companion app renders:

- `RU-W01` — **в** (v ← beta) and **р** (r ← rho), the two false friends in *привет*.
- `RU-W02` — **с** (s ← sigma) and **н** (n, the *H*-look-alike), completing в·р·с·н.
- `RU-W03` — **б** (b) and **д** (d ← delta), two shapes with no Latin disguise
  (and how б differs from в).
- `RU-W04` — **п** (p ← pi) and **и** (ee, the backwards-N quiet false friend).
- `RU-W05` — **е** (ye, iotated) and **т** (t ← tau), which **completes every
  letter of привет** — the learner can now hand-write the whole word. More letters
  get a writing lesson as later chapters introduce them; **я, ч, ы and ь are
  read-only in Ch. 2**, and э, ю and ш await the chapters that need them.

## Chapter 2 — Introducing yourself *(authored)*

**я** — one letter, PIE \**eǵh₂(om)* → Slavic \**azъ*, cousin of *ego / ich / I*;
pronouns are the least borrowable part of a language → **ты / вы**, the split
English threw away when *thou* left standard use in the 1600s → a short support
lesson on why ***вы* is polite**. *Вы* is polite the way French *vous* is, but
the routes differ: Russian and French use the
**2nd-person plural**, German the **3rd-person** (*Sie*), and Spanish **no plural
at all** (*usted* ← "your grace") — with the *вы*/*vōs* cognate row flagged as
only half a set, since English *you* and German *ihr* continue \**yūs*
→ **меня зовут…**, which is literally "**[they] call me**", so the
sentence contains **no word for "my" and none for "name"**; the "they" is nobody
(a bare plural verb, like English *they say it'll rain*), and *меня* is not *я*
but its **object form** — the course's first look at **case** → **как вас
зовут?**, which asks **how** they call you, not *what*: Russian, French
(*comment*) and Spanish (*cómo*) all ask about an **action** where English asks
about a **possession** in a separate comparison lesson → **очень приятно**,
"very pleasant", whose *приятно* ←
Slavic *prijati* "to favour" ← PIE \**preyH-* "to love, please" — the root behind
Russian **приятель** "friend" *and* English **friend**, with **free** most likely
in the same family ("belonging to the beloved household") → three short practice
lessons: retrieve the exchange, contrast the person shapes, then close on the
**zero copula** — scoped to ***очень приятно***, the one sentence with no verb at
all (Russian *does* have a present-tense *есть*; it just drops the linking verb).

Deliberately deferred, so the chapter stays five vocabulary atoms plus two
support lessons and three focused practices: **это**, and *да/нет* revisited as
full answers. Each lesson remains below five minutes. New letters **я**, **ч**,
**ы** and the soft sign **ь** are flagged read-now-draw-later — the writing track
(RU-W01–W05) has taught в р с н б д п и е т and not yet reached them.

## Chapter 3 — Six verbs, and the one you never say *(authored)*

The track's first verbs, and the first realization of the shared `VERB-*`
concepts by any track in the corpus. Six words, one grammatical idea each, in
this order:

**быть** "to be" ← PIE \**bʰuH-*, which is English **be** — and the lesson is
really about the **zero copula**: *я студент*, "I student", with nothing in
between, because Russian declines to use the verb in the present at all (the
past *был* walks straight back in). English fuses three ancient roots into
*be / is / was*; Russian keeps two of them visibly apart as **быть** and **есть**
→ **жить** "to live" ← \**gʷeih₃-*, the root of Latin *vīvere* and Greek *bíos*
**and** of English **quick** in its older sense of *alive* (*the quick and the
dead*); grammatically it introduces the **-у** that alone means "I"
→ **знать** "to know" ← \**ǵneh₃-*, literally English **know** with its silent
*k* still pronounced, plus the Latin (*notice, ignore, noble*) and Greek
(*diagnosis, agnostic*) branches; grammatically it introduces **не**, the whole
of English *don't* with no helper verb — and hands over *я не знаю*
→ **говорить** "to speak", the chapter's one new letter (**г**, Greek gamma) and
its one **false friend**: English *govern* is Greek *kybernân* "to steer" and is
**not** a relative; grammatically it splits the verbs into the **-ешь / -ишь**
families → **видеть** "to see" ← \**weid-*, the root that meant *see* **and**
*know*, hence *video/vision* on one branch and **wit / wise / Veda** on the
other; grammatically it introduces the **д → ж** swap that hits the *I* form and
nothing else (*вижу*, but *видишь*) → **идти** "to go" ← \**h₁ei-* (Latin *īre*
→ *exit, transit, itinerary*), whose past **шёл** comes from a different root
entirely — *go / went* twice over — and which introduces **verbs of motion** in
the gentlest possible dose: *иду* (now, one way) against *хожу* (habitually).

No new letters beyond **г**, no tables, and every lesson is `voice` — the whole
chapter is drivable, which is the first time that has been true of a Russian
chapter end to end.

Chapter 3 now has a LaTeX chapter, **generated** from these lessons rather than
hand-written — the first generated chapter in the Russian book. It still has no
HL05 ledger entry: with no consolidation lesson, its only candidate payoff
(`RU-C03-idti`) reaches 5 of the chapter's 19 atoms, a representativeness of
0.26 against the 0.50 floor. That debt is recorded rather than papered over.

## Chapter 4 — Verbs of the mind and the page *(authored)*

**думать** "to think" ← the noun **дума**, "a thought", and the State **Дума**;
the standard account makes *дума* an early Gothic borrowing from *dōms*
"judgement", so English **doom** and **deem** are relatives by contact rather
than by descent, and the lesson says which kind of link that is. Grammatically
it opens the fact that governs every Russian verb: verbs travel in **pairs**,
*думать · подумать*, doing against deed-done → **понимать** "to understand" ←
\**h₁em-* "take" (Latin *emere* → *exempt, redeem, example*), so the word
literally says *take hold of*, exactly as **comprehend** and **grasp** do;
grammatically it shows a pair built by reshaping a stem (*понимать · понять*)
and hands over ***я не понимаю*** → **читать** "to read", on the Slavic root
that also gave **число** "number" and **честь** "honour" — and with **no secure
English cousin**, which the lesson says outright rather than inventing one;
grammatically it explains why Russian has no separate *I am reading*: **быть**
has no present tense to build one with → **писать** "to write" ← \**peyḱ-* "to
cut, scratch", the root of Latin *pingere* and so of **paint, picture,
pigment**; it teaches the letter **ш** the track has been reading since *живёшь*
without naming, and it flags the **stress trap** — *pisát'* is *to write*,
*písat'* is a child's word for urinating, and beginners hit it constantly.

## Chapter 5 — Taking, asking, helping, loving *(authored)*

**брать** "to take" ← \**bʰer-* "to carry" — English **bear, birth, burden**,
Latin *ferre* (*transfer, refer*), Greek *phérein* (*metaphor*); its finished
partner **взять** is **suppletive**, a different word entirely, the same trick
that gives *идти* its past *шёл* and English *go* its *went* — and *взять* turns
out to sit on the very root behind *понимать*. **Брат** "brother" is flagged as
the false friend it is → **спрашивать** "to ask (a question)" ← \**preḱ-*
"to ask", which became Latin *precārī* and so English **pray, prayer,
precarious**, and German *fragen*; the ordinary pair *спрашивать · спросить*
after the strange one → **помогать** "to help", built on **мочь** "to be able" ←
\**magʰ-*, English **may, might, dismay** and Greek *mēkhanḗ* → *machine*; its
partner **помочь** ends in **-чь** rather than **-ть** → **любить** "to love,
and to like" ← \**leubʰ-*, which is English **love** itself, plus **lief**,
**believe** and *by your leave*; it teaches the **л** that appears in *люблю*
and nowhere else in the verb, and it closes the chapter by putting every verb
the track owns after *люблю* as a bare infinitive.

Aspect is **named and not finished** in these two chapters. Every one of the
eight verbs is imperfective, every lesson gives its perfective partner by name,
and what a pair does to a whole sentence is deferred to a chapter of its own —
the honest ramp for the hardest thing about Russian verbs.

## Chapter 6 — Having, and what things are *(planned)*

- **это** sentences ("this is…"), and **быть** in past and future in their own
  right rather than as a footnote to the present.
- **у меня есть…** — Russian's "I have" as "by me there is", reusing *есть* met
  in *нет* and named again in Chapter 3.
- Gender of nouns (-∅ / -а / -о) — the first grammatical-gender lesson.
- Numbers 0–10.

## Part II onward *(sketch)*

Cases introduced one at a time on real need (accusative for objects → prepositional
for location → genitive for "of"/negation), the rest of the **-ешь / -ишь**
conjugations now that Chapter 3 has named the split, aspect
(imperfective/perfective) taught properly, now that chapters 4-5 have named the
pair on sixteen verbs and the идти/ходить split is already in hand — more core verbs against the shared `VERB-*`
list, everyday vocabulary (food, city, time), building toward B1 "normal
day-to-day conversation."

The roadmap is a standing plan, updated as chapters are authored.
