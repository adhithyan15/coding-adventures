# Coding Adventures Portuguese Assessment

**Version:** 1.1 target contract, 2026-09-13

**Basis:** CAPLE from A1 upward; a project-defined precursor below it

**Status:** target specified; the pre-A1 task inventory is checked in, while
A1-C2 inventories, mocks, calibration, and human validation remain backlog

This specification names the assessment that the complete Portuguese book must
eventually prepare a book-only learner to pass. At pre-A1 the learner-facing
name is **Coding Adventures Portuguese pre-A1 Assessment — project-defined CAPLE
ACESSO precursor**. It does not claim that the current book is exam-ready.

No `assessment.json` is checked in for Portuguese yet, and that omission is
deliberate. The machine-readable contract names required artifacts by path, and
the artifact gate treats a path that leads nowhere as an error rather than as a
promise. The contract is written after the inventories, mocks, rubrics and
answer keys it points at, not before them.

## The variety decision, and why the ladder makes it

Portuguese is the one track in this corpus where **the choice of variety is
forced by the certification ladder rather than by taste**, and it is worth
setting out in full because the reasoning is not obvious.

There are two officially recognised Portuguese certification systems:

| system | authority | variety | levels |
|---|---|---|---|
| **CAPLE** | Faculdade de Letras, Universidade de Lisboa | European | ACESSO **A1**, CIPLE A2, DEPLE B1, DIPLE B2, DAPLE C1, DUPLE C2 |
| **Celpe-Bras** | Brazilian Ministry of Education | Brazilian | one exam, four bands: Intermediário (~**B1**) to Avançado Superior |

Celpe-Bras is the only officially recognised certificate of Brazilian
Portuguese, and it is a **single examination** that classifies every candidate
into one of four bands. Its lowest band sits at roughly B1. **There is no
Brazilian Portuguese certificate at A1 or A2. There is no Brazilian Portuguese
certificate at pre-A1.**

A book that carries a reader from pre-A1 to C2 and wants each rung to be
certifiable therefore has exactly one option at the bottom of the ladder, and
that option certifies European Portuguese. The alternative is not "target
Brazilian instead" — it is "accept that the first three rungs certify nothing."

**This specification targets CAPLE, and therefore European Portuguese**, from A1
upward, with a project-defined precursor at pre-A1 because CAPLE has no
pre-A1 exam either. From B1, where Celpe-Bras becomes reachable, a Brazilian
route is a legitimate second target and the contract may be versioned to carry
both; below B1 there is nothing to carry.

### What the corpus currently does, measured

The track has not made this choice yet, and the evidence is mixed rather than
absent. Measured across its 116 lessons: **você** appears 85 times against 24
for **tu**, which leans Brazilian; but the course teaches **adeus** and **Como
está o senhor?**, both markedly European, and no variety-distinguishing
vocabulary item — not one of *ônibus/autocarro*, *trem/comboio*, *café da
manhã/pequeno-almoço*, *celular/telemóvel* — appears anywhere yet.

That is a track that has not committed rather than one that has committed
wrongly, which is the better position to be in and a narrow window in which to
decide. Bringing the taught forms into line with the declared target is
outstanding work, and it is named here so that it is visible rather than
discovered at A1.

## Evidence and scope

The proficiency backbone is the Council of Europe's 2020 CEFR Companion Volume,
including its pre-A1 descriptors. CEFR is a framework for describing proficiency
and developing transparent examinations, and the Council of Europe states that
it does not verify or validate an examination's claimed link to CEFR.

CAPLE is a unit of the Faculty of Letters of the University of Lisbon,
recognised by Portugal's Ministry of Foreign Affairs through Camões I.P., by the
Ministry of Education, and by the Ministry of Internal Administration. Its
ACESSO certificate attests communicative competence at CEFR **A1**, in four
components, over about one hour and thirty-five minutes, scaled 55%–100%. A
structures paper, **Competência Estrutural**, appears from DIPLE (B2) upward and
not below it.

Sources checked 2026-09-13:

