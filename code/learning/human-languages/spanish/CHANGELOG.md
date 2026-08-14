# Changelog

## HL-C160 — B1 closes, with one lesson instead of four

`SPINE-EXPRESS-CONDITION` read as unrealized, and the obvious move was to author
its four declared concepts. Measuring first showed that would have duplicated
real teaching: **eleven lessons** across chapters 123 and 196–210 already build
the whole conditional system — `si`, `si + futuro`, `si tuviera`, the
conditional in all three verb families, both synthesis chapters. Three of the
four spine concepts are taught; they are simply tagged `ES-CONDITION-*` rather
than with the bare spine names.

Exactly one thing was genuinely missing, and that is what this adds:
**`depende`** — the condition you assert without stating, from Latin *de-* +
*pendere*, "to hang down from."

The omission ledger now records `CONDITIONAL-REAL`, `CONNECTIVE-IF` and
`MODAL-WOULD` as unclaimed **spine names**, which is honest: the teaching exists
and the tag does not. Reconciling those three is a tagging pass, not a content
one, and writing four duplicate lessons to make a number go up would have made
the corpus worse.

Two things the gates caught, both fixed in the content:

* The lesson used *depende del tiempo*, and `del` is taught 21 lessons later.
  It says *depende de ti* now — forward references stay at 499.
* Sequence 2291 sat in chapter 210 while chapter 211 already held 2290, so
  chapter numbers went backwards along the sequence. Re-seated to 2287.

**pre-A1 through B1 are complete. The spine stands at 21 of 33.** Everything
that remains is B2 and above.


## HL-C159 — B1 continues: describing an experience

`SPINE-DESCRIBE-EXPERIENCE` had four declared concepts and no lessons. Chapter
269 authors all four as the moves you make when you tell someone what something
was like: **`feliz`** (how it felt), **`espero viajar`** (what you are reaching
for), **`más que`** (how it compared), **`durante`** (how long it lasted).

Prose-only again: `drivablePercent` holds and `fullyDrivableChapters` rises
480 → 481.

**Two ratchets held, because both problems were fixed rather than absorbed.**

The rule-statement count first went to 30 against a ceiling of 29. Two guesses
at the cause were wrong; measuring it directly found the third lesson, and
softening `durante`'s "for" contrast into a note brought it back to 29.

Forward references went to 502 against a ceiling of 499, and the report named
the cause exactly: `espero` is *used* in three chapter-40 lessons, and this
chapter was the first to *own* it — 108 lessons later. Rather than raise the
ceiling for a word the corpus had been using all along, the lesson now owns the
frame it actually teaches, `espero viajar`. Forward references: 499, with zero
from chapter 269.

B1 stands at 4 of 5; the spine at 20 of 33.


## HL-C158 — B1 opens: the travel rung

`SPINE-HANDLE-TRAVEL` had four declared concepts and no lessons. Chapter 268
authors all four, as one arc a traveller can actually run: ask where a thing is,
buy the ticket, take the room, and say when it has gone wrong.

* **`¿dónde está?`** — a frame rather than a sentence. Both halves were already
  taught, so it costs no new grammar; you finish it with a noun.
* **`billete`** — ticket, and banknote, from Latin *bulla*, a wax seal. Notes
  that much of Latin America says *boleto*.
* **`habitación`** — a room, transparently *habitar* + *-ción*, which buys the
  reader the whole *-ción* / *-tion* family.
* **`problema`** — and the Greek *-ma* nouns that end in *-a* and are masculine
  anyway.

Deliberately prose-only, no tables: `drivablePercent` rises 65 → 66 and
`fullyDrivableChapters` 479 → 480, so the car edition gains a chapter rather
than losing one. That is the opposite of what HL-C157's conjugation table did,
and it is the reason to reach for prose when prose will carry the content.

The rule-statement ceiling moves 28 → 29. Three rule statements arrived with
this chapter and two were deleted rather than absorbed — `problema` restated its
own rule, `habitación` stated the `-ción` rule twice. The survivor is the Greek
`-ma` family, which is the entire point of that lesson.

B1 stands at 3 of 5; the spine at 19 of 33.


## HL-C157 — A2 closes: `ayer` and `hablare`

The A2 stage stood at 3 of 5 spine nodes. The two that were open were not
missing lessons -- `SPINE-TALK-ABOUT-PAST` already had twenty-nine path
segments teaching preterite and imperfect forms -- they were missing their
one declared concept, and the measurement had been read as a content gap.
Looking at what the corpus actually taught turned up two real holes:

* **`ayer` was never taught.** Four hundred and seventeen Spanish lessons, a
  whole chapter block on the past, and no word for "yesterday." It now opens
  chapter 103, ahead of the forms, as the time frame that obliges the verb to
  follow it backwards -- and it names the preterite/imperfect split before the
  endings arrive, so those endings do not read as arbitrary.

* **The future tense was never taught.** Spanish had `voy a hablar` and
  stopped there; `hablare` did not exist anywhere in the corpus. It joins
  chapter 49, beside the near future it doubles for, with the infinitive
  visible whole inside it and the `habeo` that fused onto the end.

A2 is 5 of 5. The spine stands at 18 of 33 overall; B1 is next at 2 of 5.

One number moved the wrong way and is pinned as such: `fullyDrivableChapters`
falls 517 -> 516 and `drivablePrefixTotal` 1258 -> 1257, because the future
lesson carries a conjugation table and a table cannot be driven. That is an
honest sight lesson, not a detachable block, so the car edition loses chapter
49's tail rather than the number being explained away.


## Chapters 7–18 — canonical book generation

The final twelve handwritten Spanish chapter bodies are now deterministic
outputs of the same 67 schema-v2 lesson ASTs that Language Ladder, narration,
review activities, and the generated back matter consume. Their existing
chapter titles, labels, prerequisite-safe lesson order, examples, review
questions, and answer contracts remain intact; four titles shed presentation-
only `\emph` markup so the manifest, capability ledger, and generated TeX agree
as plain canonical text.

The publication build also surfaced layout details that were invisible while
the canonical lessons and handwritten book copy diverged. Chapter 7's warning
emoji is now a portable text callout, the Chapter-15 and Chapter-16 terminal
recaps use speakable person rows instead of wide seven-column tables, and the
Chapter-18 regular-form comparison uses short bullets. Those changes remove the
new missing-glyph and overfull-box warnings while making two more checkpoints
voice-drivable. All 41 Spanish chapters are now source-hash checked generated
files; no independent handwritten teaching body remains.

## Chapter 18 — singular wanted events and the present subjunctive

Nine schema-v2 micro-lessons now replace the ten legacy Chapter-18 lessons.
The chapter begins with meaning rather than a paradigm: **Hablas español**
asserts an event, while **Quiero que hables español** presents it as wanted and
not yet asserted. **Quiero hablar** keeps the same-doer infinitive beside the
different-doer **quiero que** frame without introducing an object pronoun,
person noun, or broader clause taxonomy.

Regular forms arrive one singular row at a time: **hable/hables/hable**,
**coma/comas/coma**, and **viva/vivas/viva**. Three later lessons carry forward
only known irregularities, producing **quiera/quieras/quiera** from *quiero*,
**pueda/puedas/pueda** from *puedo*, and **haga/hagas/haga** from *hago*. The
other stem-changing and *-go* verbs, the old outlier inventory, every plural
person, and additional subjunctive triggers remain deferred.

The twelve-atom chapter preserves two careful etymology bridges. *Subjuntivo*
descends from Latin *subiūnctīvus*, “subjoined,” while the RAE records *ojalá*
from Hispanic Arabic *wa šá lláh*, “and God has willed.” The historical Arabic
phrase explains a route but is never counted as usable Spanish vocabulary.
Thirty-four mapped activities spend each new form immediately, and the terminal
checkpoint retrieves every atom using only the bounded **quiero que** and
**ojalá** frames.

## Chapter 12 — singular doing, making, and saying

All four Chapter-12 lessons now use schema v2: explicit order, sub-five-minute
duration, typed prerequisite closure, shared-spine placement, and twelve
objective answer contracts. The three teaching lessons introduce eight atoms,
never more than three at once; the terminal checkpoint introduces none and
retrieves the complete chapter.

The migration replaces the old five-person tables with **hago, haces, hace**
and **digo, dices, dice**, the same singular-person frame the learner already
owns. *Hacer* uses only the known object *café*, and *decir* reuses *hola* and
the known *cómo* question frame. Weather, homework, windows, and every plural
form wait until their own vocabulary and person steps.

The *tengo · hago · digo* comparison is intentionally descriptive rather than
predictive. It contains only forms already learned and no longer previews
*pongo, salgo,* or *vengo*. Its history is also more careful: inherited sound
changes and analogy made several common forms converge on **-go**; there was no
single productive Latin ending that can manufacture an unseen Spanish form.

The etymological spine remains deep without widening the usable corpus.
*Hacer* continues Latin *facere*, the family behind *fact* and *factory*;
*decir* continues Latin *dīcere*, the family behind *diction*, *dictionary*,
and *predict*. Separate Spanish-local grammar and consolidation extensions keep
the four lessons in prerequisite order on `SPINE-SAY-WHAT-I-DO`.

## Chapter 11 — singular wanting, ability, and agreement

All five Chapter-11 lessons now use schema v2: explicit order, sub-five-minute
duration, typed prerequisite closure, shared-spine placement, and thirteen
objective answer contracts. The four teaching lessons introduce eleven atoms,
never more than three at once; the terminal checkpoint introduces none and
retrieves the complete chapter.

The migration replaces two full five-person “boot” tables with the singular
frame already established by the regular verbs and *tener*. Learners first add
**quiero, quieres, quiere**, then **puedo, puedes, puede**, always followed by
one owned infinitive such as *hablar, comer,* or *vivir*. Plural persons and
untaught verbs such as *venir, dormir,* and *volver* wait.

The comparison lesson generalizes only what the learner can inspect. It places
the three learned *querer* forms beside the three learned *poder* forms and the
known *tener* set, explains stressed Latin **e/o → ie/ue** as a historical
organizer rather than a perfect prediction machine, and keeps the comparison
table narrow enough for spoken narration.

Possessive agreement now arrives one contrast at a time: **nuestro día** beside
**nuestra noche**, using two nouns whose gender was learned in Chapters 1 and 2.
The old plural forms, *vuestro* paradigm, and undeclared *casa/coche/amigos*
examples are deferred. The *noster → nuestro* history connects the new
possessive back to the same vowel-opening story without adding another usable
word.

Curriculum placement reflects the chapter's three actual jobs. *Querer*, the
vowel comparison, and the terminal checkpoint realize
SPINE-SAY-WHAT-I-WANT; *poder* realizes the shared VERB-CAN concept on
SPINE-SAY-WHAT-I-DO; and singular *nuestro/nuestra* extends
SPINE-DEFINITE-REFERENCE. Separate Spanish-local grammar and consolidation
extensions preserve the five-lesson prerequisite order across those nodes.

## Chapter 10 — singular motion, future, and possession

All four Chapter-10 lessons now use the schema-v2 contract shared by Language
Ladder and the generated book: explicit order, sub-five-minute duration, typed
knowledge closure, shared-spine placement, and ten objective answer contracts.
The three teaching lessons introduce nine atoms, never more than three at once;
the terminal checkpoint introduces none and retrieves the full chapter.

