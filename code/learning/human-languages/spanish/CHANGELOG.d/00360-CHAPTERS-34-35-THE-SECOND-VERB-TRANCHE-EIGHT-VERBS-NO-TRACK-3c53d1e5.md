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

