# Changelog

## Warning-free 104-page book — 2026-08-03

- Taught the shared inline renderer to preserve backslash-escaped Markdown
  punctuation, repairing the reconstructed `*parabolāvit` form in Chapter 15,
  and added regression coverage for the exact bold form.
- Removed paragraph indentation before generated tables, added a bookmark-safe
  Chapter 1 title, and made intentionally short lesson pages ragged-bottom.
- Tightened only the canonical prose and table cells responsible for horizontal
  layout warnings, then recalculated their sub-five-minute duration budgets.
- Forced a clean XeLaTeX build with zero missing glyphs, overfull or underfull
  boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. All 104
  rendered pages and the complete Preface/pronunciation/Chapter 1–17 outline
  were inspected successfully.

## Canonical book Chapters 2–17 — 2026-08-03

- Migrated all 49 lessons in Chapters 2–17 to the strict schema-v2 shared-spine
  contract with prerequisite-closed knowledge boundaries and sub-five-minute
  duration budgets.
- Added sixteen deterministic chapter targets and source hashes so Language
  Ladder and the downloadable book consume and verify the same canonical lesson
  AST instead of maintaining separate copies.
- Expanded the book from 13 to 104 pages, added width-aware generated tables,
  and taught the renderer portable TeX fallbacks for scholarly Unicode symbols.
- Verified zero missing glyphs and duplicate destinations, inspected all 104
  rendered pages, and retained the remaining layout/bookmark cleanup as HL-B15.

## Sub-five-minute remediation — 2026-08-02

- Corrected seventeen declared five-minute estimates whose lesson bodies
  already compute below 300 seconds.
- Replaced the three computed violations with four prerequisite-ordered
  micro-lessons: informal → formal → register-neutral wellbeing questions, then
  *essere* forms → borrowed *stato* → *andare* → participle agreement.
- Preserved the original etymology and cross-language comparisons while making
  each register, metaphor, suppletive stem, and grammar rule independently
  learnable in under five minutes. The shared report now measures zero Italian
  duration violations.
- `IT-C02-practice` at 297 computed seconds and `IT-C17-mano` at 298 are the
  tightest remaining Italian lessons and should be watched during copy edits.

## Chapter 17 — The body: the pot kept whole, and a noun that breaks the rule

- **Chapter 17 authored** (`IT-C17-testa`, `-mano`) — the **body**, the theme the
  parallel-track roadmaps name next.
- **la testa** (`IT-C17-testa`): the same pot-word as French's *tête*, and the
  point is **how little Italian did to it**. Latin ***testa*** → Italian
  ***testa***, almost unchanged (the spelling is identical; the vowel opened); French wore the identical word down to *tête*. That is
  the **conservative-sister** pattern this track keeps meeting — *acqua* against
  *eau* in Ch. 11, *parlato* keeping the Latin *-t-* that *parlé* wore away in
  Ch. 15.
  - And where French gave *caput* away almost entirely, **Italian kept it as a
    live everyday word**: ***il capo***, boss/chief — which English borrowed
    straight out of Italian — plus *capitale*, *capitolo*, *capitano*. So Italian
    runs both the old head-word and its slang replacement in parallel.
- **la mano** (`IT-C17-mano`): chosen because it **breaks the *-o*/*-a*
  tendency** — feminine, ending in *-o*, plural *le mani*. (A draft credited that
  rule to Chapter 1; `IT-C01-il-la-lo` teaches the opposite — "learn each noun
  **with** its article" — so the lesson now states the tendency itself and cites
  Ch. 1 as the reason *not* to trust endings.) The explanation is the
  lesson: Latin *manus* was **fourth declension**, a class whose nouns ended
  *-us* but could be feminine. Italian flattened five Latin declensions to
  **three** productive classes (*-o*/*-i*, *-a*/*-e*, and *-e*/*-i* as in
  *notte/notti*, which Ch. 1 already gave the learner),
  and left this word holding an old gender with an ending that now looks
  misfiled. **The word didn't change; the system around it did.**
  - Stated as a general principle, because it recurs everywhere in this
    curriculum: **an "irregular" word is usually a regular word from a system
    that no longer exists.**
  - English took **manage** specifically from Italian *maneggiare*, "to handle a
    **horse**" — so every manager is etymologically working in a riding school.

## Chapter 16 — *essere*, and the participle it borrowed from *stare*