The migration removes the old full **voy, vas, va, vamos, van** table. Learners
now receive only the singular-person set supported by the preceding chapters:
**voy, vas, va**. The first lesson asks **¿Dónde va usted?** without slipping in
a destination preposition, and keeps the useful suppletion story: *ir* continues
Latin *īre*, while its present forms continue *vādere*, the family behind
*invade* and *evade*.

The near-future lesson adds exactly one link, **a**, then combines it with only
owned infinitives: **voy a hablar, vas a comer, va a vivir**. It teaches the
motion-toward-an-event metaphor without previewing another infinitive or a plural
person. The possessive lesson is equally narrow: **mi café, tu café, su café**
uses one known singular noun, introduces no number or gender paradigm, and
defers *mis/tus/sus* along with every plural noun frame.

Curriculum placement now follows the chapter's actual jobs. Singular *ir* sits
on `SPINE-ASK-LOCATION`; the near future and closing payoff sit on
`SPINE-TALK-ABOUT-FUTURE`; and singular possessives extend
`SPINE-DEFINITE-REFERENCE`. Spanish-local grammar and consolidation extensions
make that placement explicit, while the shared `VERB-GO` concept is honestly
relocated from `SPINE-SAY-WHAT-I-DO` to the location step.

## Chapter 9 — one ser/estar contrast at a time

All five Chapter-9 lessons now use the schema-v2 contract shared by the app and
book pipeline: explicit order, sub-five-minute duration, typed knowledge
closure, shared-spine placement, and twelve objective answer contracts. The
four teaching lessons introduce nine atoms, never more than three per lesson;
the terminal checkpoint introduces none and revisits the whole chapter.

The migration replaces the old full conjugation tables with the singular frame
the learner already uses: **soy, eres, es** beside the previously learned
**estás, está**. Plural forms wait. The first contrast is correspondingly small:
**¿Es usted Ana?** identifies someone, while **¿Está usted bien?** asks about a
current state. Untaught adjectives such as *aburrido, listo, rico,* and *verde*
no longer appear as supposed minimal pairs before their own lessons.

The chapter also drops “permanent versus temporary” as if it were the rule.
The etymological bridge remains useful — *ser* carries the *esse/essence*
family, while *estar* descends from *stāre*, “stand” — but the operational
question is what kind of claim the sentence makes: identity or origin uses
*ser* here; state or location uses *estar*. A long-lasting location still takes
*estar*, and a brief identification still takes *ser*.

Origin and location now reuse only prior material: **¿De dónde eres? — Soy de
Madrid** and **¿Dónde está usted? — Ana está en Madrid** build on *dónde, en,
usted,* and the known place name *Madrid*. The location lesson revisits the
origin frame before the terminal checkpoint, improving rather than weakening
the pre-A1 reinforcement frontier. Curriculum placement is explicit across
`SPINE-SAY-WHAT-I-DO`, `SPINE-CHECK-WELLBEING`, `SPINE-EXCHANGE-NAMES`, and
`SPINE-ASK-LOCATION`, with Spanish-local extensions for the language-specific
contrast and a mapped consolidation payoff.

## Chapter 8 — numbers and age cross the schema boundary

Chapter 8 now carries the same strict schema-v2 contract as Chapter 7: all five
lessons have an explicit order, sub-five-minute duration, shared-spine placement,
typed knowledge closure, body-block declarations, and objective activity
contracts. The chapter introduces exactly 12 atoms — the configured chapter
budget — and never more than three in one lesson. Its closing practice lesson is
now reachable in the app and is the atom-scored chapter payoff.

The migration also restores the intended gentle frontier. Chapter 8 teaches only
numbers **one through ten**; the old previews of *dieciséis, diecinueve,
veintiuno,* and *veinte* reached into Chapter 31 and are gone. *Tener* now extends
the learner's existing three-person frame with **tengo, tienes, tiene** instead
of presenting untaught plural forms. The age exchange consequently uses only
known numbers: *¿Cuántos años tienes? — Tengo nueve años.*

The Roman-calendar note is corrected rather than repeated as a mnemonic myth.
The numbered month names preserve a March-first count, while Quintilis and
Sextilis were existing months later **renamed** July and August; those months
were not inserted between June and September. The lesson keeps the useful
*siete/September … diez/December* memory path and distinguishes it from the
historical explanation.

Finally, Spanish now realizes `NUMBER-ONE-TO-FIVE` on the actual shared
`SPINE-COUNT-ONE-TO-FIVE` node instead of classifying Chapter 8's numbers under
time of day and omitting the shared concept. Six-through-ten and the age phrase
remain honest Spanish-local extensions, while the closing drill is classified
as consolidation.

## Chapter 7 — reopening the schema boundary without skipping a rung

Chapter 7 is the first chapter after Spanish's old schema-v2 boundary. Its six
lessons now carry the strict duration, sequence, shared-spine, knowledge-closure,
body-block, and practice contracts used by both the book pipeline and Language
Ladder.

The migration resolves a real order conflict. The curriculum map previously
scheduled **comer → beber → qué → vivir → dónde**, while the chapter prose and
review links taught **comer → vivir → beber → qué → dónde**. The latter is now
canonical because each step supplies the next one's input: *vivir* compares
*-ir* directly with *comer*'s *-er* pattern; *beber* then rehearses that pattern;
*qué* asks with *comes/bebes*; and *dónde* finally combines the question system
with *vives*. `ES-C07-practice` closes the same path instead of remaining outside
the app schedule.

The content was tightened at the same boundary. Plural verb forms are deferred
instead of appearing in a table the learner is not yet asked to use. The old
café exchange depended on *pan*, *agua*, *una tapa*, and *algo más*, all taught
much later or not yet taught at all; it now builds only with prior *café,
estudiar,* and *español*, plus the chapter's own words. The chapter introduces
exactly 12 knowledge atoms, at the configured chapter budget, never more than
three in one lesson. Eight executable answer contracts exercise the singular
verb patterns, *qué*, and *dónde ... en ...* directly in the app.

## Chapters 38-39 — the final verb tranche, and a false cousin refused

The last seven of the shared spine's forty core verbs realized by **no track
anywhere**. `verbCoverage`'s `universallyMissing` set is now **empty**: every
core verb is realized somewhere in the corpus.

| Chapter | Lessons | Concept |
|---|---|---|
| 38 — Bringing, Getting, Playing, Meeting | `ES-C39-traer` · `ES-C39-conseguir` · `ES-C39-jugar` · `ES-C39-conocer` | `VERB-BRING` · `VERB-GET` · `VERB-PLAY` · `VERB-MEET` |
| 39 — Waiting, Answering, Buying | `ES-C40-esperar` · `ES-C40-contestar` · `ES-C40-comprar` | `VERB-WAIT` · `VERB-ANSWER` · `VERB-BUY` |

Spanish now covers **36 of the 40 core verbs (90%)**, and the corpus mean rises
from 36% to 37%. The split is by what makes each verb hard: Chapter 38 holds the four whose *yo*-form or stem misbehaves, Chapter
39 the three that are perfectly regular, where the whole difficulty is meaning.

**The brief for this tranche claimed *comprar* shares a root with English
*compare*. It does not.** Latin had **two** verbs spelled *comparāre*: one from
*com-* + *parāre* ("to make ready, to procure"), which is where *comprar* comes
from, and one built on *compār* ("equal, matched"), which is where English
*compare* comes from. They fell together in spelling, not in ancestry. The
lesson teaches the refusal rather than the link — the same discipline
`ES-C37-abrir` applies to *April*. *Comprender* (← *comprehendere*, "to seize
together") is named as a third unrelated *com-* word.

Two further links are hedged rather than asserted: **prosper** is *traditionally*
read as *prō* + the ablative of *spēs*, which is the traditional analysis rather
than a settled one; and *conocer*'s *-zc-* is described as a hard cluster the
other forms lost, without claiming a regular sound law for it.

What the chapters teach beyond the verbs:

- **esperar is one word for three English verbs** — *wait*, *hope* and *expect* —
  and nothing in the Spanish distinguishes them. This is the most useful thing in
  the tranche, and it is taught as a reading-the-situation skill rather than a
  translation table.
- **conocer vs saber.** `VERB-KNOW` is still omitted from Spanish's spine and
  *saber* is taught by no schema-v2 lesson, so *saber* is introduced here as the
  contrast partner inside `ES-GRAMMAR-CONOCER-SABER-12` rather than assumed. The
  etymology carries the split: *cognōscere*'s *-sc-* is Latin's inchoative
  marker, so the verb means "to **come** to know" — which is exactly why
  *conocer a alguien* is *to meet*.
- **jugar is the only u→ue verb in Spanish**, and the only *-gar* verb that
  stem-changes at all. Its crack landed on a *u* because Latin *iocārī*'s *o* had
  already slid there irregularly.
- **conseguir changes stem and spelling at once**: **e→i**, and the *u* of *gu*
  leaves once there is no front vowel to guard against — *consigo*, not
  *consiguo*. This completes the boot: `ES-C37-cerrar` gave e→ie, `ES-C36-dormir`
  gave o→ue, and `ES-C35-preguntar` previewed e→i in *pido*.
- **traer vs llevar** is a deixis split English does not make: *traer* carries
  toward the speaker, *llevar* away. *Llevar* ← *levāre*, the verb already behind
  *levantarse*.
- **contestar and responder** are both taught, with *la respuesta* borrowed from
  the second and never from the first.

Etymology: *trahere* (tractor, traction, attract, contract, subtract, abstract,
extract, distract, retract; and worn down through French — trace, train, trail,
portray ← *prōtrahere*, retreat ← *retrahere*); *sequī* (sequence, sequel,
consequence, subsequent, consecutive, second ← *secundus*, execute, prosecute,
obsequious, pursue, ensue, suit); *iocus* (joke, jocular, juggler ← *ioculātor*,
jeopardy ← *jeu parti*), with *lūdere* — the verb Spanish dropped — keeping
illusion, allude, elude, delude, collude, prelude, interlude, ludicrous;
*cognōscere* (cognition, recognize, incognito, notice, notion, notorious,
noble), beside its Germanic siblings *know* and *ken*; *spērāre* (despair ←
*dēspērāre* "to un-hope", desperate, desperado ← Spanish *desesperado*), with
*la esperanza* and **Esperanto**, Zamenhof's "one who hopes"; *testis* (contest,
testify, testimony, testament, attest, protest, detest) and *spondēre* (respond,
response, responsible, sponsor, spouse ← *spōnsus*); *parāre* (prepare, repair,
apparatus, apparel, separate, parade, emperor ← *imperāre*).

Reinforcement (HL09 §7). The tranche introduces 21 atoms and **rescues 18 atoms
that no lesson had revisited at any distance** — including the whole of Chapter
37's tail (*levantarse*, *de pie*), *coger* and the *preguntar*/*pedir* split
from Chapter 35, the *e-* run-up shared by *escribir* and *estudiar*, *sí* from
Chapter 19, *padre/madre* from Chapter 23, *pan*/*compañero* and *vino* from
Chapter 26, and *café*/*qahwa* and *por favor* from Chapter 6. Spanish's
never-revisited share drops from **36% to 27%** (80 of 221 atoms → 65 of 242).
Of the 21 new atoms, 18 close R1 (n+1…n+3); the three that do not are
`ES-C40-comprar`'s own, and the track has no lesson after it to carry them —
the same tail every tranche leaves for the next one, and the reason this one
could rescue Chapter 37's.

