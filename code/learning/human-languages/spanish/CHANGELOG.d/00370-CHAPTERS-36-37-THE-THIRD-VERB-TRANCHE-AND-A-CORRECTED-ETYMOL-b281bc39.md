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