- **Chapter 16 authored** (`IT-C16-essere`, `-essere-stato`, `-andare`,
  `-passato-prossimo-essere`).
  Ch. 15 taught only the *avere* half of the passato prossimo; *essere* existed
  in no lesson. This supplies it — and Italian turns out to have the most
  interesting version of the story, because **Ch. 2 already taught *stare***.
- **essere** (`IT-C16-essere`): the six forms, the `è`/`e` accent contrast
  (*Marco **è** italiano* vs *Marco **e** Anna* — one accent, two different
  words), and the *io sono* / *loro sono* collision that is one of the few places
  a pro-drop language has to keep its pronoun.
- **borrowed *stato*** (`IT-C16-essere-stato`) holds the chapter's centrepiece:
  Latin's *esse* and *stāre* both survived into Italian as separate living verbs
  — but *essere*'s own participle did not, so Italian filled the gap with
  ***stare*'s**. Both verbs' participle is **stato**, and ***sono stato***
  therefore means both "I have **been**" and "I have **stayed**", separable only
  by context.
- Set against the sisters in one table: **Spanish** split the pair fully (*ser*
  / *estar*), **French** kept *esse* as *être* and absorbed *stāre*'s whole
  *ét-* limb (*été*, *étant*, *étais*; *stāre* survives elsewhere in French too
  — *rester*, *coûter* — just not as a separate "to be"), **Italian** kept both
  but let them **overlap**. Italian sits exactly between the other two.
- **andare** (`IT-C16-andare`) is introduced explicitly as a new verb
  (`vado/vai/va/andiamo/andate/vanno`) rather than smuggled into the past. It is
  itself suppletive, shown as a
  stem table rather than prose so no form is left unaccounted for: **vad-**
  (*vado, vai, va, **vanno*** ← *vādere* "to stride") against **and-**
  (*andiamo, andate, andare, andato*, origin genuinely disputed, most likely
  *ambitāre*) — **four** present forms from the first stem, **two** from the
  second, plus the infinitive and participle. The lesson flags that ***vanno*
  files with *vado***, so the split is *not* singular-versus-plural — the same two-stem trick as *essere*, and the
  one behind Spanish *voy* vs *andar*.