Both chapters' payoffs assess **every** atom their chapter introduces (12/12 and
9/9 against a 0.5 floor) plus the reach-back above, and all seven lessons derive
`voice`: no table wider than two columns, no sight cue, all under the 300-second
computed gate (226–287s). The book compiles under XeLaTeX with **zero** missing
characters, 278 pages.

## Chapters 36-37 — the third verb tranche, and a corrected etymology

Fifteen of the shared spine's forty core verbs were still realized by **no track
anywhere**. This lands eight of them in Spanish, atom-first, one verb per lesson:

| Chapter | Lessons | Concept |
|---|---|---|
| 36 — Hearing, Sleeping, Walking, Running | `ES-C36-oir` · `ES-C36-dormir` · `ES-C36-caminar` · `ES-C36-correr` | `VERB-HEAR` · `VERB-SLEEP` · `VERB-WALK` · `VERB-RUN` |
| 37 — Opening, Closing, Sitting Down, Standing Up | `ES-C37-abrir` · `ES-C37-cerrar` · `ES-C37-sentarse` · `ES-C37-levantarse` | `VERB-OPEN` · `VERB-CLOSE` · `VERB-SIT` · `VERB-STAND` |

Spanish covers **29 of the 40 core verbs (73%)**, and these eight leave the
"taught by nobody" set, which drops from 15 to 7.

**The brief for this tranche derived *cerrar* from Latin *claudere*. It does
not.** *Cerrar* comes from Late Latin **serāre**, "to bolt", from **sera**, the
bar dropped across a door (Vulgar Latin *serrāre*; Italian *serrare*, French
*serrer*). *Claudere* is where English got *close, clause, include, exclude,
conclude, recluse* — and Spanish kept it only in scholarly re-borrowings
(*incluir*, *excluir*, *concluir*, *clausura*). That inversion became the
lesson: the bigger English family belongs to the verb Spanish gave up. Two other
tempting links are named and refused rather than stretched — **April** is not
from *aperīre* (old folk etymology; the better proposals are Etruscan *Apru* or
an Italic "the next one"), and *sera*'s link to *serere* "to join in a row" is
usual but not certain.

What the chapters teach beyond the verbs:

- **The body-position verbs are reflexive**, and both were built the same way:
  *sentar* ← Vulgar Latin \**sedentāre* from *sedēns*, and *levantar* ←
  \**levantāre* from *levāns* — two Latin **present participles** turned into new
  verbs. The pair also separates the two features cleanly: *sentarse* is
  reflexive **and** an e→ie stem-changer, *levantarse* is reflexive only. Spanish
  has no verb for standing as a *state*, so **estar de pie** is taught alongside.
- **oír vs escuchar** — the involuntary/voluntary split English marks less
  sharply. *Oír* is wildly irregular: the *-go* yo-form *oigo* joins the Chapter
  12 club, and *oyes/oye/oyen* are a **spelling** rule, not a sound change.
- **dormir (o→ue) and cerrar (e→ie)** are the payoff for the boot Chapter 11
  taught and Chapter 34 spent on *pensar*/*entender* — the same rule, both vowels.
- ***abrir* has an irregular past participle**, *abierto*, inherited whole from
  Latin *apertum*, against *cerrado* made by rule.

Etymology is the widest honest cousin web: *audīre* (audio, audience, audition,
auditorium, audit, and **obey** ← *oboedīre* = *ob-* + *audīre*), with
*escuchar* ← *auscultāre* giving *auscultate* and **scout**; *dormīre*
(dormitory, dormant, dormer — *dormouse* flagged as likely but unproved);
*currere* (current, currency, curriculum, courier, cursor, corridor, occur,
excursion), with English **car** ← Gaulish *carrus* shown to be a cousin at PIE
\**kers-*; *sedēre* (sedentary, session, siege, preside, reside, assess,
possess, cathedral, chair) beside its Germanic branch *sit/seat/settle/saddle/
nest*; *aperīre* (aperture, overt, overture, aperitif); *levāre* (lever,
elevate, levity, alleviate, relieve, leaven, levy, **the Levant**).

Two honest gaps are stated as gaps rather than filled: *caminar* rests on
**Gaulish** *camminus* by way of *el camino*, a Celtic loan that stocked every
Romance language and reached English only inside borrowed French phrases (while
the near-identical Latin *camīnus* "hearth" is a different word and gave English
*chimney*); and ***andar*'s origin is genuinely disputed** — Medieval Latin
*andāre*, with *ambulāre* and *ambitāre* both proposed and neither proved, so
*amble* and *ambulance* are offered as a conditional maybe.

Reinforcement runs at two cadences, per HL09 §7.2. Each lesson practises the
atoms of the one or two before it, across the chapter seam (`ES-C36-oir`
practises Chapter 35's *gustar*; `ES-C37-abrir` practises Chapter 36's
*correr*), which closes R1 at zero new lessons. Each payoff reaches several
chapters back to atoms nothing had revisited: Chapter 36's `correr` rescues
`ES-LEX-PERRO-03`/`ES-EVIDENCE-PERRO-04` (Ch 32) and `ES-LEX-WEEKEND-01` (Ch
21); Chapter 37's `levantarse` rescues `ES-GRAMMAR-REFLEXIVE-FIRST-PERSON` (Ch
3, the atom *sentarse* is built on) and `ES-LEX-MANO-01`/
`ES-GRAMMAR-MANO-GENDER-02`/`ES-LEX-CABEZA-01` (Ch 24). Spanish's
never-revisited atoms fall from **90 of 199 (45%)** to **80 of 221 (36%)**.

Wiring: `curriculum.json` gains `ES-PATH-032` and `ES-PATH-033` on
`SPINE-SAY-WHAT-I-DO` and drops the eight concepts from that node's `omits`;
`chapters.json`, `core/book-generation.json`, `book.tex` and the generated
`ch36`/`ch37` TeX and narration follow. All eight lessons are schema v2, `voice`
modality, ≤3 new atoms each (11 per chapter, against `maxNewAtomsPerChapter: 12`)
and computed at 268-298 s — the drivable prefix is 4 of 4 for both chapters. The
book compiles to 262 pages with zero `Missing character` warnings.

## Chapters 34-35 — the second verb tranche, eight verbs no track had

Twenty-three of the shared spine's forty core verbs were realized by **no track
anywhere**. This lands eight of them in Spanish, atom-first, one verb per lesson:

| Chapter | Lessons | Concept |
|---|---|---|
| 34 — Four Verbs of the Mind | `ES-C34-pensar` · `ES-C34-entender` · `ES-C34-leer` · `ES-C34-escribir` | `VERB-THINK` · `VERB-UNDERSTAND` · `VERB-READ` · `VERB-WRITE` |
| 35 — Taking, Asking, Helping, and the Backwards Verb | `ES-C35-tomar` · `ES-C35-preguntar` · `ES-C35-ayudar` · `ES-C35-gustar` | `VERB-TAKE` · `VERB-ASK` · `VERB-HELP` · `VERB-LIKE-LOVE` |

Two of these are Spanish's own signature, and both are cashed cheques rather
than new debt:

- **pensar and entender are stem-changers** (*pienso*, *entiendo*). Chapter 11
  taught "verbs that break in the middle" as a sound-law; these are the first
  chapter to spend it, and both lessons name *querer* as the pattern they are
  reusing rather than re-teaching the boot.
- **gustar is inverted.** *Me gusta el libro* is "the book pleases me": the
  thing liked is the **subject**, which is why the verb agrees with it
  (*me gustan los libros*) and never with the speaker. It is the single hardest
  structural surprise Spanish holds for an English speaker, so it gets a whole
  lesson and closes the chapter as its payoff. The lesson also reaches back to
  *mucho gusto* from Chapter 3 — same *gustus*, same "taste".

Etymology, as always, is the hook: *pēnsāre* "to weigh" (pensive, compensate,
expense, peso, pansy); *intendere* "to stretch toward" (intend, intense,
attention, tendon, tent), with Spanish **pretender** flagged as a false friend;
*legere* "to gather" (legible, legend, lecture, lesson, collect, select, elect,
intellect), with Greek *légein* marked a cousin and not a parent; *scrībere*
"to scratch" (scribe, script, describe, manuscript), plus the propped-up **e-**
rule that also gives *escuela* and *España*; *adiūtāre*, which is English
**aid** arriving by the other road. Two honest gaps are stated as gaps:
**tomar** has no agreed etymology at all, and **preguntar** left English
nothing — what survives is its picture, sounding water with a pole.

Two practical warnings a beginner needs early: *coger* is the ordinary word in
Spain and coarse slang across much of Latin America, which is why *tomar* is
taught as the safe verb; and English's one *ask* splits into *preguntar* (a
question) and *pedir* (a thing).

Wiring: `curriculum.json` gains `ES-PATH-031` on `SPINE-SAY-WHAT-I-DO` and drops
the eight concepts from that node's `omits`; `chapters.json`,
`core/book-generation.json`, `book.tex` and the generated `ch34`/`ch35` TeX and
narration follow. All eight lessons are schema v2, `voice` modality, and
computed at 228-282 s — the chapter drivable prefix is 4 of 4 for both chapters.

## HL-C18 gentle ramp — the fifteen over-budget lessons, split

`core/chapter-policy.json` caps a lesson at `maxNewAtomsPerLesson: 3`, measured at
the corpus's own p90. Fifteen Spanish lessons exceeded it, led by
`ES-C31-numeros-11-20` at **seven** — ten numbers in one sitting, exactly the
"drill a list of ten greetings" model HL00 rejects. All fifteen are now split into
**thirty-three** prerequisite-ordered micro-lessons (net +18 lessons). Nothing was
waived and no atom was re-declared away: every atom the original taught is still
introduced, once, by whichever half now owns it.

Each boundary follows a seam in the language rather than an atom count:

| Was | Becomes | The seam |
|---|---|---|
| `ES-C31-numeros-11-20` (7) | `once-quince` · `dieciseis-diecinueve` · `teens-latinos` · `veinte` | 11–15 are **fused** Latin compounds you must remember; 16–19 are **transparent** *dieci-* + digit you can generate. Latin's own subtractive *duodēvīgintī*/*ūndēvīgintī* get their own lesson, and *veinte* was worn but never fused. |
| `ES-C30-el-tiempo` (6) | `el-tiempo` · `hace-calor` · `llueve` | *tiempo* the double-sense noun; then the **nouns** *calor/frío/sol* that need *hacer*; then *llueve*, the impersonal verb that refuses the *hacer* frame. |
| `ES-C03-tu-usted` (5) | `tu-usted` · `tu-usted-register` | The two pronouns, then the **register** choice and the third-person agreement that follows from *usted*'s origin as a noun phrase. |
| `ES-C06-hablar` (5) | `hablar` · `ar-presente` | One word and its *f-* → *h-* history, then the **productive** *-ar* paradigm and pro-drop. |
| `ES-C22-negro-blanco` (5) | `negro-blanco` · `blanco-germanico` | The two colour words, then the **borrowing event** and the retreat of Latin *albus*. |
| `ES-C26-agua-vino` (5) | `agua` · `vino` | *agua* carries the stressed-*a* article rule; *vino* carries none. |
| `ES-W02-enye` (5) | `enye` · `enye-formas` | The letter, sound and the scribal *-nn-*; then *mañana*/*español*, which reach ñ by the **other** road, Latin *-ni-*. |
| `ES-C03-como` (4) | `como` · `como-acento` | The word eroded from *quo modo*; then Spanish's **question-marking orthography** (¿ and the diacritic). |
| `ES-C04-estar` (4) | `estar` · `estar-estado` | The verb and *stāre*; then what *estar* is for and its two forms. |
| `ES-C06-espanol` (4) | `espanol` · `hablo-espanol` | The noun from *Hispania*; then the assembled sentence and the bare-language rule. |
| `ES-C20-lo-siento` (4) | `lo-siento` · `perdon` | Two different words, two different roots, two different pragmatic jobs. |
| `ES-C22-rojo-azul` (4) | `rojo` · `azul` | Inherited via *russus*; borrowed from Arabic *lāzaward*. |
| `ES-C23-hermano-hermana` (4) | `hermano-hermana` · `hermano-hache` | The *germen* etymology; then the silent *h*, which is **not** the *hijo*/*hacer* *h*. |
| `ES-C32-perro-gato` (4) | `gato` · `perro` | A fully sourced etymology against a genuinely unsolved one. |
| `ES-C33-verde-amarillo` (4) | `verde` · `amarillo` | Named after a colour verb; named after a **taste**. |

