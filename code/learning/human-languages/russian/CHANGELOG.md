# Changelog — Russian track

## Chapter 2 — Introducing yourself

Russian was one of **two tracks still at Chapter 1** (with Latin); every other
track had reached Ch. 4 or beyond. This closes that gap, following the
Ch. 2 plan the roadmap already set out.

- **`RU-C02-ya`** — *я*, one letter and one of the least-changed words in the
  family: PIE \**eǵh₂(om)* → Proto-Slavic \**azъ* → a single vowel. Cousins
  *ego*, *ich*, *I*, *egṓ*. **Pronouns are the least borrowable part of a
  language**, which is why this row survives when everything around it turns over.
  Warns that **я is not a mirrored Latin R**.
- **`RU-C02-ty-vy`** — the split English threw away. *Thou* fell out of standard
  English through the 1600s (hedged — it survives in Quaker usage, northern
  dialect and liturgy), leaving *you* for everyone.
  - *вы* is polite the way French *vous* is, and the lesson is careful about
    **who actually did what**: Russian and French use the **2nd-person plural**,
    German borrows the **3rd-person plural** (*Sie*, **not** *ihr*), and **Spanish
    used no plural at all** — *usted* ← *vuestra merced*, "your grace". A draft
    lumped Spanish in; `ES-C03-tu-usted` and `GE-C02-du-sie` both say otherwise.
    Russian's *вы*-politeness is also flagged as a likely **18th-century import**
    rather than an independent invention.
  - The cognate table is **honest about its third row**: *вы* and *vōs* continue
    PIE \**wos*, but English *you* and German *ihr* come from the paradigm's
    **other** stem \**yūs* — so that row is half a set, and says so.
  - Introduces **ы** properly, since *ты*/*вы* turn on it and it is the hardest
    Russian vowel for an English speaker — and **adds it to
    `pronunciation-reference.md`**, where it was missing from every letter group,
    with a new `yery-vowel` id the lesson cites. The reference previously had no
    entry for it at all.
- **`RU-C02-menya-zovut`** — the naming construction, which is the chapter's real
  content. *Меня зовут Анна* is literally "**[they] call me Anna**": there is
  **no word for "my"** and **no word for "name"** in it. The "they" is nobody — a
  bare plural verb meaning "people in general", which English does too in *they
  say it'll rain*.
  - This is the course's **first look at case**. *Меня* is not *я* but its object
    form, and the lesson is careful to set the expectation honestly: English does
    this with about six **pairs** (*I/me*, *he/him*…), Russian does it with
    **every noun and pronoun**. The learner isn't asked to learn the system — only to notice
    that the *shape* carries the meaning.
- **`RU-C02-kak-vas-zovut`** — asks **how** they call you, not *what*. Russian,
  French (*comment*) and Spanish (*cómo*) all ask about an **action**; English is
  the odd one out, asking about a **possession**. Completes the object-form set
  (*меня / тебя / вас*) so the exchange is a matched pair.
- **`RU-C02-ochen-priyatno`** — "very pleasant", with no *I am* and no *to meet
  you*. The etymology is the payoff: *приятно* ← Slavic *prijati* "to favour" ←
  PIE \**preyH-* "to love, please" — the root behind Russian **приятель**
  "friend" **and** English **friend**, arriving from opposite ends of Europe.
  **Free** most likely belongs to the same family ("belonging to the beloved
  household", i.e. not a slave), hedged as the usual account rather than asserted.
- **`RU-C02-practice`** — drills the full formal exchange, then the informal one
  (**greeting and pronoun change; the verb *зовут* never does**), two of the
  shapes each pronoun takes, and a "what Russian leaves out" table.
  - The zero-copula point is **scoped to the one sentence that shows it**:
    ***очень приятно*** has no verb at all. A draft claimed "no sentence in the
    chapter contains a word for *is*" and that "Russian has none in the present"
    — both wrong. *Меня зовут* has a verb (*зовут*); it simply isn't a copular
    sentence. And Russian **does** have a present-tense **есть**, which Ch. 1
    already met inside **нет** = *не + есть*, and which the roadmap's own Ch. 3
    section reuses for *у меня есть*.

### Conventions checked, not assumed

- Concept tags are **canonical** (`PRONOUN-I`, `PRONOUN-YOU`, `INTRO-MY-NAME-IS`,
  `INTRO-WHATS-YOUR-NAME`, `INTRO-NICE-TO-MEET-YOU`) — verified present in
  `concepts/taxonomy.json`, so no new entries were needed.
- **я and ты/вы are separate lessons**, which is why. A draft covered all three in
  one lesson tagged `PRONOUN-I` — leaving Russian with **no `PRONOUN-YOU` node**
  at all, and dropping the ты/вы split out of a join that **14 other lessons** take part in.
  **Most** other tracks split them the same way (`FR-C02-je` + `FR-C02-tu-vous`,
  `GE-C02-ich` + `GE-C02-du-sie`) — though Italian and Portuguese do not, carrying
  only `PRONOUN-I`.
- The practice lesson matches **this track's own** shape — `type: practice-mix`
  with `CH2-PRACTICE` — not the Arabic track's `practice`/`REVIEW`. `CH2-PRACTICE`
  was already in the taxonomy note's label list.
- `sounds:` ids come from `russian/pronunciation-reference.md`. A first draft used
  `cyrillic-new-shapes`, which **is** used by `RU-W03`/`RU-W04` but is **missing
  from the reference's id list** (as is `cyrillic-honest`) — so the new lessons
  use `cyrillic-false-friends` and `stress-unmarked`, both canonical.
- The read-now-draw-later notes (**я**, **ч**, **ы**, **ь**) were written after listing
  the writing track's actual headwords (в р · с н · б д · п и · е т) — none of
  those four has been taught.

## [Unreleased]

### Added — Chapter 1 (Greetings & courtesy)
- Track scaffold: `README.md`, `roadmap.md`, `session-map.md`,
  `pronunciation-reference.md`, and `track.json` declaring the **Cyrillic**
  script (so the data layer resolves Russian → cyrillic).
- Six word lessons, Cyrillic taught inline:
  - `RU-C01-privet` — привет (informal hi); the *-вет* "speak" root ↔ **Soviet**.
  - `RU-C01-zdravstvuyte` — здравствуйте (formal hello); "be healthy", polite `-те`.
  - `RU-C01-spasibo` — спасибо (thank you); worn-down *спаси Бог*, "God save you".
  - `RU-C01-da` — да (yes).
  - `RU-C01-net` — нет (no); *не + есть* "not-is", the PIE **\*ne** cousin of *no/not*.
  - `RU-C01-pozhaluysta` — пожалуйста (please / you're welcome); the favour root *жал-*.
- `RU-C01-practice` — Chapter 1 recap drilling the four false friends (в=v, р=r,
  с=s, н=n) and the greeting exchange.
- Uses the canonical concept taxonomy; adds `COURTESY-PLEASE` to the taxonomy for
  пожалуйста.

### Added — Writing the letters (the "break it apart and write it" strand)
- Three `writing`-type lessons (the HL02 hand-writing surface, taught inline the
  same etymology-first way; no `concept_tag`, exempt from the cross-language join).
  Each breaks a letter into its component strokes with a stroke order and reviews
  the Chapter 1 word it lives in:
  - `RU-W01-false-friends-v-r` — writing **в** (v, ← Greek beta) and **р** (r, ←
    Greek rho): the two false friends from *привет*, stroke by stroke.
  - `RU-W02-false-friends-s-n` — writing **с** (s, ← Greek sigma) and **н** (n,
    the Latin-*H* look-alike), completing the four false friends в·р·с·н.
  - `RU-W03-new-shapes-b-d` — writing **б** (b) and **д** (d, ← Greek delta), two
    shapes with no Latin disguise; contrasts б vs в (the top flag + one belly vs
    two bellies).
  - `RU-W04-privet-letters-p-i` — writing **п** (p, ← Greek pi Π) and **и** (ee,
    the quiet false friend — a *backwards* Latin N: its diagonal **rises** where
    N's falls); contrasts п (top bar) vs н (middle bar).
  - `RU-W05-privet-letters-e-t` — writing **е** (ye, an *iotated* honest vowel) and
    **т** (t, ← Greek tau); **completes every letter of привет** (п·р·и·в·е·т), so
    the learner can hand-write their first Russian word end to end.
- Stroke data is the canonical `data/scripts/cyrillic.json` the companion
  `language-ladder` app renders, so the lessons and the app agree.

### Notes
- Headwords use the lowercase citation form (Cyrillic case is not yet in the
  script inventory).
- The LaTeX book is authored next (lessons-first workflow), typeset with the
  vendored `NotoSansCyrillic-Static.ttf`.