- **passato prossimo with essere** (`IT-C16-passato-prossimo-essere`): after the
  dedicated atoms, opens with **`sono stato`** so the first *essere*-past costs
  **no new vocabulary**.
  - The **subject agreement** in all four endings (*andato / -a / -i / -e*), with
    the note that a woman says *sono andat**a***; explained via the same
    adjective fossil as French (*Anna è andata* ← "Anna **is** gone-away",
    describing her, like *Anna è stanca*).
  - Closes the three-language table: **French and Italian keep** participle
    agreement, **German drops** it — with the lesson body corrected to say the
    three systems were arrived at **in parallel** rather than "inherited", since
    German's is a native Germanic development that grew up alongside the Romance
    ones through contact. (The same correction was applied to the German track;
    this lesson's table is the other place the claim appears.)

## Chapter 15 — The compound past, and the one geography decides

- **Chapter 15 authored** (`IT-C15-passato-prossimo`, `-passato-remoto`): the
  everyday past, built on Ch.14's *avere* — reviewing Ch.5/14 via `reviews_of`.
- **passato prossimo** (`IT-C15-passato-prossimo`): *avere* + past participle
  (*-are*→*-ato*, *-ere*→*-uto*, *-ire*→*-ito*), with two callbacks. The silent
  **h** from Ch.14 is doing its job again — *ho parlato* can't be misread as *o
  parlato* ("or spoken"). And Italian **keeps the Latin -t-** (*parlato*) that
  French wore away (*parlé*), both from *-ātum*: the conservative sister again,
  matching Ch.11's *acqua* against *eau*. Same buried possessive as French — Latin
  *habeō litterās scriptās*, where the participle was an **adjective** agreeing with
  the object, a construction that **hardened into a tense**; the agreement survives
  when the object precedes (*le ho vist**e***).
- **passato remoto** (`IT-C15-passato-remoto`): *parlò* ← Vulgar Latin **\*parabolāvit**, with
  the final written stress the numbers chapter already introduced. Italian's
  distinctive fact is that this tense's survival is **geographic, not stylistic** —
  everyday speech in **Sicily and much of the south**, both-with-a-distinction in
  Tuscany, and literary in the **north**. So the "correct" past tense changes as you
  travel, and Italian is the language caught **mid-process**: French lost the
  inherited past from speech entirely, Spanish and Portuguese never gave it up, and
  Italian still holds it in half the country.
- Taxonomy: namespaced `IT-PAST-COMPOUND`, `IT-PAST-SIMPLE-REGIONAL`.

## Chapter 14 — avere, and having your years

- **Chapter 14 authored** (`IT-C14-avere`, `-eta`): the workhorse verb and the
  language's only silent letter, reviewing Ch.5/9/10/11/12/13 via `reviews_of`.
- **avere** (`IT-C14-avere`): *ho/hai/ha/abbiamo/avete/hanno* ← *habēre*, the same
  source as French *avoir*, and English's *habit/inhabit/exhibit/prohibit*. The
  chapter's real subject is the **silent h**: Italian discarded the Latin *h*
  almost everywhere (*homō* → *uomo*, *herba* → *erba*), but kept it in exactly
  these four forms because without it they collide with **o** ("or"), **ai** ("to
  the"), **a** ("to") and **anno** ("**year**"). The letter is never pronounced and
  survives **only so the eye can tell the words apart** — spelling doing a job
  sound cannot. Also notes *abbiamo*'s **bb** as the old *habē-* resurfacing while
  *ho* wore down to one vowel.
- **ho venti anni** (`IT-C14-eta`): age via *avere*, never *essere*; *anno* ←
  *annus* → *annual/anniversary*, with a genuinely held **double n**. The silent
  *h* then pays off inside this very chapter — ***hanno*** ("they have") and
  ***anno*** ("year") are **homophones** that co-occur in age sentences, which is
  exactly why the letter was worth keeping. Closes on the five-language table:
  **Romance has its years; Germanic is its years.**
- Sets up the *passato prossimo*, which is built on *avere*.
- Taxonomy: namespaced `IT-VERB-HAVE`, `IT-AGE`.

## Chapter 13 — Colours

- **Chapter 13 authored** (`IT-C13-nero-bianco`, `-rosso-blu`): two colours from two
  different peoples, reviewing Ch.11/12 via `reviews_of`.
- **nero & bianco** (`IT-C13-nero-bianco`): *nero* ← Latin *niger* is Rome's own word,
  barely changed. **Bianco** is not: it comes from Germanic ***blank*** ("shining"),
  most likely carried in by the **Lombards**, the Germanic people who ruled the north
  for two centuries and left their name on **Lombardia**. The loan won so completely
  that Latin *albus* was pushed out of the colour slot, surviving as **alba**
  ("dawn"), **albume** ("egg white"), and in place names.
- **rosso & blu** (`IT-C13-rosso-blu`): *rosso* ← *russus* ← PIE ***h₁rewdʰ-***,
  a **cousin** of *red/rot/rouge* rather than a borrowing (with a note on holding the
  **double s** — *roso* ≠ *rosso*). *Blu* ← Germanic *blāo* confirms the chapter
  pattern: Italian's **white and blue are both Germanic imports**. Then **azzurro** ←
  Arabic ***lāzaward*** ("lapis lazuli") ← Persian *lāžward*, the initial *l-*
  swallowed as if it were an article — the same journey that gave Spanish/Portuguese
  *azul*, French *azur*, English **azure**. Payoff: **gli Azzurri** are named, at the
  end of a long chain, after a blue stone mined in Afghanistan.
- Taxonomy: namespaced `IT-COLOUR-BLACK-WHITE`, `IT-COLOUR-RED-BLUE`.

## Chapter 12 — Numbers 11–20

- **Chapter 12 authored** (`IT-C12-numeri-11-16`, `-17-20`): the teens, atom-first,
  reviewing Ch.6/Ch.11 via `reviews_of`.
- **undici–sedici** — Italian keeps the Latin fusions **most legibly**: the shared
  **-dici** is still visibly *decem* ("ten"), the very word the learner says as
  **dieci**. Set against the sisters, the clarity is the point — Latin *sēdecim* →
  Italian **sedici** (ten audible) vs French **seize** (worn to *-ze*) vs Portuguese
  **dezesseis** (rebuilt entirely).
- **diciassette–venti** — the **reversal**: at 17 Italian turns the count around,
  *se-dici* ("six-ten") becoming *dici-assette* ("**ten**-and-seven"), the ten
  jumping to the front; the linking sounds (*diciAssette*, *diciANnove*) are just
  Italian smoothing the joint. *venti* ← *vīgintī*.
- Includes the three-sister table of **where each breaks** — Portuguese 16, French
  and Italian 17 — one inherited Latin system, three different seams.
- Taxonomy: namespaced `IT-NUM-11-16`, `IT-NUM-17-20`.

## Chapter 11 — Food (bread, water, wine)

- **Chapter 11 authored** (`IT-C11-pane`, `-acqua-vino`): the everyday table trio,
  atom-first, reviewing Ch.10/Ch.1 via `reviews_of`.
- **pane** ("bread") — **closest to Latin** *pānis*; the **companion** payoff
  (*com-* + *pānis*, "one you share bread with"), plus the purely Italian
  **companatico** — "whatever you eat **with** bread."
- **acqua / vino** — **acqua** ("water") **kept** Latin *aqua* almost whole (even
  doubling *-cq-*), a sharp contrast with French *eau* worn to a single vowel;
  **vino** ← *vīnum* → *wine/vine/vinegar*.
- Taxonomy: namespaced `IT-FOOD-BREAD`, `IT-FOOD-DRINKS`.

## Chapter 10 — Family

- **Chapter 10 authored** (`IT-C10-genitori`, `-fratello-sorella`): the immediate
  family, atom-first, reviewing Ch.9/Ch.1 via `reviews_of`.
- **padre / madre** — the sisters' **closest to Latin** *pater / māter* (only the
  *-t-* softened to *-d-*); *padre* is the very word English borrowed for a priest.
  **i genitori** ("parents") ← *genitor* "**begetter**" (*gignere* "beget") →
  genesis/gene/progenitor; with the **false-friend** warning that *parenti* means
  **relatives**, not "parents."
- **fratello / sorella** — Italian rebuilt "brother/sister" with its **diminutive**
  *-ello / -ella* ("little brother/sister"), keeping the *frat- / soror-* roots
  (→ fraternal, sorority).
- Taxonomy: namespaced `IT-FAMILY-PARENTS`, `IT-FAMILY-SIBLINGS`.

## Chapter 9 — Months & seasons

- **Chapter 9 authored** (`IT-C09-mesi`, `-stagioni`): the calendar year, atom-first,
  reviewing Ch.6–8 via `reviews_of`, with Spanish twins supplied.
- **The months, closest to Latin** of the sisters (*gennaio* keeps *Januarius*'s
  *-aio*; *ottobre* echoes *otto*): the god/emperor parade (Janus, Mars, Maia, Juno,
  Julius, Augustus), with the payoffs — *marzo* is the **same Mars** as *martedì*,
  and *settembre–dicembre* are the Latin **7–10** (Roman year began in March).
- **The seasons**: *primavera* = *prima vera*, "**first spring / first green**"
  (from Latin *vēr*); *estate* ← *aestas*; *autunno* ← *autumnus*; *inverno* ←
  *hibernum*. Even *stagione* is Latin *statiō*, "a standing" — a *station* of the
  year.
- Taxonomy: namespaced `IT-MONTHS`, `IT-SEASONS`.

## Chapter 8 — Time & the clock

- **Chapter 8 authored** (`IT-C08-ora`, `-mezzogiorno-mezzanotte`): telling the
  time, atom-first, reviewing Ch.6–7 via `reviews_of`.
- **ora** ← Latin *hōra* ← Greek *hṓrā* — the **closest of the sisters to Latin**
  (French wore it to *heure*; Italian barely touched *hōra → ora*). The Italian
  twist: time is told with the **feminine article**, the word *ore* left implied —
  *è l'una* (one, singular) but *sono le due* ("they are the two [hours]").
- **mezzogiorno / mezzanotte** — noon/midnight = *mezzo/mezza* ("half/middle," ←
  *medius*, cousin of French *mi-*) + *giorno* (← *diurnum*, root of *journal/
  journey*) / *notte* (← *noctem*). *mezza*notte is feminine (for *notte*),
  *mezzo*giorno masculine (for *giorno*) — the gender system again.
- Taxonomy: namespaced `IT-TIME-HOUR`, `IT-TIME-NOON-MIDNIGHT`.

## Chapter 7 — Days of the week

- **Chapter 7 authored** (`IT-C07-giorni-1`, `-giorni-2`): the seven days,
  atom-first, reviewing Ch.6 via `reviews_of`, with Spanish/French twins supplied.
- **giorni-1** (lunedì–venerdì): the **planet-week** with the accented **-dì**
  (← *diēs* "day") kept audible and stressed (*lu-ne-DÌ*). Three-sister lines make
  the shared Latin visible — *lunedì / lunes / lundi* are one word, *lūnae diēs*,
  worn three ways (IT/FR keep the day-word at the end, Spanish dropped it);
  *giovedì* wears *Giove* (Jupiter), the king-god English honours as Thor.
- **giorni-2** (sabato, domenica): the religious weekend — *sabato* ← *Sabbatum*
  (Hebrew *shabbāt*), the Sabbath every Romance language kept; *domenica* ← *(diēs)
  Dominica* "the **Lord's** day" (*Dominus* → dominion/dame), feminine *la domenica*.
- Taxonomy: namespaced `IT-DAYS-WEEKDAYS`, `IT-DAYS-WEEKEND`.

## Chapter 6 — Numbers 1–10

- **Chapter 6 authored** (`IT-C06-numeri-1-5`, `-numeri-6-10`): counting to ten,
  atom-first, each ~4 min, reviewing Ch.5 via `reviews_of`; Spanish and French
  twins supplied for each.
- **Italian kept the numbers closest to Latin** — it stayed next to Rome and wore
  them down least: *cinque* keeps Latin *quīnque*'s *-que* whole (vs Spanish *cinco*
  / French *cinq*), and Latin's *-pt-*/*-ct-* clusters **assimilate** to a doubled
  *-tt-* rather than dropping (*septem → sette*, *octō → otto*), so *otto* and
  *ottobre* still show the 8 side by side.