Consequences:

- Five paired lessons were renamed to single-word ids (`ES-C22-rojo`, `ES-C26-agua`,
  `ES-C31-once-quince`, `ES-C32-gato`, `ES-C33-verde`), because keeping a two-word id
  on a one-word lesson would have been a lie in the filename.
- `curriculum.json` gains the eighteen new lessons in prerequisite-safe positions and
  four new extension nodes (`ES-EXT-007-REGISTER`, `ES-EXT-008-SCRIPT`,
  `ES-EXT-011-GRAMMAR`, `ES-EXT-018-LANGUAGE-SPECIFIC`).
- `chapters.json` repoints seven payoffs at the real terminal lesson: Chapters 20, 22,
  23, 30, 31, 32 and 33. Each `assesses` list is still a subset of that lesson's own
  `practises.knowledge`.
- Every split lesson computes **well under 300 effective seconds** (maximum 275,
  `ES-C03-como-acento`), and each declared `max_seconds` is at least the computed cost.
- Spanish over-budget count: **15 → 0**. Corpus-wide: 52 → 37, with the maximum
  dropping from 7 to 6.

## HL05 chapter capabilities — Chapters 4–6 and 19–33

- Extended `chapters.json` from 3 to 21 entries, covering every Spanish chapter
  that owns a `core/book-generation.json` target. Chapters 1–3 are untouched.
- Each new entry declares a first-person `canDo`, the shared spine nodes the
  chapter realises (derived from `curriculum.json` path segments), and a
  `payoff` naming the lesson that proves the claim, its kind, a one-line
  summary, and the knowledge atoms it exercises. Every `assesses` list is
  exactly the payoff lesson's own `practises.knowledge` — nothing invented.
- Payoff selection: Chapters 4–6 use their terminal `practice-mix`. Chapters
  19–33 have no practice lesson, so the payoff is the chapter's last lesson by
  sequence, which is where its recombination and wrap-up recall live.
- **Skipped, deliberately:** Chapters 7–18 (twelve chapters). Their lessons are
  still schema v1 with no declared `practises.knowledge`, so no payoff can be
  claimed honestly. They also own no book-generation target. The absence is
  tracked debt; a stub would have destroyed the HL05 gap report's signal.
- **Representativeness risks** measured against the 0.5 threshold in
  `core/chapter-policy.json` — payoff atoms over atoms the chapter introduces:
  Chapter 3 (0.25, 6/24, already known), Chapter 4 (0.32, 10/31), Chapter 5
  (0.29, 5/17), Chapter 6 (0.47, 9/19), Chapter 26 (0.38, 3/8). These are
  genuine split candidates — chapters that introduce far more than their payoff
  exercises — and are recorded rather than papered over by inflating `assesses`.
- Rewrote the ledger's `note` to describe what is authored, what is skipped, and
  why, replacing the placeholder that predated this work.

## Complete book — clean, portable print layout

- Replaced fixed-width legacy grammar tables with width-aware columns, split
  dense recap prose into scannable lists, and gave long chapter and section
  titles compact running-head and bookmark forms.
- Added portable Latin Modern small caps, natural page bottoms, a modest
  line-break reserve, and truly empty open-right chapter versos.
- Reworked the canonical Chapter 21 weekday recall as four prerequisite-safe
  prompts; regenerated book content and Language Ladder's independent source
  fingerprint from that same lesson.
- Forced the complete book to 214 pages with zero missing glyphs, overfull or
  underfull boxes, duplicate destinations, Hyperref warnings, LaTeX warnings,
  or font warnings. All rendered pages, including Arabic **لازورد** and all 19
  intentionally blank physical pages, were visually inspected.

## Chapters 19–33 — canonical app/book publication

- Migrated all 21 existing lessons from *sí / no* through *verde / amarillo*
  to the executable schema-v2 contract: shared-spine placement, stable order,
  explicit prerequisite and knowledge flow, typed teaching blocks, and honest
  sub-five-minute duration budgets.
- Added the missing grammatical-gender prerequisites to *mano* and *agua /
  vino*, so each new step depends only on material the learner has already
  completed.
- Generated fifteen new LaTeX chapters from the same canonical lesson ASTs
  consumed by Language Ladder. The Spanish book now reaches Chapter 33, and
  the corpus-wide lesson-to-book chapter gap is zero.
- Added repository-backed inline Arabic shaping for the **لازورد** etymology in
  the colors chapter, preserving the original script without a machine-local
  font dependency.
- Forced and visually inspected the complete 210-page book. Metadata, outline,
  generated hashes, Unicode coverage, and destinations pass; the legacy layout
  warning baseline is recorded as the next focused cleanup tranche.

## Chapters 1–6 — block-boundary prompt closure

- Declared introductions and assessments at every typed block boundary across
  all 51 schema-v2 lessons. Guided production and recall now fail validation if
  they ask for an atom outside the lesson's transitive knowledge frontier.
- Made the boundary order executable: a block is checked before its own
  introductions become available, and every lesson-level introduction and
  practice atom must be accounted for by the body AST.
- Removed early production of *muy bien*, *¿y usted?*, *el gusto es mío*, and
  later preview forms; promoted *te llamas* and *gusto* to explicit atoms; and
  completed the practice declarations found by the editorial pass.
- Regenerated all six chapter fingerprints without exposing metadata comments
  in the printed book or Language Ladder lesson copy.

## Chapters 4–6 — canonical book generation

- Replaced three handwritten LaTeX chapters with deterministic output from all
  27 schema-v2 well-being, writing, farewell, and first-verb micro-lessons.
  Language Ladder now verifies the same source hashes across all 51 lessons in
  generated Chapters 1–6.
- Added width-aware Markdown-table rendering so question families, register
  choices, farewell timing, and verb conjugations keep their row-and-column
  meaning in print. The renderer also emits a safe LaTeX approximation symbol.
- Corrected two headerless canonical verb tables and removed three unsupported
  glyphs exposed by the first forced build without weakening the etymology.
- Forced the complete XeLaTeX build to 158 pages and visually inspected every
  page in Chapters 4–6. The generated span has no missing glyphs, layout-box
  warnings, bookmark warnings, clipping, collisions, or malformed tables.

## Chapters 4–6 — prerequisite-safe schema-v2 content

- Migrated all 25 existing lessons across well-being, farewells, first verbs,
  accents, eñe, and question punctuation to the same typed schema consumed by
  Language Ladder and canonical book generation.
- Added two gentle prerequisite-repair lessons for **y / ¿y tú?** and **café**.
  Later dialogue no longer asks learners to produce these words—or *sí*, *un
  poco*, *ojalá*, and other future material—before teaching them.
- Assigned unique sequences 250–510 and explicit knowledge, coverage,
  register, variety, and sub-five-minute duration metadata to all 27 lessons.
  The full Spanish track now has 51 schema-v2 lessons and 77 legacy lessons.
- Classified `Script` sections as typed canonical blocks, preserving the inline
  accent, eñe, and inverted-question ramp for both app and future generated-book
  renderers. The LaTeX swap remains intentionally queued as HL-G03.

## Every Spanish lesson now fits a sub-five-minute step

- Removed all 55 remaining Spanish duration violations and brought the full
  20-language corpus to zero. Forty-one lessons already computed below the
  limit and now declare honest four-minute budgets; six borderline lessons lost
  only duplicated recap prose.
- Split the genuinely deep material into twelve prerequisite-ordered support
  lessons. The new steps isolate subjunctive formation, inherited stems,
  outliers, name history, two-subject traps, *ojalá*, three practice lenses,
  formal/informal register, *hasta* limits, future conjecture, written accents,
  and question-span punctuation without discarding their etymology or grammar.
- Kept the schema-v2 Chapters 1–3 pilot unchanged and validated every rewritten
  or new lesson below 300 effective seconds. The data package and Language
  Ladder now consume 1,063 canonical lessons with no unknown prerequisites.
- Forced and visually audited the 138-page Spanish XeLaTeX book. Its long-form
  Chapter 18 still preserves the combined narrative; the backlog now records
  canonical generation for Chapters 4–6, publication of Chapters 19–33, and
  the pre-existing layout, bookmark, and font warnings.

## Chapters 2–3 — complete the generated schema-v2 pilot

- Added the remaining 17 migrated lessons to canonical book generation, so all
  24 Chapter 1–3 lesson ASTs now drive both the LaTeX book and Language Ladder.
- Extended app/book hash checks across five Chapter 2 and twelve Chapter 3
  lessons and grew the compiled book to 138 pages.
- Fixed nested Markdown emphasis, long practice-list wrapping, and arrow-bearing
  bookmark titles in the shared renderer; visually checked both generated
  chapter openers and representative grammar, etymology, practice, and recall
  pages.

## Chapter 1 — canonical book generation

- Replaced the handwritten first chapter with deterministic LaTeX rendered
  from its seven schema-v2 lesson ASTs in prerequisite-safe sequence order.
- Added a committed FNV-1a source fingerprint manifest shared with Language
  Ladder, plus a CI drift check that rejects stale generated TeX.
- Built the 122-page XeLaTeX book and visually checked the generated opener,
  grammar, culture, guided-practice, and recall layouts.

## Chapters 1–3 — executable schema-v2 pilot

- Migrated the first three chapters to the HL04 lesson contract: canonical
  spine nodes, stable sequence values, strict sub-five-minute duration budgets,
  explicit skill/mode/strand metadata, and declared knowledge flow.
- Split grammatical gender, the Latin *qu-* question family, and the origin of
  *usted* into three prerequisite-ordered support lessons. The etymology stays
  deep while every individual lesson remains independently resumable.
- Removed the two later-chapter prerequisite roots in Chapter 3 and made all 24
  lessons close over knowledge introduced by their transitive prerequisites.
- Exposed every level-two teaching section as a typed block consumed from the
  same lossless Markdown body by downstream book and app renderers.

## The book catches up — Chapters 4–18 typeset

