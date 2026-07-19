# Changelog

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
  `-cosi-cosi`, `-practice`): the "how are you?" exchange, atom-first, reviewing
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
