# Changelog

## Sub-five-minute remediation — 2026-08-02

- Corrected twenty-two declared five- or six-minute estimates whose lesson
  bodies already compute below 300 seconds.
- Replaced three computed violations with three prerequisite-ordered support
  lessons for explicit register/liaison, *être* suppletion, and pronominal past
  agreement.
- Preserved the vocabulary, grammar, etymology, exceptions, and cross-language
  depth. The shared report now measures zero French duration violations.
- `FR-C03-practice` at 293 computed seconds and `FR-C15-passe-simple` at 291 are
  the tightest remaining French lessons and should be watched during copy edits.

## The book catches up -- Chapters 3-16 typeset

The lessons had run ahead of the published artifact: 61 authored lessons through
Chapter 16, but the LaTeX book still stopped at Chapter 2 ("Introducing
Yourself"). Because the CI book build only compiles what is wired into
`book.tex`, the missing chapters were invisible to CI and the gap drifted
silently. This closes it -- **fourteen new book chapters**, written from the
existing `FR-C03`-`FR-C16` lessons and wired into `book.tex`:

- **Ch3** How Are You (merci, de rien, aller, comment ca va, comme ci comme ca)
- **Ch4** Farewells (au revoir, a plus tard, a bientot, a demain)
- **Ch5** The First Verbs (parler, habiter, travailler, je parle francais)
- **Ch6** Numbers One to Ten * **Ch7** The Days of the Week (the planet-gods)
- **Ch8** Telling the Time * **Ch9** Months and Seasons
- **Ch10** Family (parents, freres/soeurs -- with the Grimm's-law table)
- **Ch11** Bread, Water, Wine * **Ch12** Numbers Eleven to Twenty
- **Ch13** Colours * **Ch14** To Have, and How Old You Are (avoir, age)
- **Ch15** The Compound Past (passe compose, passe simple)
- **Ch16** To Be, and the Past That Takes It (etre, and the verbs that take it)

Each chapter follows the established book conventions: one `\section` per lesson
with a slug `\label`, the `cousinweb` / `culture` / `grammarlens` / `sounds`
boxes (the only four this book's preamble defines), `booktabs` tables, and every
atom traced to its root. Content is faithful to the lessons -- no new etymologies
introduced. Practice-section labels are chapter-qualified (`lesson:chN-practice`)
so they stay unique.

The book grows to **79 pages**; compiles clean with XeLaTeX (0 errors, 0 missing
characters, 0 undefined references, 0 duplicate labels) and was rasterized and
visually QA'd -- the PIE forms (*ph2ter, *bhreh2ter), the `oe` ligature in
*soeur*, and the nested Grimm's-law table all render correctly.

Also fixed: `FR-C07-jours-1.md` called Tiw "the Norse war-god". Tiw is the Old
English form (the Norse cognate is Tyr), so it now reads "the Germanic war-god"
-- matching how the German track's parallel lesson already phrased it.

## Chapter 17 — The body: a head that was a pot, and a hand inside English

- **Chapter 17 authored** (`FR-C17-tete`, `-main`) — the **body**, which is the
  theme all four parallel-track roadmaps name next after family and food.
- **la tête** (`FR-C17-tete`): the headline is that **French threw away the Latin
  word for "head."** *Tête* is not from *caput* but from ***testa***, an
  **earthenware pot** — Roman soldiers' slang for the skull, the way English says
  *noggin*. The joke replaced the real word, so *j'ai mal à la tête* is
  historically "**my pot hurts**."
  - *Caput* is shown surviving where it did: French **chef** ("chief" = head) and
    **chapitre**, and abroad in *cabeza*, *capo*, *captain*, *decapitate*.
  - The **circumflex is a receipt** — *testa* → Old French *teste* → *tête* — and
    the payoff is that English, which borrowed *before* the *s* fell, still has
    **test**, originally the shallow pot an alchemist assayed metals in.
- **la main** (`FR-C17-main`): flagged as **feminine despite a consonant
  ending**, because French gender is not predictable from the ending and this
  course always supplies the article. From ***manus***, presented as the most
  productive hand in English — *manual*, *manuscript*, *maintain*, *manage*,
  *manoeuvre* — with two things done deliberately:
  - **manufacture** ("made by hand") is called out as now naming precisely the
    thing that isn't.
  - **maintenir** = Latin *manū tenēre*, "hold in the hand" — so *maintenance* is
    literally keeping something in hand. (A draft cited "Ch. 14's *tenir*"; the
    French track has **no *tenir* lesson** — Ch. 14 is *avoir* and *âge*. Caught
    by grepping. *Ter* ← *tenēre* is the **Portuguese** Ch. 14.)

## Chapter 16 — *être*, and the half of the past Chapter 15 couldn't reach

- **Chapter 16 authored** (`FR-C16-etre`, `-etre-roots`,
  `-passe-compose-etre`, `-pronominal-past`). Chapter 15
  could only teach the *avoir* half of the compound past, because ***être* was
  taught in no lesson of any track**. This chapter supplies it and closes the
  other half.
- **être / its roots** (`FR-C16-etre`, `-etre-roots`): the six present forms, presented honestly as
  unpatternable, and then explained. *être* is **suppletive across three stems**:
  - *es-* — *suis/es/est/sommes/êtes/sont* **and the infinitive** *être*
    (← \**essere*), all from Latin ***esse***
  - *fu-* — the passé simple *je fus, il fut* (← the old perfect ***fuī***, PIE
    \**bʰuH-*), which ties straight back to Ch. 15's tense
  - ***ét-*** — **every form beginning *ét-***, from ***stāre***, "to stand":
    the participle *été*, the present participle *étant* (← *stantem*), and the
    whole imperfect *étais/était* (← *stābam*). Stated as a **limb**, not a stray
    form, because that is what it is — and it makes the lesson's own thesis
    stronger, not weaker.
  - The payoff is comparative: *stāre* is exactly the verb **Spanish** kept as
    ***estar*** (**ES-C04**, contrasted with *ser* in ES-C09). Spanish keeps the
    two **apart** and makes you choose; French kept *esse* and **swallowed a
    large piece** of the other. Noted that *stāre* also left French words outside
    *être* (*rester* ← *re-stāre*, *coûter* ← *constāre*) — what it didn't do is
    survive as a **separate** "to be" the way *estar* did. Anchored to English
    *go/went* so "suppletion" names something the learner already does.
- **passé composé with être / pronominal agreement**
  (`FR-C16-passe-compose-etre`, `-pronominal-past`): verbs of **motion
  and change of state** take *être* — taught as a **shape** (going, coming, being
  born, dying) rather than a list to memorise — built on *aller* from Ch. 3,
  **plus all pronominal verbs**. Then the visible part: the participle **agrees
  with the subject** (*elle est allé**e***, *elles sont allé**es***).
  - Two warnings included so the rule doesn't mislead: plain motion verbs
    (*marcher, courir, nager, danser, voyager*) take *avoir*, and
    *monter/descendre/sortir/rentrer/passer* **switch to *avoir* when
    transitive** (*j'ai monté les valises*).
  - **A dedicated "pronominal exception" section**, because the two additions
    above would otherwise contradict each other: pronominal verbs take *être* but
    their agreement follows the ***avoir*** rule — a **preceding direct object**,
    which is usually the reflexive pronoun and sometimes isn't. *Elle s'est
    lavé**e*** (reflexive = direct object) vs *elle s'est lav**é** les mains*
    (object follows) vs *elles se sont parl**é*** (reflexive is indirect). This
    lands as a *third* sighting of Ch. 15's rule rather than a new one — the
    auxiliary looks like *être*, the agreement behaves like *avoir*. And the one
    group that escapes even that: **essentially pronominal** verbs (*se
    souvenir*, *s'enfuir*, *s'évanouir*), which have no non-reflexive form for
    the pronoun to be an object *of*, so they agree with the subject after all
    (*elles se sont souvenu**es***).
  - The chapter's real argument is that this is **not a second rule**. Ch. 15
    established that *j'ai parlé* was once "I have [a thing] spoken", with the
    participle an **adjective** agreeing with the object — which is why it still
    agrees with a *preceding* object. *Elle est allée* was likewise once "**she
    is** gone", *gone* describing **her**. One idea underneath both: **the
    participle was an adjective and agrees with whatever it described.** The two
    auxiliaries differ only in what that was.
- Prerequisites and `reviews_of` verified against existing ids (Ch. 3 *aller*,
  Ch. 14 *avoir*, Ch. 15 both lessons).

## Chapter 15 — The compound past, and the tense it drove out

- **Chapter 15 authored** (`FR-C15-passe-compose`, `-passe-simple`): the everyday
  past, built on Ch.14's *avoir* — reviewing Ch.5/14 via `reviews_of`.
- **passé composé** (`FR-C15-passe-compose`): *avoir* + past participle (*-er*→*-é*
  ← *-ātum*, *-ir*→*-i* ← *-ītum*, *-re*→*-u* ← *-ūtum*), noting that *parler*,
  *parlé* and *parlez* are **homophones** — three spellings, one sound. The
  etymology carries the lesson: *j'ai parlé* was once literally "**I have [a thing]
  spoken**," from Latin *habeō litterās scriptās* ("I have letters written") — a
  **possessive** in which the participle was an **adjective** agreeing with the
  object. Over centuries "I possess a written thing" slid into "I wrote," so a
  possessive construction **hardened into a tense** — and the fossil is still
  working: when the object comes first, the participle **still agrees** (*les
  lettres que j'ai écrit**es***), a two-thousand-year-old adjective ending doing its
  old job.
- **passé simple** (`FR-C15-passe-simple`): *il parla* ← Vulgar Latin **\*parabolāvit**, the
  direct inheritance — framed as recognise-don't-produce, since it fills the past
  tense of essentially all French literature and appears in no conversation. Its
  value here is comparative: it is the **same tense** as Spanish *habló*, Portuguese
  *falou* and Italian *parlò*, and the chapter closes on the cross-language
  observation that **French, German and Italian all** built a "have"
  compound and let it push the inherited simple past out of speech, while
  **Spanish and Portuguese, at the western edge, kept theirs**.
- Taxonomy: namespaced `FR-PAST-COMPOUND`, `FR-PAST-SIMPLE-LITERARY`.

## Chapter 14 — avoir, and having your years

- **Chapter 14 authored** (`FR-C14-avoir`, `-age`): the verb the rest of the
  course is built on, reviewing Ch.5/9/12/13 via `reviews_of`.
- **avoir** (`FR-C14-avoir`): *j'ai/tu as/il a/nous avons/vous avez/ils ont* —
  with the observation that the three singular forms are **homophones** (*ai · as
  · a*), so only the pronoun tells you who. Etymology: ← *habēre*, and the payoff
  is that Chapter 5 already taught this root — **habiter** ← *habitāre* is
  *habēre*'s frequentative, "to keep having a place," so *avoir* and *habiter* are
  the same word twice. English took the family whole: *habit* (what you have
  regularly), *inhabit*, *exhibit* ("hold out"), *prohibit* ("hold back"). Plus how
  far it wore down — Latin *habeō* → **j'ai**, a single vowel sound, French's usual
  erosion (cf. *aqua* → *eau*).
- **j'ai vingt ans** (`FR-C14-age`): age takes **avoir**, never *être*, and *ans*
  is **obligatory** where English drops "years old." *An* ← *annus* →
  *annual/anniversary/annals*. Includes **liaison** — the silent *t* of *vingt*
  wakes up before a vowel: *vin-t-an*. Closes on the five-language table:
  **French, Spanish, Italian and Portuguese all *have* their years; German and
  English *are* theirs** — age as possession vs age as identity.
- Sets up the compound past: *avoir* is the auxiliary the *passé composé* needs.
- Taxonomy: namespaced `FR-VERB-HAVE`, `FR-AGE`.

## Chapter 13 — Colours

- **Chapter 13 authored** (`FR-C13-noir-blanc`, `-rouge-bleu`): the **borrowing**
  chapter, reviewing Ch.11/12 via `reviews_of`.
- **noir & blanc** (`FR-C13-noir-blanc`): *noir* ← Latin *niger* (→ *denigrate*) is
  the expected inheritance — but **blanc is not from Latin *albus*** at all. It is
  **Frankish *blank*** ("shining, gleaming"), a **Germanic** word borrowed **into**
  French: the reverse of the usual Latin→Germanic flow, and it displaced *albus*
  entirely, probably by being the more **vivid** option. *Albus* didn't die, it just
  stopped being a colour: **aube** ("dawn"), *aubépine* ("white thorn"), *album*,
  *albinos*.
- **rouge & bleu** (`FR-C13-rouge-bleu`): *rouge* ← *rubeus* ← PIE ***h₁rewdʰ-***,
  making *rouge*, English *red/rust/ruby* and German *rot* **cousins by descent**,
  not borrowings — one of the oldest reconstructible colour words. *Bleu* is a
  **second** Germanic loan (*blāo*), and English then borrowed **back** from French,
  so *blue* is a Germanic word that came home in disguise; *azur* (← Arabic
  *lāzaward*) noted alongside. Payoff: of **bleu-blanc-rouge**, **two of three are
  loanwords**.
- Taxonomy: namespaced `FR-COLOUR-BLACK-WHITE`, `FR-COLOUR-RED-BLUE`.

## Chapter 12 — Numbers 11–20

- **Chapter 12 authored** (`FR-C12-nombres-11-16`, `-17-20`): the teens, atom-first,
  reviewing Ch.6/Ch.11 via `reviews_of`.
- **onze–seize** — the six numbers French inherited **already fused** from Latin
  (*ūndecim, duodecim … sēdecim*). The shared **-ze** is *decem* ("ten") worn thin —
  the **same ten** the learner already knows in **dix** and **décembre** (Ch.9).
  Each word's front is its Chapter 6 digit (*deux→dou-*, *six→sei-*).
- **dix-sept–vingt** — the **seam**: at 17 French **abandons** the fusion and goes
  transparent, *dix-sept* = plainly "ten-seven" — and the **order flips** with it
  (digit-first *seize* → ten-first *dix-sept*). Notes that Latin itself wobbled here
  (*duodēvīgintī*, "two-from-twenty"), a subtraction all the sisters dropped.
  *vingt* ← *vīgintī* → English **vigesimal**, and the seed of *quatre-vingts*.
- Taxonomy: namespaced `FR-NUM-11-16`, `FR-NUM-17-20`.

## Chapter 11 — Food (bread, water, wine)

- **Chapter 11 authored** (`FR-C11-pain`, `-eau-vin`): the everyday table trio,
  atom-first, reviewing Ch.10/Ch.1 via `reviews_of`.
- **pain** ("bread") ← *pānis* — with the payoff that a **companion** is literally
  "one you **share bread** with" (*com-* + *pānis*); also *company*, *pantry*.
- **eau / vin** — **eau** ("water") is French's **most eroded** loan from Latin:
  *aqua → eau*, worn down to a bare vowel "oh" (three silent letters for one
  sound), while English kept the loud original in *aquatic/aquarium*. **vin** ←
  *vīnum* held its shape → *wine/vine/vinegar/vintage*.
- Taxonomy: namespaced `FR-FOOD-BREAD`, `FR-FOOD-DRINKS`.

## Chapter 10 — Family

- **Chapter 10 authored** (`FR-C10-parents`, `-freres-soeurs`): the immediate
  family, atom-first, reviewing Ch.9/Ch.1 via `reviews_of`.
- **père / mère** ← *pater / māter* ← PIE *\*ph₂tḗr / \*méh₂tēr*. Taught as the
  **same inherited words** as English *father / mother*, split only by **Grimm's
  law** (*p → f*, *t → th*) — French kept Latin's *p*, English shifted it. Root
  payoff: paternal/patron, maternal/matron.
- **frère / sœur** ← *frāter / soror* → fraternal/friar, sorority; the **œ**
  ligature is introduced as the fused vowel spelling worn-down *soror*.
- Taxonomy: namespaced `FR-FAMILY-PARENTS`, `FR-FAMILY-SIBLINGS`.

## Chapter 9 — Months & seasons

- **Chapter 9 authored** (`FR-C09-mois`, `-saisons`): the calendar year, atom-first,
  reviewing Ch.6–8 via `reviews_of`.
- **The months** are a parade of Roman gods and emperors: *janvier* ← Janus (the
  two-faced god of beginnings), *février* ← the *Februa* purification, *mai* ← Maia,
  *juin* ← Juno, with **two big payoffs** — *mars* is the **same Mars** behind
  *mardi* (Tuesday), and *septembre–décembre* still mean the Latin **7–10** learned
  in the numbers chapter (the Roman year began in March; *juillet/août* ← Julius/
  Augustus were inserted and shifted the count).
- **The seasons**: *printemps* = *prime* + *temps*, "the **first time / prime
  season**"; *été* ← *aestas* "heat"; *automne* ← *autumnus*; *hiver* ← *hibernum*
  "wintry" (cousin of English **hibernate**). Plus the *au printemps* / *en été…*
  preposition split.
- Taxonomy: namespaced `FR-MONTHS`, `FR-SEASONS`.

## Chapter 8 — Time & the clock

- **Chapter 8 authored** (`FR-C08-heure`, `-midi-minuit`): telling the time,
  atom-first, reviewing Ch.6–7 via `reviews_of`.
- **heure** ← Latin *hōra* ← Greek *hṓrā* ("a time of day," the *Horae* being the
  season-goddesses) → English *hour*, the same word spelt apart (both keep the
  silent Latin *h-*). Telling time: *il est une heure / deux heures* ("it is two
  hours"), with the liaison *deu-z-eur*.
- **midi / minuit** ← *medius diēs* "mid-day" / *media nox* "mid-night" — the two
  unnumbered hours, each *mi-* ("middle," ← *medius*) + *-di* (*diēs*, the day of
  *lundi*) / *-nuit* (*noctem*, cousin of English *night*). Aside: English *noon*
  is Latin *nōna hōra* "ninth hour," drifted from mid-afternoon to midday.
- Taxonomy: namespaced `FR-TIME-HOUR`, `FR-TIME-NOON-MIDNIGHT`.

## Chapter 7 — Days of the week

- **Chapter 7 authored** (`FR-C07-jours-1`, `-jours-2`): the seven days, atom-first,
  reviewing Ch.6 via `reviews_of`, with the **planet-god week** as the through-line.
- **jours-1** (lundi–vendredi): every weekday is *[planet-god]* + **-di** (← *diēs*
  "day") — *lundi* = *lūnae diēs* "Moon's day," etc. The centrepiece is the
  **Roman-planet ↔ Germanic-god bridge**: *mardi* and English *Tuesday* are the same
  day (the war-god's), named *Mars* in Latin but *Tiw* in Germanic; *jeudi*
  (Jupiter) = *Thursday* (Thor) — *interpretatio germanica*.
- **jours-2** (samedi, dimanche): where **religion overwrote astronomy** — *samedi*
  ← *Sabbatum* (the Hebrew Sabbath, so English *Saturday*/Saturn and French *samedi*
  are the same day, two names); *dimanche* ← *diēs Dominica* "the **Lord's** day"
  (*Dominus* → dominion/dame), the *di-* fossil moved to the front.
- Taxonomy: namespaced `FR-DAYS-WEEKDAYS`, `FR-DAYS-WEEKEND`.

## Chapter 6 — Numbers 1–10

- **Chapter 6 authored** (`FR-C06-nombres-1-5`, `-nombres-6-10`): counting to ten,
  atom-first, each ~4–5 min, reviewing Ch.5 via `reviews_of`; every number carries
  its Latin source, **Spanish twin** (French grounds in its Romance sibling), and
  English cousins.
- **1–5** (*un/deux/trois/quatre/cinq* ← *ūnus/duo/trēs/quattuor/quīnque*): *un/une*
  doubles as "a/an" (like Spanish *un/una*); *cinq* → English *cinque* (the 5 on
  dice).
- **6–10** (*six/sept/huit/neuf/dix*): the dramatic erosion of *octō → oit → huit*
  (the 8 survives only in the month *octobre*); *dix* → English **dime** (via Old
  French *disme* ← *decima*); and the **septembre–décembre = Latin 7–10** calendar
  trick (the Roman year began in March; *juillet/août* pushed the counting months
  down two).
- Taxonomy: namespaced `FR-NUM-1-5`, `FR-NUM-6-10`.

## Chapter 5 — The first verbs (sentences start to move)

- **Chapter 5 authored** (`FR-C05-parler`, `-habiter`, `-travailler`,
  `-je-parle-francais`, `-practice`): French's first **grammar-engine** chapter,
  mirroring the Spanish Ch.6 verbs chapter. The learner stops reciting phrases and
  starts **building sentences from a pattern**.
- **The regular -er present tense** — the biggest French verb family: drop *-er*,
  add *-e/-es/-e/-ons/-ez/-ent*. Taught on **parler** and cemented on **habiter**
  and **travailler**.
- **The silent-ending insight + the pro-drop contrast**: *-e/-es/-ent* are all
  **silent**, so *je parle / tu parles / ils parlent* sound identical (*parl*) —
  which is exactly **why French keeps its subject pronouns** where Spanish/Italian
  drop them (the ear can't hear the person, so the pronoun must). Stated as the
  single biggest structural difference from the Iberian cousins.
- **Etymology**: *parler* ← *parabolāre* "tell parables" (→ parable/parole/
  palaver/parley); *habiter* ← *habitāre* "keep having a place" (→ habitat/
  inhabit); *travailler* ← *tripalium* "torture" (→ **travail/travel**; twin of
  Spanish *trabajar*); *français* ← *Francia* (the Franks, whose name meant
  "free" → English *frank*). First self-assembled sentence: **Je parle français**.
- Taxonomy: namespaced `FR-VERB-PARLER/HABITER/TRAVAILLER`, `FR-WORD-FRANCAIS`
  documented.

## Writing nuances — the accents, the cédille, the tréma

- **First French `writing`-type lessons** (`FR-W01-accents`, `FR-W02-cedille`,
  `FR-W03-trema`): orthography taught etymology-first, the same way as the
  Spanish writing lessons, once enough accented words have accumulated.
- **The three accents on *e*** (`é è ê`): *é* aigu = "ay", *è* grave = open "eh"
  (and the grave that only separates look-alikes, *a/à*, *ou/où*), and the star —
  the **circonflexe ê as a tombstone for a lost *s***, with the English cousin
  usually keeping it (*forêt*→forest, *hôpital*→hospital, *île*→isle,
  *bête*→beast, *être*→*stāre*). The single most useful French reading trick.
- **The cédille ç**: keeps *c* soft (*s*) before *a/o/u* (*français*, *garçon*),
  and the hook's origin as a shrunken subscript *z* (Spanish *zedilla*, "little z").
- **The tréma ï/ë**: "pronounce these vowels **separately**" (*naïve*, *Noël*,
  *maïs* vs *mais*) — explicitly contrasted with the German umlaut (which *changes*
  a vowel rather than *splitting* two).
- Uses the `writing` lesson type (no `concept_tag`) — no taxonomy change.

## Chapter 4 — Farewells (parallel of Spanish Ch. 5)

- **Chapter 4 authored** (`FR-C04-au-revoir`, `-a-plus-tard`, `-a-bientot`,
  `-a-demain`, `-practice`): closing a conversation, atom-first, reviewing
  Chapter 3. Reuses the canonical `FAREWELL` + `FAREWELL-LATER/TOMORROW/SOON`
  concepts introduced with Spanish Ch. 5, mapping each French goodbye to its
  Spanish twin.
- **The "see you again" metaphor**: *au revoir* = "until the re-seeing" (*voir* ←
  *vidēre* → vision/video/revise) — explicitly paired with German *auf
  Wiedersehen* ("on the seeing-again"), against Spanish *adiós* ("to God").
- **Cross-language root callbacks**: *à plus tard* — *tard* ← Latin *tarde*, the
  same word as Spanish *tarde*; *à demain* — *demain* ← *dē māne* "from the
  morning", sharing *māne* with Spanish *mañana* (and English *matinée*).
- **A writing-nuance aside**: the circumflex on *bientôt* (← *tost*) as the ghost
  of a dropped *s* (*hôtel* ← *hostel*), tying back to the accent-mark thread.
- All soft goodbyes are **à** + a time, mirroring Spanish's **hasta**.

## Chapter 3 — "Comment ça va ?" (the parallel of Spanish Ch. 4)

- **Chapter 3 authored** (`FR-C03-merci`, `-de-rien`, `-aller`,
  `-comment-ca-va`, `-comment-registers`, `-comme-ci-comme-ca`, `-practice`):
  the "how are you?"
  exchange, atom-first, reviewing Chapter 2 throughout. Built deliberately as the
  cross-language mirror of the Spanish Chapter 4 shipped in the same PR — same
  canonical concepts (`STATE-HOW-ARE-YOU`, `COURTESY-YOUREWELCOME`, `WORD-SOSO`),
  so the interleaving method has real parallel material.
- **Etymology contrasts made explicit** (the point of the curriculum):
  - *merci* ← *mercēs* "reward / wages" (→ mercy/merchant/commerce) — set against
    Spanish *gracias* ← *grātia* "grace" and Portuguese *obrigado* ← "obliged".
  - *de rien* ← *rem* "a thing" → "nothing" — the exact twin of Spanish *de nada*
    ← *nāta* "a born thing" (a callback the Spanish lesson already forward-references).
  - *aller* "to go" as the state-verb ("how does it *go*?") — contrasted with
    Spanish *estar* "to stand"; its suppletive paradigm traced to *ambulāre*
    (amble/ambulance), *vādere* (invade/evade), *īre* (exit/transit).
  - *comme ci, comme ça* — *comme* shares *quōmodo* with *comment*; the shrug set
    against Spanish *más o menos* and Italian *così così*.
- Taxonomy: namespaced `FR-VERB-ALLER` documented in the examples list.

## Chapter 2 — Introducing Yourself

- New chapter built around the introduction dialogue (*Je m'appelle Susanne. /
  Comment vous appelez-vous? / Je m'appelle David. / Enchanté.*), atom-first,
  one word per lesson (`lessons/FR-C02-*`, `book/chapters/ch02-introductions.tex`):
  - **je** ("I" ← *ego*; English *ego*)
  - **me** ("myself" ← Latin *mē*; English *me*, *my*, *mine*) — its own lesson,
    with the reflexive set *me / te / se* traced. (Every atom of *je m'appelle*
    is taught and rooted, not just glossed.)
  - **(s')appeler** ("to call [oneself]" ← *appellāre*; *appeal*, *appellation*)
    — introduces **reflexive verbs**.
  - **je m'appelle…** — assembled: **"my name is…"** ("I call myself"), with the
    literal *mon nom est* (← *nōmen*, English *noun*) as the stiffer alternative.
  - **tu / vous** (familiar / formal "you" ← *tū / vōs*) — politeness by using
    the plural on one person; contrasted with Spanish *usted*.
  - **comment** ("how" ← *quo modo*; same source as Spanish *cómo*).
  - **comment vous appelez-vous?** — **"what's your name?"** by inversion; the
    informal *comment tu t'appelles?*.
  - **enchanté(e)** ("pleased to meet you" ← *in-cantāre*; *enchant*,
    *incantation*, *chant*) — gender agreement with the speaker.
  - **practice** — the whole dialogue.
- Also fixed two leftover beginner-audience slips the earlier pass missed
  (`roadmap.md` "the learner's in-progress language"; `session-map.md` "the
  Spanish twin"). Book compiles clean with XeLaTeX.

## Beginner-audience pass — Spanish no longer assumed as prior knowledge

Corrected a systemic violation of HL00's Audience rule: the book and practice
lessons addressed a reader who was "also learning Spanish" and leaned on
Spanish as knowledge already owned. The books are for a true beginner whose
only shared language is English; Spanish comparisons are enrichment the text
must supply in full, not a baseline it may assume.

- Preface rewritten: drops "Because the reader is also learning Spanish…" and
  "exactly as in the Spanish book"; states the true-beginner framing and that
  every Spanish comparison is supplied by the text (a reader who knows Spanish
  "simply nods along").
- Chapter 1 (`book/chapters/ch01-greetings.tex`) and the matching practice
  lessons: recast every "Spanish twin," "the *bueno/buena* machine from
  Spanish," "One mercy over Spanish," and "you know this from Spanish" into
  self-contained "Spanish, another daughter of Latin, does X" enrichment.
  Section title "*bien* — and a Spanish twin" → "*bien* — 'well'."
- Filled the two missing noun plurals the standard wants: *les soirs*,
  *les nuits* (a new Grammar Lens on *soir*, extended on *nuit*).
- Book still compiles clean with XeLaTeX (13 pages).

## Chapter 1 — Greetings (track bootstrapped)

- New French track, built on the same HL00 framework as Spanish: one word per
  lesson, slug ids, gender-before-nouns, atom-first assembly, derivations
  shown (not just roots named), LaTeX book.
- Chapter 1 (`lessons/FR-C01-*`), atom-first:
  - **salut** (informal hi ← Latin *salus* "health") · **bien** ("well" ←
    *bene*; the Spanish twin) · **bon / bonne** ("good" ← *bonus*; agreement)
  - **le / la / les** ("the"; grammatical gender ← Latin *ille/illa/illos*,
    same as Spanish *el/la*, also the source of *il/elle*)
  - **jour** ("day" ← *diurnum* ← *dies*; the detour that gives English
    *journal*/*journey* and explains why French *jour* ≠ Spanish *día*)
  - **bonjour** (assembled; *singular*, contrasted with plural *buenos días*)
  - **soir** ("evening" ← *sērus* "late"; parallels Spanish *tarde* ←
    *tardus*) · **bonsoir**
  - **nuit** ("night" ← *noctem*; the *-ct-→-ch-* (Spanish) vs *-ct-→-it-*
    (French) sound-change table) · **bonne nuit** (feminine agreement)
  - **practice**
- Grounds each word against English **and Spanish** (the learner's in-progress
  language), foregrounding the Romance twins' differences.
- Book compiles clean with XeLaTeX (13 pages); the CI workflow auto-discovers
  `french/book/` and builds it as a PDF artifact.
