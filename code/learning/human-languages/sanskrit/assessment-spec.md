# Coding Adventures Sanskrit Assessment

**Version:** 1.0 target contract, 2026-09-13

**Basis:** project-defined CEFR-aligned equivalent

**Status:** target specified; the pre-A1 task inventory is checked in, while
A1-C2 inventories, mocks, calibration, and human validation remain backlog

This specification names the assessment that the complete Sanskrit book must
eventually prepare a book-only learner to pass. It is not an external
qualification, and it does not claim that the current book is exam-ready. The
learner-facing name at each rung is **Coding Adventures Sanskrit <Level>
Assessment — project-defined equivalent**.

No `assessment.json` is checked in for Sanskrit yet, and that omission is
deliberate. The machine-readable contract names required artifacts by path, and
the artifact gate treats a path that leads nowhere as an error rather than as a
promise. The contract is written after the inventories, mocks, rubrics and
answer keys it points at, not before them.

## Why a classical language gets a speaking paper

This is the first question a reader will ask and it deserves answering before
anything else, because the obvious answer — *Sanskrit is a classical language,
so test reading and writing and leave it there* — would quietly change what this
book is.

It would also be wrong about the world. **Samskrita Bharati**, the largest
Sanskrit-teaching movement there is, teaches Sanskrit **through conversation**.
Its graded correspondence ladder runs **Pravesha, Parichaya, Shiksha, Kovida**,
with an optional examination after each level, held twice a year. Its flagship
four-year certificate, run with Kavikulaguru Kalidas Sanskrit University, is
named **सम्भाषणतः शास्त्रपर्यन्तम्** — *from conversation to the śāstras* — and
the order in that title is the argument. Speech is not the afterthought at the
end of the classical training; it is where the training starts.

So a four-skill contract is the right shape for Sanskrit rather than a category
error, and a speaking paper measures something a large body of teachers already
teaches and examines. What the contract must say, and does, is **who the
listener is**: a response is scored for intelligibility to a trained speaker of
the living spoken register, which is the standard Samskrita Bharati's own
examinations use.

Samskrita Bharati's examinations are nonetheless optional at each level and
publish no per-skill pass rule, and the Central Sanskrit University's programmes
are degree and śāstra study rather than a proficiency ladder for a beginner. So
the target here stays project-defined, and the rule is stated in the open.

## The sandhi decision, which Sanskrit cannot leave unanswered

Written Sanskrit joins its words. **सः भारतात् आगच्छति**, written as this book
writes it, would in running Sanskrit be joined at every boundary, and a reader
of real Sanskrit meets the joined form. An examination has to say which it
requires, because the difference is not cosmetic: external sandhi changes the
surface of nearly every word in a sentence, and requiring it at pre-A1 would be
requiring a whole grammar the learner has not met.

This specification settles it as follows:

- **Pre-A1 and A1 present and accept unjoined text.** Every word is written with
  space around it and its ending intact, which is what the book itself does, and
  what lets a first reader see the joints while they are still learning where
  they are. A candidate who applies sandhi correctly is not penalised.
- **From A2 onward, reading input applies external sandhi** and written
  production is scored for it, because by then the machinery has been taught and
  a reader who cannot undo a join cannot read.
- Internal sandhi inside a single word is in scope at every level, since it is
  part of knowing the word at all.

The assessed register is the classical language as taught, in Devanagari. Vedic
accent and Vedic morphology are out of scope at every rung of this contract;
where a text carries accent marks they are supplied as given and never required
in production.

Sources checked 2026-09-13:

- [CEFR Companion Volume and language versions](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-companion-volume-and-its-language-versions)
- [Council of Europe CEFR descriptors](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-descriptors)
- [Council of Europe guidance on tests and examinations](https://www.coe.int/en/web/common-european-framework-reference-languages/tests-and-examinations)
- [Council of Europe introduction and certification caveat](https://www.coe.int/en/web/common-european-framework-reference-languages/introduction-and-context)
- [Samskrita Bharati USA — s2s certificate course](https://samskritabharatiusa.org/sbusa/classes/s2s-certificate-course/)
- [Central Sanskrit University](https://www.sanskrit.nic.in/)

## Pass rule

Reading, listening, writing, and speaking are separate 100-point papers. A
candidate must score at least **60/100 on every paper in the same
administration**. There is no aggregate compensation: 90 in reading cannot hide
59 in writing. An unattempted required part scores zero. A complete pass also
requires both published timed mocks at the rung to have been passed under the
same independent thresholds before the book may call the learner mock ready.

Writing and speaking use four equally weighted analytic dimensions:

1. task fulfilment and relevant content;
2. comprehensibility, organisation, and interaction;
3. range and control of Sanskrit vocabulary and grammar;
4. Devanagari orthographic control for writing, or pronunciation and fluency for
   speaking.

Each dimension is scored 0–5, then scaled to the paper's points. A response that
is not meaningfully in Sanskrit cannot earn task-fulfilment credit.

Dimension 4 for writing is scored on three things:

- the **word-final visarga or anusvāra**, scored by name rather than folded into
  general orthography. Sanskrit's noun families are told apart by how a word
  ends, and **ः** and **ं** are the two endings that carry most of that work.
  The corpus writes the visarga **1,011** times against **103** anusvāras, so a
  learner who confuses them is not making a spelling slip — they are putting a
  word in the wrong family, and everything agreeing with it goes wrong too;
- the **śirorekhā**, the headline stroke that runs across the top of the line
  and is what makes Devanagari read as a line rather than as separate marks;
- **word boundary**, which at pre-A1 and A1 means keeping the words apart rather
  than joining them, per the sandhi decision above.

## The writing ramp, measured rather than assumed

Sanskrit has **50 script lessons** in its learning record and **not one
writing-stage directive** in any of them. Observe/trace, guided copy, delayed
copy and dictation/transcription are unproved for this track — not partly
proved, unproved — while the pre-A1 writing paper below requires delayed recall
and dictation.

The inventory describes the examination, not the corpus, and the gap between the
two documents is the work. Naming it is what makes it fixable; the alternative
is a contract that reads as satisfied by fifty lessons that do not stage
anything.

## Administration rules

- Listening recordings use trained speakers of the living spoken register.
  Words-per-minute bands below are form-assembly targets.
- Pre-A1 and A1 directions may also be supplied in a declared support language,
  so misunderstanding directions is not confused with Sanskrit proficiency. From
  A2 onward directions are in Sanskrit, with one unscored example.
- No dictionary, grammar reference, spell-checker, translator, or generative
  assistant is allowed. Ordinary accessibility accommodations may change
  presentation or response mode without changing the construct; every change is
  recorded.
- Listening pause and replay rules are fixed below. A technical restart replaces
  the affected recording; it is not an extra candidate-selected replay.
- Speaking is recorded and independently scored by two trained raters. A third
  rater adjudicates when scaled totals differ by more than 10 points, or when
  the first two raters disagree about a pronunciation tradition — regional
  recitation traditions differ, and a consistent one is not an error.
- Full mocks use unseen prompts and the target timing.

The 60% threshold is a transparent provisional project standard, not validation
evidence. Before release, at least eight qualified Sanskrit educators or
linguists must perform a modified Angoff review; two raters must double-score at
least 50 writing and 50 speaking samples per rung; and a book-only pilot must
report every skill separately. The contract is versioned if standard setting
changes the threshold or task envelope.

## Level envelopes

The later `task-shapes/<level>.json` inventories must make every part below
executable without expanding its input, response, timing, replay, interaction,
or aid boundary. From A2 the envelope must additionally carry externally
sandhi-joined reading input. The A1-C2 bands are the project's own shared
ladder, identical across tracks by design.

### pre-A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 10 | 6 Devanagari shape/word matches, 4 short phrase or notice matches, and 4 personal-detail selections; unjoined text, none over 8 words |
| listening | 10 | 6 sound/word recognitions, 4 greeting-response choices, and 4 personal-detail selections; 70–90 wpm, two plays, 8-second inter-item pauses |
| writing | 12 | 2 delayed-copy items, 4 one-word dictation items, and 2 independently produced personal or greeting responses; no visible answer model during scoring |
| speaking | 8 | return a greeting, state a name or chosen identity, answer 3 familiar one-turn questions, and make 1 short request; the interlocutor may repeat once slowly |

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