The lessons had run ahead of the published artifact: 90 authored lessons through
Chapter 18, but the LaTeX book still stopped at Chapter 3 ("Introducing
Yourself"). This closes that gap — **fifteen new book chapters**, written from
the existing `ES-C04`–`ES-C18` lessons and wired into `book.tex`:

- **Ch4** How Are You · **Ch5** Farewells · **Ch6** The First Verbs ·
  **Ch7** The *-er* and *-ir* Verbs · **Ch8** Numbers and Age
- **Ch9** *Ser* and *Estar* · **Ch10** Going, and the Near Future ·
  **Ch11** Stem-Changing Verbs · **Ch12** The *-go* Verb Club ·
  **Ch13** Completing the *-go* Club
- **Ch14** The Preterite · **Ch15** Completing the Preterite ·
  **Ch16** The Imperfect · **Ch17** The Future and the Conditional ·
  **Ch18** The Subjunctive

Each chapter follows the established book conventions: one `\section` per
lesson with a slug `\label`, the `cousinweb` / `grammarlens` / `culture` /
`sounds` boxes, `booktabs` conjugation tables, and every atom traced to its
root. Content is faithful to the lessons — no new etymologies introduced.

The book grows from ~20 pages to **114 pages**; compiles clean with XeLaTeX
(0 errors, 0 missing characters, 0 undefined references, 0 duplicate labels)
and was rasterized and visually QA'd. Practice-section labels are
chapter-qualified (`lesson:chN-practice`) to keep them unique.

Also fixed: a corrupted cognate line in `ES-C04-gracias.md` ("French
**grâce/merci-less** *grâces*") now reads as the intended claim — the Romance
family kept *grātia* even where everyday thanks went elsewhere.

## Chapter 18 — The subjunctive, and how much of it you already knew

- **Chapter 18 authored** (`ES-C18-subjuntivo`, `-quiero-que`, `-practice`) — the
  mood the Ch. 17 practice lesson teased, and the first one this course teaches.
- **The present subjunctive** (`ES-C18-subjuntivo`): built **from the *yo* form
  of the present**, not the infinitive — drop the *-o*, then **flip the vowel**
  (*-ar* verbs take *-e-*, *-er/-ir* take *-a-*; the two families **swap
  costumes**). *-er* and *-ir* are identical here, as they already were in the
  Ch. 15 preterite and the Ch. 16 imperfect — stated without an ordinal, since
  Ch. 16's own "third time" count is muddled and Ch. 17 collapses all **three**
  conjugations anyway.
  - The lesson opens by naming the **indicative** as a mood rather than claiming
    earlier tenses stated facts: predictions and imagined outcomes can still be
    presented from the speaker's asserted point of view.
  - **Deriving it from *yo* is the whole pedagogical point**, because every
    irregular *yo* form is then inherited **for free**. The ***-go* club** of
    Chapters **12–13** — six verbs already learned — transfers whole as *tenga,
    diga, haga, ponga, salga, venga*, with **zero new memorisation**: the *-g-*
    was already in the form. Chapter 11's stem-changers come along too (*quiero →
    quiera*, *puedo → pueda*), with the caution that *nosotros* keeps the plain
    stem (*queramos*, not \**quieramos*), for the same stress reason as the
    present.
  - **Where the recipe runs out**, stated as a property of the recipe rather than
    a list to memorise: it needs a strippable *-o*, and **five** taught verbs
    don't have one (*soy, voy, sé, he, **estoy***). Four are then unpredictable
    and learned outright — **sea, vaya, sepa, haya** — while ***estar* is not**:
    remove the whole *-oy* and the stem ***est-*** is completely ordinary
    (*esté, estés, esté, estemos, estén*). An earlier draft put *estar* on the
    "rule works perfectly" side while defining the rule as "drop the *-o*", which
    would have produced \**esty*.
  - ***Estar*'s real irregularity is stress, not spelling** — saying "only the
    accent is special" would have taught a learner to say \**ES-te*. It is
    explained via the rule the repo already documents
    (`stress-default-vowel-ns`, now cited in both lessons' `sounds:`): Spanish
    stresses the **second-to-last** syllable of a word ending in a vowel, *-n* or
    *-s*. *Hablar* obeys it throughout — **HA**-ble, ha-**BLE**-mos,
    **HA**-blen — while es-**TÉ**/es-**TÉS**/es-**TÉN** land on the **last**
    syllable and therefore need a written accent, and es-**TE**-mos obeys the
    default and takes none. The accents are the **receipt for a broken default**.
    (An intermediate draft said "regular *-ar* verbs stress the stem," which is
    false for *hablemos* — ha-**BLE**-mos — a form sitting in this lesson's own
    paradigm table.)
  - ***Dar*** is named on *estar*'s side (*doy* → *d-* → *dé, des, demos, den*),
    since *doy* appears four times in Ch. 9–10 in the *soy/estoy/doy* quartet and
    omitting it from a list naming *soy, voy, estoy* would be conspicuous. But
    the lesson is careful to say its lone accent is **not** a stress mark — *dé*
    is monosyllabic, and the tilde is the ***qué*/*que*** diacritic of Ch. 7
    separating it from the preposition *de*. Offering *dar* as a stress parallel
    would have contradicted the sentence immediately above it.
  - *Sea* gets a five-word payoff rather than a bare form: Ch. 9 already told the
    learner that *ser* had ***sedēre*** "to sit" folded into it, and *sea* is that
    verb's subjunctive (*sedeam*) — the wanderer turning up again.
  - The lesson also marks its own shortcut honestly: "no *-o* to strip" is a
    **memory aid for the modern language, not history** — *sea, vaya, sepa, haya*
    descend from *sedeam, vādam, sapiam, habeam*, and *saber*/*haber* obviously
    do have stems (*sab-*, *hab-*) outside the *yo* form.
  - Etymology: *subiūnctīvus*, "**joined underneath**" — *sub-* + *iungere*,
    which **descends** into English **join**, **junction**, **conjunction**.
    **Yoke** (native, OE *geoc*) and **yoga** (Sanskrit *yuj-*) are its
    **cousins**, not its children — all three from PIE \**yewg-*, and the lesson
    says so rather than lumping them together. Latin coined the term to render
    Greek *hypotaktikḗ*, "subordinated." The name describes **where the mood
    lives** — in a clause hanging under another — not what it means, which sets
    up the next lesson structurally rather than as trivia.
- **The trigger** (`ES-C18-quiero-que`): the subjunctive appears when **two
  different subjects** are joined by *que*. *Quiero hablar* (one subject →
  infinitive) against *Quiero **que hables*** (two → *que* + subjunctive). Set
  against reporting, so the contrast is about meaning rather than a verb list:
  *Sé que **hablas*** stays indicative because **knowing reports the world**,
  while **wanting tries to change it** — same *que*, same person, different mood.
  - **The English trap is named outright**, since it's the commonest
    English-speaker error: English's *"I want **you to speak**"* (object +
    infinitive) has no Spanish equivalent **after verbs of wanting** — there is
    no \**quiero te hablar*, and the full clause is obligatory. Scoped that way
    deliberately: Spanish allows object + infinitive freely after **perception
    and causative** verbs (*te vi salir*, *me hizo hablar*, *déjame hablar*), so
    a blanket "Spanish can't do this" would be false. The lesson also notes that
    *quiero hablarte* is perfectly good — it just means "I want to speak **to
    you**," one subject — so the learner doesn't over-read the ban as a clitic
    rule.
  - ***Ojalá***, which takes the subjunctive **always**, and needs **no *que* and
    no main verb** — because the word *is* the wish. From Andalusi Arabic ***law
    šāʾ Allāh***, "if God should will": the **Allāh is still in there** every
    time it's said. That makes it the **second Arabic function word** in the
    course, beside Ch. 5's *hasta* ← *ḥattā* — and the point is made precisely:
    borrowing nouns is ordinary (*azúcar, aceite, álgebra*), borrowing a word
    that **forces a grammatical mood** is not. Scoped to "the two this course
    will teach you," since Spanish has a few other marginal Arabic function words.
  - **`ES-C05-hasta.md` corrected in the same change.** Ch. 5 gave *ojalá* a
    **different** etymology (*wa-šā' allāh*) and filed it in a list of borrowed
    **nouns** — so Ch. 18 would have silently contradicted the very lesson it
    cross-references, and undercut its own "function word" framing. Ch. 5 now
    carries the DRAE etymology (*law šāʾ Allāh*), pulls *ojalá* out of the noun
    list, and points forward to Ch. 18. Found by checking a cross-reference
    instead of assuming it.
- **Practice** (`ES-C18-practice`, `type: practice-mix`): drills the flip
  (deliberately ending on the four verbs where **no new stem was ever learned**),
  the two-subject test as matched pairs, and fact-vs-wish under the same *que* as
  **true minimal pairs** — same verb, one mood apart (*Sé que **comes** aquí* /
  *Quiero que **comas** aquí*; *Sé que está* / *Ojalá esté*; *Dice que viene* /
  *Quiero que venga*). Closes
  on a Ch. 14 callback: **hable**/**hablé** and **trabaje**/**trabajé** are
  different words, separated by nothing but the written accent — which is what
  Ch. 14 meant by the accent *carrying* the tense.
- Taxonomy: namespaced `ES-SUBJUNCTIVE-PRESENT` and `ES-SUBJUNCTIVE-TRIGGER`.
  The practice lesson uses the bare label **`CH18-PRACTICE`** with **no
  dictionary entry**, per the taxonomy's own note that non-word lessons carry a
  session label rather than a concept; `CH18-PRACTICE` is appended to that note's
  list. (An earlier draft added a namespaced `ES-CH18-PRACTICE` *entry*, which
  would have made the file contradict its own documented convention.)
- Roadmap: Ch. 18 authored, next = Ch. 19 (description).
- All **11** distinct referenced `prerequisites` / `reviews_of` ids verified to
  exist before authoring.
- `sounds:` ids now come from `spanish/pronunciation-reference.md`, which declares
  itself the registry — an earlier draft invented `vowel-flip` / `stress-shift`,
  which existed nowhere (and a vowel *flip* is morphology, not a sound).

## Chapter 17 — The future and the conditional, or one weld twice

Chapter 17 now uses schema v2 throughout: explicit order, sub-five-minute
duration, typed prerequisite closure, shared-spine placement, and twenty-eight
objective answer contracts. Seven teaching lessons introduce twelve atoms,
never more than three at once; the terminal checkpoint introduces none and
retrieves the whole chapter.

The migration replaces full five-person paradigms with a singular-person ramp.
Learners receive **hablaré, hablarás, hablará**; then **comeré, comerás,
comerá**; then **viviré, vivirás, vivirá**. The conditional repeats that gentle
order with **hablaría**, **comería**, and **viviría**. Every plural form waits,
and each regular verb family gets its own retrieval step before comparison.

Only three already-known irregular verbs enter the chapter: **hacer** uses
**har-**, **tener** uses **tendr-**, and **poder** uses **podr-**. Each stem is
practised with the same learned future and conditional endings. The other seven
stems from the old inventory, including untaught production of *saber* and
*haber*, remain deferred rather than appearing inside one crowded checkpoint.

Examples stay within the declared corpus and natural combinations:
**Hablaré español**, **Beberé café**, **Viviré en Madrid**, **Haría café**, and
**Podría hablar español**. The old politeness, object-pronoun,
future-in-the-past, Portuguese mesoclisis, and clock-time conjecture examples
wait for their own prerequisites instead of being translated around gaps.

The etymological bridge is also more careful. Romance future and conditional
forms grew from constructions pairing an infinitive with forms of Latin
*habēre*: present material contributed to the future and past or imperfect
material to the conditional. The lessons acknowledge competing forms and
layered sound change rather than saying one Latin future simply died, treating
modern *haber* as a suffix table, or presenting the modern forms as a one-step
weld. The terminal checkpoint maps all twelve atoms and adds nothing new.

## Chapter 16 — The imperfect, and choosing between the two pasts

Chapter 16 now uses schema v2 throughout: explicit order, sub-five-minute
duration, typed prerequisite closure, shared-spine placement, and twenty-eight
objective answer contracts. Seven teaching lessons introduce twelve atoms,
never more than three at once; the terminal checkpoint introduces none and
retrieves the whole chapter.

The migration replaces three full five-person paradigms followed by three more
with a singular-person ramp. Learners receive **hablaba, hablabas, hablaba**;
then **comía, comías, comía**; then **vivía, vivías, vivía**. Only after those
regular rows does the chapter add **era**, **iba**, and **veía**, one short set
at a time. Every plural form waits.

**Ver** is no longer smuggled into an “irregulars” table as though it were known.
A dedicated lesson first teaches the infinitive, **veo, ves, ve**, and its path
through Old Spanish *veer* from Latin *vidēre*. The later imperfect lesson can
therefore explain **veía** by retrieving an owned verb and the already-learned
**-ía** row.

The tense contrast now stays inside the declared corpus: **Hablé/Hablaba
español**, **Viví/Vivía en Madrid**, and **Fui/Iba a Madrid**. The former
*ayer, siempre, mientras*, child, school, beach, concert, María, Juan,
*saber, conocer, llegar, llamar,* and *querer* examples are deferred instead of
being translated around a prerequisite gap.

The historical account is correspondingly more careful. **-aba** continues the
Latin *-ābam* family, while **-ía** reflects several Romance changes to
*-ēbam* material rather than one letter simply wearing away. **Era** continues
*eram*; **iba** continues *ībam* from *īre*; and the learned *voy/fui/iba*
contrast illustrates several sources without claiming that every form of *ir*
fits a tidy three-source box. Absolute claims about the full irregular inventory
also wait beyond this bounded singular lesson.

## Chapter 15 — Completing the preterite

- **Chapter 15 authored** (`ES-C15-comer-vivir-preterite`, `-preterite-fuertes`,
  `-practice`): finishes the everyday past tense opened in Ch.14, reviewing
  Ch.4/7/8/12/14 via `reviews_of`.
- **the regular -er/-ir preterite** (`ES-C15-comer-vivir-preterite`):
  *comí/comiste/comió/comimos/comieron*, *viví/viviste/vivió*. The point worth
  making explicit — **-er and -ir share ONE set of preterite endings**, though in
  the **present** they pull apart (*comemos* vs *vivimos*): two conjugations
  **merge** in the past, a genuine simplification. ← Latin perfect *-ī/-istī/-it*,
  where *-istī* → *-iste* survives almost intact. Trap kept from Ch.14: *vivimos*
  is present **and** preterite, but *comemos* ≠ *comimos*.
- **the strong preterites** (`ES-C15-preterite-fuertes`): *tener→tuve*,
  *hacer→hice*, *estar→estuve*. The stress moves **off** the ending and back into
  the **stem** (*TUve*, *HIce*) — so the written accent **vanishes**, the exact
  reverse of Ch.14's *hablÉ/hablÓ*; the *yo* ending is **-e**, not *-é*. Also
  *hizo*'s **c→z** (the letter changes so the sound won't). Etymology: these are
  Latin's **strong perfects** (*tenuī*, *fēcī*, *stetī*) inherited whole instead of
  rebuilt — irregular precisely because they were too common to be re-made; *tuv-*
  and *estuv-* even share the *-uv-* shape.
- **practice** — the full five-row map (*-ar*, *-er*, *-ir*, strong, *ser/ir*) and
  a single diagnostic: **where does the stress land?** Ending → regular, accent on
  *yo*/*él*; stem → strong, no accent. Re-drills the tense-ambiguous *nosotros*
  forms and *fui* = "I was"/"I went."
- Taxonomy: namespaced `ES-PRETERITE-ER-IR`, `ES-PRETERITE-STRONG`; practice label
  `CH15-PRACTICE`.

## Chapter 14 — The preterite (the first past tense)

- **Chapter 14 authored** (`ES-C14-ser-ir-preterite`, `-hablar-preterite`,
  `-practice`): the course's **first past tense**, the everyday **preterite** —
  reviewing Ch.6/9/10/13 via `reviews_of`.
- **ser & ir — one shared preterite** (`ES-C14-ser-ir-preterite`):
  *fui/fuiste/fue/fuimos/fueron* serves **both** verbs, so *fui* = "I **was**" **and**
  "I **went**," disambiguated only by **context** (*fui a Madrid* = went; *fui
  profesor* = was). The etymology: *fuī* is the perfect of a **second** Latin "be"
  root (PIE *\*bʰuH-* "become," cousin of English *be/been/future/physics*) grafted
  onto *sum/es/est* (**suppletion**, like *go/went*); *ir* then borrowed it after
  Latin's own past of *īre* wore away — so **"to be" and "to go" merged in the past.**
- **the regular -ar preterite** (`ES-C14-hablar-preterite`):
  *hablé/hablaste/habló/hablamos/hablaron* ← Latin perfect *-āvī* (*amāvī→amé*), the
  *-āv-* dissolving to leave a **stressed final vowel** the accent now marks. The
  key insight: the **accent carries the tense** (*HAblo* "I speak" vs *hablÓ* "s/he
  spoke"); plus the *hablamos* present=preterite trap (context / *ayer* disambiguates).
- **practice** — switches present↔past, drills the accent-flip, and the *ser*-vs-*ir*
  reading of *fui/fue*.
- Taxonomy: namespaced `ES-PRETERITE-SER-IR`, `ES-PRETERITE-AR`; practice label
  `CH14-PRACTICE`.

## Chapter 13 — Completing the -go club

- **Chapter 13 authored** (`ES-C13-poner`, `-salir`, `-venir`, `-practice`):
  closes the *-go* club opened in Ch.12, reviewing Ch.8/11/12 via `reviews_of`.
- **poner** ("to put/place") — plain *-go* yo-form **pongo**; ← *pōnere* →
  position/compose/deposit/postpone/component; everyday *poner la mesa* ("set the
  table") and reflexive *ponerse* ("put on / become").
- **salir** ("to leave/go out") — *-go* yo-form **salgo**; ← *salīre* "**to
  leap**" → salient/sally/somersault, and *salmō* "the leaper" (**salmon**); *la
  salida* = "the **exit**." Notes that English *exit* is the rival *exīre*, not
  this leaping root.
- **venir** ("to come") — the **doubly irregular** finale: *-go* yo-form
  **vengo** **and** the **e→ie** stem-change (*vienes/viene/vienen*, *venimos*
  escaping the boot) — the exact mirror of *tener*. ← *venīre* →
  adventure/event/convene/invent/avenue/souvenir.
- **practice** — the full six-member roll-call (*tengo/hago/digo/pongo/salgo/
  vengo*), sorted into plain-*-go* / *-go*+e→i / *-go*+e→ie, plus the *tener*↔*venir*
  boot twins.
- Taxonomy: namespaced `ES-VERB-PONER`, `ES-VERB-SALIR`, `ES-VERB-VENIR`; practice
  label `CH13-PRACTICE`.

## Chapter 12 — Doing, making, saying, and the -go club

- **Chapter 12 authored** (`ES-C12-hacer`, `-decir`, `-yo-go`, `-practice`): two
  workhorse irregular verbs plus the pattern that unites their *yo*-forms —
  reviewing Ch.8/10/11 via `reviews_of`.
- **hacer** ("to do/make") — covers **both** English "do" and "make"; the Latin
  **f→h** softening (*facere → hacer*, the same change as Ch.6 *hablar/hijo*), the
  **-go** *yo*-form **hago** (echoing *tengo*), and the everyday weather idiom
  *hace calor/frío/sol* ("it **makes** heat…"). Root payoff: *facere* →
  fact/factory/perfect/manufacture/satisfy/affect.
- **decir** ("to say/tell") — the **doubly irregular** verb: it stacks the **-go**
  *yo*-form (**digo**) **and** an **e→i** stem-change (*dices/dice/dicen*, with
  *decimos* escaping the boot). Root payoff: *dīcere* →
  dictate/diction/dictionary/predict/verdict/contradict. Anchored on the learner's
  most useful question, *¿Cómo se dice…?*
- **-go club** (`ES-C12-yo-go`) — a pattern lesson: *tengo/hago/digo/pongo/salgo/
  vengo* form a small, closed set whose *yo*-form grows a hard *g* (yo-only, the
  rest regular). Framed as a Latin fossil kept by the oldest, most-used verbs —
  three met (*tengo/hago/digo*), three flagged for Ch.13 (*pongo/salgo/vengo*).
- Taxonomy: namespaced `ES-VERB-HACER`, `ES-VERB-DECIR`, `ES-YO-GO`; practice
  label `CH12-PRACTICE`.

## Chapter 11 — Wants, ability, and the stem-change boot

- **Chapter 11 authored** (`ES-C11-querer`, `-poder`, `-stem-changes`, `-nuestro`,
  `-practice`): the two most useful modal-ish verbs plus the stem-change rule that
  governs a whole class of Spanish verbs — reviewing Ch.8–10 via `reviews_of`.
- **querer** ("to want/love") — the **e→ie** stem-changer (*quiero/quieres/quiere/
  queremos/quieren*), ← *quaerere* "to seek" (→ query/quest/question/inquire/require/
  conquer/exquisite) — the same *e→ie* crack first met in *tener* (Ch.8).
- **poder** ("to be able/can") — the **o→ue** stem-changer (*puedo/puedes/puede/
  podemos/pueden*), ← *potēre* (→ **power**/potent/possible/potential), with *poder*
  + infinitive for ability (*puedo hablar*).