- **6–10** (*sei/sette/otto/nove/dieci*) carry the **settembre–dicembre = Latin
  7–10** calendar trick (the Roman year began in March; *luglio/agosto* displaced
  the counting months).
- Taxonomy: namespaced `IT-NUM-1-5`, `IT-NUM-6-10`.

## Chapter 5 — The first verbs (sentences start to move)

- **Chapter 5 authored** (`IT-C05-parlare`, `-abitare`, `-lavorare`,
  `-parlo-italiano`, `-practice`): Italian's first **grammar-engine** chapter,
  parallel to Spanish Ch.6 / French Ch.5 / German Ch.5. The learner stops reciting
  phrases and starts **building sentences from a pattern**.
- **The regular -are present tense** — drop *-are*, add *-o/-i/-a/-iamo/-ate/-ano*.
  Taught on **parlare**, cemented on **abitare** and **lavorare**. Italian is
  **pro-drop** (drops *io*, like Spanish).
- **The pronoun-rule circle closed** across five languages: **drop** (Spanish
  *hablo*, Italian *parlo* — distinct endings) vs **keep** (French *je parle* —
  silent endings; German *ich lerne* — grammar needs a subject).
- **Etymology, with cross-language contrasts**: *parlare* ← *parabolāre* "tell
  parables" (= French *parler*; Spanish *hablar* is from *fabulārī* instead);
  *abitare* ← *habitāre* (twin of *habiter*); **the "work" split** — *lavorare* ←
  *labōrāre* "to labour" (→ labor/laboratory/elaborate) where Spanish *trabajar* /
  French *travailler* come from *tripalium*, "torture." First self-assembled
  sentence: **Parlo italiano** (*italiano* ← *Italia*, perhaps "land of calves").