- [CEFR Companion Volume and language versions](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-companion-volume-and-its-language-versions)
- [Council of Europe guidance on tests and examinations](https://www.coe.int/en/web/common-european-framework-reference-languages/tests-and-examinations)
- [Council of Europe introduction and certification caveat](https://www.coe.int/en/web/common-european-framework-reference-languages/introduction-and-context)
- [ACESSO ao Português — CAPLE, Universidade de Lisboa](https://caple.letras.ulisboa.pt/exame/12/acesso-e)
- [Exames — CAPLE, Universidade de Lisboa](https://caple.letras.ulisboa.pt/exames)
- [CELPE-Bras](https://en.wikipedia.org/wiki/CELPE-Bras)

## Pass rule

Reading, listening, writing, and speaking are separate 25-point papers at
pre-A1. A candidate must score at least **60% on every paper in the same
administration**. There is no aggregate compensation.

This is stricter than CAPLE's own rule, which is a single **55% overall** across
the four components. A single overall figure lets a strong reader carry a weak
writer, and this rung exists to prove all four moved together. Where the
contract is stricter than the target it says so, and it never claims the
stricter rule as a CAPLE equivalence.

Writing and speaking use four equally weighted analytic dimensions:

1. task fulfilment and relevant content;
2. comprehensibility, organisation, and interaction;
3. range and control of Portuguese vocabulary and grammar;
4. Portuguese orthographic control for writing, or pronunciation and fluency for
   speaking.

Dimension 4 for writing is scored on the two things Portuguese spelling asks for
that its neighbours do not:

- **nasal marking.** *pão*, *mãe*, *bom*, *sim*, *irmã* — Portuguese carries a
  whole series of nasal vowels and diphthongs, written with a tilde or with a
  following *m*/*n* that is not itself pronounced as a consonant. A learner who
  writes *pao* for *pão* has not misspelled a word; they have written a
  different one;
- **graphic accent placement** — *é*, *está*, *português*, *avô* against *avó*.
  Portuguese marks stress and vowel quality together, and the last pair differ
  in nothing else.

## The writing ramp, measured rather than assumed

Version 1.0 of this document recorded a gap: Portuguese had **one** writing
lesson, carrying observe-trace and guided-copy and nothing beyond, while the
pre-A1 paper below requires a delayed-recall item and a dictation item.

**That gap is now paid.** Chapter 1 carries the full pre-A1 ladder on the one
word it already had:

| stage | lesson | what the hand is asked to do |
|---|---|---|
| observe-trace | `PT-C01-ola` | notice the shape, trace it with the model visible |
| guided-copy | `PT-W01-ola-guided-copy` | copy it, model still on the page |
| delayed-copy | `PT-W01-ola-delayed-copy` | model covered, ten seconds, write from memory |
| dictation-transcription | `PT-W01-ola-dictation` | nothing on the page — write from the sound |

The order is load-bearing and enforced: a delayed copy is not valid evidence
unless the tracing and the guided copy come earlier in sequence.

**olá** is the right word to do this on because of its accent, which behaves
differently at each of the last two stages. Covered-model recall is where the
accent is the expected miss — it carries no sound of its own, so it is the part
a hand drops first when it stops copying and starts remembering. Dictation is
where it stops being a mark to remember and becomes one that can be **worked
out**: the ear hears *oh-LAH* rather than *OH-lah*, and the written accent
records exactly that stress. Which is what dimension 4's *graphic accent
placement* is asking for.

## Administration rules

- Listening recordings use speakers of the declared target variety. From A1 that
  is European Portuguese, per the decision above. Words-per-minute bands below
  are form-assembly targets, not a claim that Portuguese has one natural
  speaking rate.
- Pre-A1 directions may also be supplied in a declared support language, so
  misunderstanding directions is not confused with Portuguese proficiency. From
  A1 onward directions are in Portuguese, with one unscored example.
- No dictionary, grammar reference, spell-checker, translator, or generative
  assistant is allowed. Ordinary accessibility accommodations may change
  presentation or response mode without changing the construct; every change is
  recorded.
- A candidate who answers consistently in Brazilian Portuguese is not penalised
  for the variety at pre-A1 and A1, where the taught forms are still settling.
  Consistency is scored; the variety itself is not, until the contract is
  versioned to say otherwise.
- Speaking is recorded and independently scored by two trained raters. A third
  rater adjudicates when scaled totals differ by more than 10 points or when the
  first two raters disagree about a documented variety form.
- Full mocks use unseen prompts and the target timing.

The 60% pre-A1 threshold is a transparent provisional project standard, not
validation evidence. Before release, at least eight qualified Portuguese
educators or linguists must perform a modified Angoff review; two raters must
double-score at least 50 writing and 50 speaking samples per rung; and a
book-only pilot must report every skill separately.

## Level envelopes

The later `task-shapes/<level>.json` inventories must make every part below
executable without expanding its input, response, timing, replay, interaction,
or aid boundary. From B2 the envelope must additionally accommodate CAPLE's
Competência Estrutural paper. The A1-C2 bands are the project's own shared
ladder, identical across tracks by design.

### pre-A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 8 | known-word matches and one choose-the-reply part; no text over 8 words |
| listening | 7 | greeting recognition and catching a spoken name; two plays |
| writing | 10 | 1 delayed-recall item, 1 dictation item, and 1 bounded independent response; no visible answer model during scoring |
| speaking | 5 | return a greeting, state a name or chosen identity, answer familiar one-turn questions, and make 1 short request; 2 minutes preparation |

### A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 20 | 3 parts and 18–22 items across signs, forms, messages, and personal descriptions; 250–350 source words total, no text over 90 words |
| listening | 18 | 3 parts and 15–18 items across announcements, short exchanges, and a personal account; 90–110 wpm, two plays |
| writing | 20 | complete a practical form, then write a 30–40 word message for a named reader and purpose |
| speaking | 10 | personal interview, 1-minute prepared description, and a simple transactional role-play; 5 minutes preparation |

### A2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 30 | 3 parts and 22–26 items across notices, correspondence, and two connected everyday texts; 550–750 source words total |
| listening | 25 | 3 parts and 18–22 items across announcements, conversations, and one 2–3 minute account; 110–130 wpm, two plays |
| writing | 30 | a 25–35 word functional response and a 70–90 word connected message or description |
| speaking | 12 | interview, 2-minute picture or topic account, and a role-play requiring information exchange; 5 minutes preparation |

### B1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 45 | 4 parts and 26–32 items across correspondence, public information, narrative, and an informational article; 1,100–1,400 source words total |
| listening | 35 | 4 parts and 22–28 items across transactions, interviews, narrative, and a short talk; 130–150 wpm, two plays for short items and one replay for each long item |
| writing | 45 | a 50–70 word functional text and a 130–170 word connected narrative, description, or opinion for a named audience |
| speaking | 15 | interview, 3-minute prepared account, collaborative planning task, and follow-up discussion; 10 minutes preparation |

### B2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 60 | 4 parts and 30–36 items across argument, reporting, instructions, and literary or cultural prose; 1,800–2,300 source words total |
| listening | 45 | 4 parts and 24–30 items using multiple speakers and at least two regional voices; 150–170 wpm, long recordings played once after one orienting preview |
| writing | 60 | a 100–130 word interaction text and a 220–280 word report, article, narrative, or reasoned argument in the required register |
| speaking | 18 | 4-minute presentation, collaborative problem-solving, and defended discussion with follow-up questions; 10 minutes preparation |

### C1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 75 | 4 parts and 32–40 items across dense public, professional, academic, and literary texts; 2,800–3,500 source words total, including implicit stance |
| listening | 50 | 4 parts and 26–32 items across unscripted discussion, lecture, interview, and narrative; 160–185 wpm with natural variation, recordings played once |
| writing | 75 | synthesize two short sources in 180–220 words, then produce a 300–380 word audience-aware argument, report, or critical response |
| speaking | 22 | 5-minute source-based presentation, collaborative synthesis, and sustained challenge and defence; 15 minutes preparation with paper notes only |

### C2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 90 | 4 parts and 34–42 items across stylistically varied specialist, public, and literary texts; 4,000–5,000 source words, including ambiguity and intertextual stance |
| listening | 60 | 4 parts and 28–34 items across rapid multiparty interaction, extended argument, narrative, and culturally dense media; natural 165–200 wpm variation, recordings played once |
| writing | 90 | synthesize and reframe multiple sources for one audience in 220–280 words, then produce a precise 420–520 word extended text in a contrasting genre or register |
| speaking | 25 | 6-minute synthesis or mediation presentation, multiparty negotiation, and sustained defence requiring reformulation for a second audience; 15 minutes preparation with paper notes only |

## Required artifacts and readiness language

For each rung, implementation must add:

1. the four-skill JSON task inventory named by `assessment.json`;
2. two complete timed mock forms with fresh input and prompts;
3. a shared analytic rubric plus task-specific scoring notes;
4. one answer key per mock, including acceptable regional alternatives;
5. rater training samples, double-scoring evidence, and adjudication records;
6. the pre-registered book-only human-validation report.

Until items 1–4 exist, reports may say **target contracted** only. After a
learner passes both mocks they may say **mock ready**. Only the pre-registered
human study can support **learner proven**, and its result must never be inferred
from corpus coverage.