- **The stem-change rule made explicit**: one sound-law — short **stressed** Latin
  *e* broke to *ie*, short stressed *o* to *ue* — in nouns too (*terra→tierra*,
  *porta→puerta*; *septem→siete*, *novem→nueve*), producing the **"boot"** (all
  forms crack but *nosotros/vosotros*).
- **nuestro/vuestro** ("our/your-pl", ← *noster/voster* → Paternoster/nostrum) — the
  possessive that agrees in **gender AND number** (four forms), contrasted with
  *mi/tu/su* (number only); itself an *o→ue* fossil (*noster→nuestro*).
- Taxonomy: namespaced `ES-VERB-QUERER`, `ES-VERB-PODER`, `ES-STEM-CHANGES`,
  `ES-POSSESSIVE-OUR`; practice label `CH11-PRACTICE`.

## Chapter 10 — The near future and possessives

- **Chapter 10 authored** (`ES-C10-ir`, `-ir-a-futuro`, `-mi-tu-su`, `-practice`):
  the pilot starts running on **rules, not phrases** — reviewing Ch.6–9 via
  `reviews_of`.
- **ir** — "to go," the **most suppletive** verb in Spanish: its infinitive is
  from *īre* (→ exit/transit), but its whole present (*voy/vas/va/vamos/van*) comes
  from a **different** Latin verb, *vādere* "to advance" (→ invade/evade/wade), and
  its past from a **third** (*esse/fuī*). Framed as the exact *go/went* suppletion
  English uses; *voy* wears the *-oy* of *soy/estoy/doy*. Includes *voy a* + place
  (destination; *a* ← *ad*, *a+el→al*).