- Taxonomy: namespaced `IT-VERB-PARLARE/ABITARE/LAVORARE`, `IT-WORD-ITALIANO`.

## Chapter 3 — Introducing Yourself

- **Chapter 3 authored** (`IT-C03-io`, `-mi-chiamo`, `-come-ti-chiami`,
  `-piacere`, `-practice`): fills the gap between the greetings/how-are-you
  chapters and the farewells, so Italian now runs greet → introduce →
  how-are-you → goodbye end to end. Each lesson reviews Chapter 2.
- **io** (← *ego*) introduces the **pro-drop** habit — Italian usually omits the
  subject pronoun because the verb ending already carries it (shared with
  Spanish *yo* / Portuguese *eu*).
- **mi chiamo** — "I call myself" (*chiamarsi* ← Latin *clāmāre* "to call out" →
  claim/exclaim/clamor), completing the Romance naming-verb set: Spanish *me
  llamo* (*cl-*→*ll-*), Italian *mi chiamo* (*ch* = hard *k*), Portuguese *me
  chamo* (*ch* = *sh*).
- **Come ti chiami? / Come si chiama?** — the name asked with *come* ("how"), tu
  vs Lei; **piacere** ← *placēre* "to please" (please/pleasure/placid; twin of
  Portuguese *prazer*).