- **The near future** — *ir a* + infinitive (*voy a hablar* = "I'm going to
  speak"): Spanish builds tomorrow out of the verb "to go," the same metaphor as
  English *going to* and French *je vais parler* — a whole tense almost for free,
  the second verb staying a bare infinitive.
- **mi/tu/su** (← *meus/tuus/suus* → my/thy, and the *-ty* family) — possessives
  that agree in **number** (*mis/tus/sus*), **not gender**; *su* covers his/her/its/
  your-formal/their; and *tu* "your" vs *tú* "you" reactivates the *tilde
  diacrítica* from the writing chapter.
- Taxonomy: namespaced `ES-VERB-IR`, `ES-NEAR-FUTURE`, `ES-POSSESSIVE`; practice
  label `CH10-PRACTICE`.

## Chapter 9 — ser vs estar, head-on

- **Chapter 9 authored** (`ES-C09-ser`, `-ser-vs-estar`, `-soy-de`, `-esta-en`,
  `-practice`): the pilot's **payoff chapter** — the two "to be" verbs sorted for
  good, reviewing *estar* (Ch.4) and *tener* (Ch.8).
- **ser** — the learner's second irregular and the **most irregular verb in
  Spanish**: **suppletive** *soy · eres · es · somos · son*, traced to their
  separate Latin sources (*sum, eris, est, sumus, sunt*) and shown as **cousins of
  English *is / am / are*** (PIE *\*h₁es-*); root *esse* → essence/essential/
  present/absent. Completes the pair opened by *estar* (← *stāre*) in Ch.4.
- **The ser/estar split, derived from the roots**: *essence* (ser) vs
  *estado/standing* (estar) — the mnemonic *ser = essence, estar = estado*. Proven
  with **minimal pairs that flip meaning**: *es aburrido* "is boring" vs *está
  aburrido* "is bored"; *es listo* "clever" vs *está listo* "ready"; *es rico*
  "rich" vs *está rico* "tastes delicious"; *es verde* "green" vs *está verde*
  "unripe."
- **soy de…** (origin, profession, nationality → **ser**, because they define you;
  *de* ← *dē* → English *de-*/depart/deduct; article-dropping *soy profesor*) and
  **está en…** (physical location → **estar**, always; *en* ← *in* = English *in*),
  contrasted through the twin questions *¿De dónde eres?* (ser) vs *¿Dónde estás?*
  (estar) — same *dónde*, meaning-locked verbs.
- Taxonomy: namespaced `ES-VERB-SER`, `ES-SER-VS-ESTAR`, `ES-ORIGIN-SER`,
  `ES-LOCATION-ESTAR`; practice label `CH9-PRACTICE`.

## Chapter 8 — Numbers, having, and age

- **Chapter 8 authored** (`ES-C08-numeros-1-5`, `-numeros-6-10`, `-tener`,
  `-cuantos-anos`, `-practice`): counting, the have-verb, and telling your age —
  reviewing Ch.6–7.
- **Numbers 1–10**, each tied to its English cousins (*uno* ← *ūnus* →
  unit/union; *cinco* ← *quīnque* → quintet; *diez* ← *decem* → decimal), plus the
  fact that **September–December are Latin 7–10** (the Roman year began in March),
  and the **teen fusion** *diez y seis → dieciséis* (built exactly like *sixteen*).
- **tener** — the learner's **first irregular / stem-changing verb** (← *tenēre*
  "to hold" → tenant/tenacious/retain/contain): the *-go* yo-form **tengo** and
  the **e→ie boot pattern** (*tienes/tiene/tienen*, but plain *tenemos*) — the
  single most common Spanish irregularity, introduced by its model.
- **¿Cuántos años tienes?** — the idiom that **age is *had*, not *been*** (*tengo
  veinte años*, never *soy veinte*; shared with French *j'ai … ans* / Italian *ho
  … anni*); *años* ← *annus* brings back the **ñ**; *cuántos* ← *quantus* →
  quantity.
- Taxonomy: namespaced `ES-NUM-1-5`, `ES-NUM-6-10`, `ES-VERB-TENER`,
  `ES-QUESTION-AGE`.

## Chapter 7 — The rest of the present tense, and the first questions

- **Chapter 7 authored** (`ES-C07-comer`, `-vivir`, `-beber`, `-que`, `-donde`,
  `-practice`): completes the **regular present tense** and starts **asking
  questions** — the learner can now hold a real back-and-forth.