- Uses canonical `PRONOUN-I`, `INTRO-MY-NAME-IS`, `INTRO-WHATS-YOUR-NAME`,
  `INTRO-NICE-TO-MEET-YOU` — no taxonomy change.

## Chapter 4 — Farewells

- **Chapter 4 authored** (`IT-C04-arrivederci`, `-a-domani`, `-a-presto`,
  `-a-piu-tardi`, `-practice`): closing a conversation, reviewing Chapter 2. The
  learner can now run an Italian exchange end to end. Numbered Ch. 4 so the
  introductions chapter can slot in at Ch. 3.
- **The "until re-seeing" family**: *arrivederci* = *a + ri(re-) + vedere 'see'
  + ci 'us'* → "to our seeing-again" — shown beside French *au revoir* and German
  *auf Wiedersehen*, all the same gesture. The heavier *addio* (*a Dio*, "to
  God") is flagged as the twin of Spanish *adiós*.
- **"See you when" set**, each an atom traced: *a domani* (*domani* ← *dē māne*
  "from the morning", kin of Spanish *mañana*); *a presto* (*presto* ← *praestō*
  "at hand" → the English music/magic word); *a più tardi* (*più* ~ *plūs*/plus,
  *tardi* ~ *tardus*/tardy).
- Uses the canonical `FAREWELL`, `FAREWELL-TOMORROW`, `FAREWELL-SOON`,
  `FAREWELL-LATER` concepts (shared with the Spanish/French/German farewells) —
  no taxonomy change.

## Chapter 2 — "Come stai?" (the how-are-you chapter)

- **Chapter 2 authored** (`IT-C02-prego`, `-come`, `-stare`, `-come-stai`,
  `-come-sta`, `-come-va`, `-cosi-cosi`, `-practice`): the "how are you?"
  exchange, atom-first, reviewing
  Chapter 1. Fourth track in the PR's cross-language how-are-you set, reusing the
  canonical concepts `STATE-HOW-ARE-YOU`, `COURTESY-YOUREWELCOME`, `WORD-SOSO`.
  Register (tu/Lei) and the question word (come) are introduced inline, since the
  track had no separate introductions chapter yet.
- **Italian sits between the two metaphors**: it asks *Come stai?* on **stare**
  (← Latin *stāre* "to stand" — literally Spanish *estar* with the propping *e-*
  removed) **and** *Come va?* on **andare** ("to go") — so it bridges the
  Spanish "stand" and the French/German "go."
- **Etymology hooks**: *prego* ← *pregare* "to pray" (→ pray/precarious/deprecate),
  behaving like German *bitte*; *come* ← *quōmodo* (sibling of *cómo*/*comment*);
  *così così* ← *(ec)cum sīc* "thus" (the *[sic]* English still writes) — and
  English "so-so" is a loan translation of it.
- Taxonomy: namespaced `IT-VERB-STARE` documented.

## Chapter 1 — Greetings (track bootstrapped)

- New Italian track on the HL00 framework: one word per lesson, slug ids,
  gender-before-nouns, atom-first, derivations shown, LaTeX book (Latin Modern;
  CI auto-discovers `italian/book/`).
- Chapter 1 (`lessons/IT-C01-*`), atom-first, with Italian's closest-to-Latin
  flavour:
  - **ciao** ("hi/bye" ← *s-ciào*, "I am your slave" ← Latin *sclavus*; English
    *slave*, *Slav*) — the showpiece etymology.
  - **buono / buon** ("good" ← *bonus*; adjective agreement).
  - **il / la / lo** ("the"; grammatical gender ← *ille/illa*; the two masculine
    articles).
  - **giorno** ("day" ← *diurnum* ← *dies*; English *journal*, *journey*;
    plural by vowel-change).
  - **buongiorno** (assembled).
  - **sera / buonasera** ("evening" ← *serus*; feminine agreement).
  - **notte / buonanotte** ("night" ← *noctem*; the Latin *-ct-* → Italian
    *-tt-* rule vs. Spanish *-ch-* / French *-it-*).
  - **grazie** ("thanks" ← *gratia*, "grace"; English *grace*, *gratitude*).
  - **practice**.
- Grounds each word against English + Latin, with Spanish/French supplied for
  contrast (beginner-audience, no prior knowledge assumed). Book compiles clean
  with XeLaTeX.