- **The -er and -ir families**: *comer* (← *comedere* → edible/comestible) teaches
  the *-o/-es/-e* endings; *vivir* (← *vīvere* → vivid/survive/revive) shows *-ir*
  is **identical to -er in the singular**; *beber* (← *bibere* → beverage/imbibe)
  cements it. The three-family table (*hablo/como/vivo*) makes the whole system
  visible: *-ar* → -o/-as/-a, *-er*/*-ir* → -o/-es/-e.
- **First question words**: *qué* (← *quid* → quiddity/quid-pro-quo) and *dónde*
  (← *de unde* "from whence"), both wearing the **accent** (diacrítica) and opening
  with **¿** — reusing the writing chapter. Real questions land: *¿Qué comes?*,
  *¿Dónde vives?*, with the first preposition *en* in the answers.
- **Taxonomy**: canonical `QUESTION-WHERE` (`core:false`) added alongside
  `QUESTION-WHAT`; namespaced `ES-VERB-COMER/VIVIR/BEBER` documented.

## Chapter 6 — Please, and the first verbs (sentences start to move)

- **Chapter 6 authored** (`ES-C06-por-favor`, `-hablar`, `-trabajar`,
  `-estudiar`, `-espanol`, `-practice`): the pilot's first **grammar-engine**
  chapter. The learner stops reciting fixed phrases and starts **building
  sentences from a pattern**.
- **por favor** completes the courtesy set (gracias / de nada / por favor) — *por*
  ← *prō* + *favor* ← *favēre* (favour/favorite).
- **The regular -ar present tense** is the centrepiece: drop *-ar*, add
  **-o / -as / -a** (…-amos / -an). Taught on **hablar** and cemented on
  **trabajar** and **estudiar** — one template, hundreds of verbs.
- **Deep etymology, as ever**: *hablar* ← *fābulārī* "tell tales" (← *fābula* →
  fable) plus the **f→h sound-law** decoder (*facere*→*hacer*, *farīna*→*harina*,
  *fīlius*→*hijo*); *trabajar* ← *tripaliāre* "to torture" (← *tripalium*) →
  **travail/travel**; *estudiar* ← *studēre* "to be eager" → student/studio.
- **First self-assembled sentence**: *Hablo español* — verb + noun, **pro-drop**
  (no *yo*), no article; *español* ← *Hispania* brings back the **ñ**.
- **Taxonomy**: namespaced `ES-VERB-HABLAR`, `ES-VERB-TRABAJAR`,
  `ES-VERB-ESTUDIAR`, `ES-WORD-ESPANOL` documented; `COURTESY-PLEASE` (canonical,
  already existed) realized for Spanish.

## Chapter 5 — Farewells (and the first Arabic-loanword deep dive)

- **Chapter 5 authored** (`ES-C05-adios`, `-hasta`, `-hasta-luego`,
  `-hasta-manana`, `-hasta-pronto`, `-practice`): closing a conversation, each
  lesson 3–5 min and reviewing Chapter 4. The learner can now open *and* close a
  Spanish conversation end to end.
- **The first Arabic-loanword deep dive**, via **hasta** ← Arabic *ḥattā*: 800
  years of al-Andalus, ~4000 Spanish words from Arabic, and the striking fact
  that *hasta* is a borrowed **function word** (a preposition — languages almost
  never borrow those), with the *al-* noun family (*almohada, azúcar, álgebra*)
  as the visible tip.
- **Etymology throughout**: *adiós* ← *a Dios* "to God" (twin of *goodbye* ←
  "God be with ye"; the adiós/adieu/addio/adeus set); *luego* ← *loco* "place" →
  "then" (local/locus); *mañana* ← *maneana* "early hour" (morning *and*
  tomorrow) — which **reactivates the ñ writing lesson** (the tilde as a frozen
  *nn*); *pronto* ← *promptus* → English *prompt*.
- **Taxonomy**: canonical `FAREWELL-LATER`, `FAREWELL-TOMORROW`, `FAREWELL-SOON`
  (`core:false`) join the existing `FAREWELL`; namespaced `ES-WORD-HASTA`
  documented.

## Writing nuances — the accent, the ñ, the inverted marks

- **First `writing`-type lessons** (`ES-W01-acento`, `ES-W02-enye`,
  `ES-W03-inverted`): orthography taught the same etymology-first way as the
  vocabulary, once enough accented words have accumulated to make it concrete.
  Each names the mark, explains *why* it exists, and says how to draw it.
- **The acute accent** (`á é í ó ú`): Spanish's only accent mark, its two jobs
  (irregular-stress marker + the diacrítica that splits *tu/tú, el/él, si/sí,
  se/sé, mas/más, como/cómo, que/qué*), and the rule that question words carry it.
- **ñ**: the tilde as a frozen medieval shorthand for a doubled *nn* (Latin
  *annus* → *año*, cousin of *annual/anniversary*); ñ as a distinct letter, not
  an accented n.
- **¿ ¡**: why Spanish opens a question/exclamation (word order often doesn't
  change, so the reader needs an early intonation cue — Royal Academy, 1754),
  and that they bracket the *question*, not necessarily the sentence.
- These use the new `writing` lesson type in `@coding-adventures/human-language-data`
  (0.3.0) — exempt from the concept join, no `concept_tag`.

## Chapter 4 — Responding to "¿cómo está usted?"

- **Chapter 4 authored** (`ES-C04-gracias`, `-de-nada`, `-estar`, `-como-esta`,
  `-regular`, `-practice`): the "how are you?" exchange, built atom-first and
  reviewing Chapter 3 throughout. Each lesson stays 3–5 minutes and folds prior
  words back in via `reviews_of`.
- **The two "to be" verbs.** *estar* is introduced as the *temporary* be-verb
  (← Latin *stāre*, "to stand" → stay/state/status/estate) and explicitly
  contrasted with *ser* (identity). "How are you?" is framed as *estar* because
  it asks about your current state.
- **Deep etymology hooks**, per the curriculum's method: *gracias* ← *grātia*
  (grace/grateful/gratis/congratulate); *de nada* ← *(rēs) nāta* "a born thing"
  → "nothing" (cousin of native/nature/natal, mirroring French *rien* ← *rem*);
  *regular* ← *rēgula* "straight rod" (rule/ruler/regulate/rail) and flagged as
  a false friend (so-so, not "normal"); *más o menos* ← *magis* + *minus*.
- **Taxonomy additions**: canonical `COURTESY-YOUREWELCOME`, `STATE-HOW-ARE-YOU`,
  `WORD-SOSO` (all `core:false`), plus namespaced `ES-VERB-ESTAR` documented in
  the examples list, and `CH4-PRACTICE` added to the practice-tag note.

## Slug-based lesson ids + el/la before día

- **Stopped numbering lessons.** Ordinal ids (`ES-C01-L03`) forced a
  renumber cascade on every insertion. Lessons now use **stable slug ids**
  (`ES-C01-dia`), matching filenames; prerequisites/cross-links reference
  slugs; prose refers to other lessons by word ("the *bien* lesson"), not
  "Lesson N". Order lives where a tool counts it: the **book** (LaTeX
  auto-numbers chapters/sections; `\ref` cross-refs self-correct on insert)
  and `session-map.md`. HL00 schema updated to document this.
- **Added an *el* / *la* lesson before *día*.** Gender should be understood
  before nouns, so the two articles — and the grammatical-gender concept,
  traced to Latin demonstratives *ille*/*illa* (which also gave *él*/*ella*)
  — now get their own lesson ahead of the first noun. *día* is slimmed to
  *apply* that gender rather than introduce it from scratch.
- Verified the book still compiles clean (20 pages) with the section
  inserted — LaTeX renumbered and the `\S\ref` cross-references resolved
  automatically.

## Chapters 2-3 + grammatical gender at the first noun

- **Grammatical gender introduced at *día*** (the first noun), not a later
  chapter: *día* now teaches the concept, tags itself masculine (*el día*),
  establishes that every noun from here carries its *el*/*la*, and — per
  feedback — **traces the gender system to Latin** (Latin's three genders →
  Spanish's two, gender usually inherited from the Latin word). *tarde*
  (*la tarde*, fem.) and *noche* (*la noche*, fem.) now carry gender too, and
  every future noun will. HL00's gender methodology updated to match.
- **Chapter 2 — The Rest of the Greetings** (`ES-C02-L01`–`L05`): tarde →
  buenas tardes (introduces **feminine** agreement, *buenas*) → noche →
  buenas noches → practice. Includes the Latin *-ct-*→*-ch-* sound pattern.
- **Chapter 3 — Introducing Yourself** (`ES-C03-L01`–`L10`), covering the
  standard greetings/introductions dialogue atom by atom: me (first pronoun,
  a true English cognate) → llamo (*clamare*; *cl-*→*ll-*) → me llamo
  (**reflexive verbs**) → **tú / usted** (informal vs formal "you," with both
  roots traced: *tú* ~ *thou*; *usted* ← *vuestra merced* "your grace," the
  *merc-* family) → cómo (question word + accent) → se llama (reflexive *se*;
  the *se-* family) → ¿cómo se llama usted? (assembled + answered) → mucho
  (*multus*; *-lt-*→*-ch-*) → mucho gusto (*gustus*) → practice.
- Pronouns traced to roots throughout (per feedback); *tú*/*usted* presented
  as a contrastive pair so the informal/formal distinction is explicit.
- Book: added `ch02-greetings.tex`, `ch03-introductions.tex`; Chapter 1
  *día* section rewritten for gender. Compiles clean (20 pages).

## Chapter 1 re-sequenced: atom-first, compositional

Reworked Chapter 1 so words are built from their pieces, then assembled —
per feedback ("start with Hola; then bien and its origin/English cousins and
how it responds on its own and becomes bueno/buena; then día + plurals →
días; then put them together for buenos días; then practice"):

- `L01` **hola** · `L02` **bien** (adverb "well"; ← Latin *bene* → benefit/
  benevolent/benediction; stands alone as an answer; its adjective sibling
  *bueno/buena* ← *bonus*; the well/good ↔ bien/bueno parallel) · `L03`
  **día** (← *dies*; introduces the **plural rule**, día → días; flags día as
  masculine) · `L04` **buenos días** (assembled from *bueno* + *días*;
  introduces **adjective agreement**, *bueno* → *buenos*; the "may God give
  you good days" blessing) · `L05` **practice** (hola & buenos días).
- Grammar is introduced exactly where a word needs it (plurals at *día*,
  agreement at *buenos días*), never front-loaded.
- Removed the previous, out-of-sequence Chapter 1 lessons (buenos días as a
  monolith, plus tardes/noches/gracias/por favor/adiós/cómo estás/me llamo/
  quiero) — kept in git history; tarde/noche/etc. will be rebuilt atom-first
  in their own later chapters, and Chapter 2 will cover *responding* to a
  greeting (a recursive cycle).
- Book Chapter 1 rewritten to match; compiles clean (12 pages). Book author
  set to **Adhithya Rajasekaran** (title page, copyright, PDF metadata).
- **CI fixed**: the book workflow failed on a non-existent apt package
  (`texlive-lang-indic`); replaced the fragile package list with
  `texlive-full` (every engine/language/font). The workflow now goes green
  and uploads each book as a PDF artifact.

## Redesign: deep one-word lessons

Major reshape after learner feedback ("I can't possibly learn *buenos días*
plus every other greeting in one go … we learn the cultural aspects and why
phrases are idiomatic, not what Spanish 101 teaches"):

- **Granularity**: the unit is now **one word or phrase per lesson**, gone
  deep — not a cluster. New `lessons/` directory, IDs `ES-C<chapter>-L<lesson>`.
- **Chapter 1 — First Words** rebuilt as 10 deep lessons (`ES-C01-L01`–`L10`):
  hola, buenos días, buenas tardes, buenas noches, gracias, por favor, adiós,
  ¿cómo estás?, me llamo, quiero. Each with: inline "Sounds you'll need",
  prerequisite cross-links, the widest honest **web of English cousins**,
  **prefix/suffix construction** taught in context (the *quaerere* →
  inquire/require/acquire/conquer/exquisite table), and a **cultural/idiomatic
  "why"** (e.g. *buenos días* as the fossil of "may God give you good days";
  *adiós* = "to God").
- **Pronunciation is no longer a chapter**: added `pronunciation-reference.md`
  (look-up reference) + inline per-lesson sound notes. The old front-loaded
  Part 0 is gone — feedback was that a sounds chapter makes people quit before
  Lesson 1.
- **HL00 spec** updated to encode all of the above (Lesson Anatomy, the
  Cousin Web + Morphology-In-Context section, "Pronunciation & Script:
  Inline, Never a Gate", new schema) and to **drop forward-references** to
  languages later in the curriculum — each track grounds on English + its own
  deep roots only.
- **Book** rebuilt: pronunciation appendix + new deep Chapter 1; new
  `cousinweb`/`culture`/`sounds`/`prefixtable` callout styles; compiles clean
  with XeLaTeX (15 pages).
- **Legacy**: the pre-redesign `units/` (old Part 0 + Part I) and old book
  chapters are superseded. Old book chapters removed (in git history); old
  `units/` kept as source material to re-excavate into deep lessons.

## Part I complete (Chapters 2-4) — pre-redesign, superseded

- Added Chapter 2 (`units/ES-P0-U09` through `U15`): numbers 11-100
  (including the "diez y seis"→"dieciséis" fusion and its English
  "sixteen" parallel), telling time (*es la una* vs. *son las dos*),
  months & seasons, the seven core question words (and their written-accent
  rule), survival phrases, and a practice-mix capstone.
- Added Chapter 3, the promised gender-mastery chapter (`units/ES-P0-U16`
  through `U21`): articles, the *-o*/*-a* pattern's real exceptions (*el
  día, la mano, el problema, el agua*), adjective agreement, colors
  (including *azul*, another Arabic loanword alongside *cero*), family
  vocabulary (the one place grammatical gender lines up with real-world
  sex), and a practice-mix capstone.
- Added Chapter 4, closing Part I (`units/ES-P0-U22` through `U26`): *hay*,
  *tener* (a first stem-changing verb), possessive adjectives, negation
  (Spanish's mandatory double-negative, a genuine "opposite of the English
  rule" case), and a cumulative Part I review capstone.
- Extended `session-map.md` through session 28 (Part I's end); introduced
  the bonus-queue framing explicitly once review volume exceeds what a
  2-4-item core block can hold, rather than continuing to hand-track every
  single far-future review individually.
- Extended `book/` with `chapters/part1-ch02-numbers-and-time.tex`,
  `part1-ch03-gender-mastery.tex`, `part1-ch04-hay-tener-negation.tex` —
  Part I is now fully typeset, title page through Chapter 4.
- Updated `HL00` with four standing-methodology amendments driven directly
  by learner feedback: just-in-time **script** introduction (no dedicated
  alphabet-review chapter for any track that needs one), the same
  just-in-time principle extended to **grammar** (motivated by the new
  Tamil track), a **frequency-driven content selection** principle, and the
  **Cross-Language Comparison Web** — an accumulating hierarchy where each
  new language compares against every language already established before
  it (Spanish→English/Latin; French→+Spanish; German→+French; Arabic→+German;
  Hindi→+Arabic+Sanskrit; Tamil→+Hindi+Sanskrit; Kannada/Malayalam/Telugu→all
  of the above).
- Added `.github/workflows/human-languages-books.yml`: CI that discovers
  every language's `book/` directory and compiles it to PDF with XeLaTeX,
  uploaded as a build artifact per language.
- Bootstrapped seven more language tracks (French, German, Arabic, Hindi,
  Tamil, Kannada, Malayalam, Telugu) — see each track's own `CHANGELOG.md`.

## Part 0 & Chapter 1 (book/framework expansion)

- Added five standing pillars to `HL00`: a **Grammar Lens** unit section
  (plain-language grammar concept + English contrast), a **`morphology`**
  unit type (lexical Latin roots), a **Part 0** phase (script/sound-system
  introduction, scaled per language), a **Grammatical Gender** methodology
  (nouns tagged from the first one onward), and a **LaTeX book** deliverable
  per language track (CC BY-SA 4.0, XeLaTeX/fontspec/polyglossia).
- Renamed Phase→Part, Week→Chapter in all user-facing prose (`roadmap.md`,
  READMEs); frontmatter fields (`phase`, `week`) unchanged internally.
- Added Part 0 — Sounds & Letters (`units/ES-P0-U00A/B/C`): the five vowel
  sounds, consonants that differ from English, stress & written-accent
  rules. Renumbered Chapter 1's session schedule (+3) to make room.
- Added a morphology unit (`ES-P0-M01`): the Latin *clamare* root ("to
  call/shout") — extends the *llamar* etymology from Unit 1 into
  *llamar/llamada/exclamar* (Spanish) and *claim/exclaim/acclaim/clamor/
  proclamation* (English).
- Retrofitted Grammar Lens sections into `U01` (reflexive-verb preview),
  `U02` (pronouns, pro-drop contrast), `U03` (linking-verb concept), `U05`
  (stative vs. dynamic), `U06` (full ser/estar contrastive payoff); added a
  grammatical-gender explanation to `U07` (days of the week, all masculine).
- Rewrote `roadmap.md` with Part/Chapter framing and a "Part 0" lead-in;
  elevated Chapter 3's description to gender-mastery framing.
- Rewrote `session-map.md` for the new 10-session Part 0 + Chapter 1
  schedule.
- Added `book/`: a LaTeX book (`book.tex`, `preamble.tex`,
  `chapters/part0-sounds-and-letters.tex`,
  `chapters/part1-ch01-greetings-and-pronouns.tex`), compiled and verified
  with XeLaTeX via `latexmk`. Title page, preface, and CC BY-SA 4.0 notice
  included. Grows one chapter at a time from here.

## Chapter 1 (originally "Week 1", Phase 0 — Foundations)

- Added `roadmap.md`: full year skeleton, Phases 0-4 plus buffer/assessment weeks.
- Added `session-map.md`: Week 1 session composition (sessions 1-7) and worked spaced-repetition schedule (N+1/N+3/N+7/N+15).
- Added Week 1 units (`units/ES-P0-U01` through `ES-P0-U08`): greetings, subject pronouns, *ser* (identity/origin), numbers 0-10, *estar* (state/location), *ser* vs *estar* contrast (practice-mix), days of the week, and an "introduce yourself" capstone (practice-mix) recombining the whole week.
- Added two worked `review` units (`ES-P0-R01`, `ES-P0-R02`) as concrete examples of the "fresh combination, not verbatim repeat" review pattern; later review instances follow the same pattern and are described algorithmically in `session-map.md` rather than each hand-authored.
- Etymology notes on every new vocabulary item, with a deliberate first look at Spanish's Arabic-derived vocabulary (*cero* ← Arabic *ṣifr*, also the source of English *cipher*) alongside the primary Latin chain.
