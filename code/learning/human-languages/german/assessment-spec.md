# Coding Adventures German Assessment

**Version:** 1.0 target contract, 2026-08-21

**Basis:** project-defined pre-A1 bridge; official adult Goethe-Zertifikat
targets from A1 through C2

**Status:** target contracted; most task inventories, mocks, calibration, and
book-only human validation remain backlog

This specification names the examinations that the complete German book must
eventually prepare a book-only learner to pass. It does **not** claim that the
current book is exam-ready. The checked-in [assessment.json](assessment.json)
is the machine-readable contract. A path in that file is a dependency, not
evidence that an inventory, rubric, answer key, or mock already exists.

| curriculum rung | assessment target |
|---|---|
| pre-A1 | Coding Adventures German pre-A1 Assessment — project-defined Goethe precursor |
| A1 | Goethe-Zertifikat A1: Start Deutsch 1 for adults |
| A2 | Goethe-Zertifikat A2 for adults |
| B1 | Goethe-Zertifikat B1 for adults |
| B2 | Goethe-Zertifikat B2 for adults |
| C1 | Goethe-Zertifikat C1 |
| C2 | Goethe-Zertifikat C2: Großes Deutsches Sprachdiplom |

The youth variants are not silently blended into this adult ladder. A future
inventory may add a separately sourced youth variant, but every mock must name
the one real form it administers. Pre-A1 is an original four-skill runway; it
is not a Goethe certificate or an assertion that Goethe offers a pre-A1 exam.

## Official evidence

Goethe-Institut publishes adult examinations at every CEFR rung from A1 through
C2. A1 and A2 are whole exams with reading, listening, writing, and speaking
sections. B1, B2, C1, and C2 have four modules with those same skill names; the
modules may be taken separately or in combination.

Sources checked 2026-08-21:

- [Goethe-Institut German examinations A1–C2](https://www.goethe.de/en/spr/prf.html)
- [A1 exam information](https://www.goethe.de/ins/de/en/prf/prf/gzsd1.html)
- [A1 result and pass rule](https://www.goethe.de/de/spr/prf/pes/pas1.html)
- [A1 official accessible model](https://bfu.goethe.de/a1_sd1/)
- [A2 exam information](https://www.goethe.de/ins/de/en/prf/prf/gzsd2.html)
- [A2 result and pass rule](https://www.goethe.de/de/spr/prf/pes/paa2.html)
- [B1 module information](https://www.goethe.de/ins/de/en/prf/prf/gzb1/inf.html)
- [B1 result and pass rule](https://www.goethe.de/en/spr/prf/pes/pab1.html)
- [B2 exam information](https://www.goethe.de/ins/de/en/prf/prf/gzb2.html)
- [B2 result and pass rule](https://www.goethe.de/de/spr/prf/pes/pab2.html)
- [C1 exam information](https://www.goethe.de/ins/de/en/prf/prf/gzc1.html)
- [C1 result and pass rule](https://www.goethe.de/de/spr/prf/pes/pac1.html)
- [C2 exam information](https://www.goethe.de/ins/de/en/prf/prf/gzc2.html)
- [C2 result and pass rule](https://www.goethe.de/en/spr/prf/pes/pac2.html)

Official regulations, current practice materials, and scoring forms are the
authority when a task count, time, option, or rating rule changes. Every future
task-shape inventory must record the form and source edition it reproduces.
C2 inventories must also date any literature-list option; they may not freeze a
retired reading list into the permanent contract.

## Official pass arithmetic and the stronger readiness rule

The external pass mechanics differ by rung:

- **A1:** all sections must be taken and the combined result must reach 60/100.
  The official maximum is 75 points for the written exam and 25 for speaking.
- **A2:** all sections must be taken, the combined result must reach 60/100,
  the written exam must reach 45/75, and speaking must reach 15/25.
- **B1–C2:** reading, listening, writing, and speaking are separate 100-point
  modules. Each module is passed at 60/100. Four individual module certificates
  are equivalent to the corresponding overall certificate.

Original mocks must reproduce the relevant official arithmetic. The book adds
one deliberately stricter diagnostic rule: reading, listening, writing, and
speaking must each reach **60/100 on both complete timed mocks** at every rung.
This blocks A1 or A2 readiness if aggregate compensation hides a weak skill. It
does not rewrite Goethe's awarding rule.

Goethe's current official rating grids govern the external-format mocks.
Project scoring notes may make task fulfilment, coherence, range and control,
orthography, pronunciation, fluency, and interaction more observable, but may
not silently replace the official criteria. German writing lessons explicitly
build noun capitalization, sentence punctuation, umlauts, `ß`, word order,
register, and genre. A variant receives credit only when the current official
criteria and qualified raters accept it; this contract does not invent a list
of supposedly equivalent spellings.

## Administration and the gentle writing ramp

- Every lesson remains five minutes or shorter. Full mocks preserve official
  continuous timing because exam endurance is evidence, not instruction.
- Writing begins at pre-A1 with observe/trace, guided copy, delayed copy, and
  dictation/transcription. A1 adds controlled and timed production; A2 adds
  connected composition. Every later rung retains the entire cumulative ramp.
- Tracing and a visible model are learning supports. Only independent work
  within the mock's aid boundary earns readiness evidence.
- Early explanations may use a declared support language. Scored stimulus and
  response language follow the official adult form.
- No dictionary, translator, grammar reference, spell-checker, or generative
  assistant is allowed unless the current official rules or a documented
  accommodation expressly permit it.
- Local writing and speaking samples are independently double-scored by trained
  raters, with material disagreement adjudicated. Public sample wording is not
  copied into repository mocks; original prompts reproduce the construct.
- Two full mocks are the minimum final gate. Five-minute lessons, part practice,
  and progressively longer rehearsals build stamina before continuous timing.

## Level envelopes

Future `task-shapes/<level>.json` files make each envelope executable. They must
pin exact parts, input and response sizes, replay, interaction, aids, scoring,
and edition rather than treating the summary below as a complete form.

### pre-A1

The project-defined bridge has independently scored reading, listening,
writing, and speaking papers. It covers familiar signs and words, very short
slow exchanges, greetings and identity, simple requests, and short independent
German-script responses. The first four writing stages must be proven before a
timed mock. Its duration and task inventory remain separate backlog; it must be
gentler and shorter than A1 without being mislabeled as an external exam.

### A1

The adult target is Goethe-Zertifikat A1: Start Deutsch 1. The current form has
25 minutes reading, about 20 minutes listening, 20 minutes writing, and a
15-minute oral group exam. Writing completes a simple form and produces a short
personal everyday text. Speaking introduces the candidate, exchanges familiar
questions and answers, and makes and answers a request. German's existing
[A1 inventory](task-shapes/a1.json) is the executable source for future mocks.

### A2

The adult Goethe-Zertifikat A2 currently gives 30 minutes each to reading,
listening, and writing, plus a 15-minute pair speaking exam. Reading samples
short articles, email, advertising, and public information; listening samples
everyday conversation, announcements, interviews, phone messages, and public
announcements. Writing produces everyday messages. Speaking asks and answers
personal questions, recounts familiar life, and makes plans or agreements.

### B1

Goethe-Zertifikat B1 is modular. Reading is 65 minutes, listening about 40,
writing 60, and pair speaking about 15. Reading covers blogs, email, journalism,
advertising, and instructions. Writing produces personal and formal
correspondence plus an opinion forum post. Speaking combines collaborative
planning, an everyday presentation, questions, opinions, and suggestions.

### B2

Goethe-Zertifikat B2 is modular. Reading is 65 minutes, listening about 40,
writing 75, and pair speaking 15. Writing requires a justified forum position
on a current social issue and a formal professional message. Speaking combines
a short presentation, partner discussion, and exchange of arguments. The book
must build register control and defended production, not merely B2 vocabulary.

### C1

The current modular Goethe-Zertifikat C1 gives 65 minutes to reading, about 40
to listening, 75 to writing, and 20 to pair speaking. Writing argues a position
on a current social issue and produces a formal message with audience-appropriate
tone, style, and register. Speaking presents a complex topic and sustains a
controversial discussion. Future inventory work must use the current modular
form rather than a retired pre-2024 C1 format.

### C2

Goethe-Zertifikat C2: Großes Deutsches Sprachdiplom is modular. Reading is 80
minutes, listening about 35, writing 80, and individual speaking about 15.
Reading handles complex factual, journalistic, and public texts with implicit
meaning. Listening uses natural-speed media, conversation, and expert interview.
Writing reformulates a short presentation and produces a well-structured,
stylistically appropriate letter or review from a general or current
literature-list option. Speaking gives a complex presentation and defends a
position through detailed follow-up and pro/con discussion.

## Required artifacts and readiness language

Every rung still requires:

1. a dated four-skill task-shape inventory (only German A1 is present today);
2. two original, complete, timed mock forms;
3. official-grid-aligned rubrics and task-specific scoring notes;
4. answer keys, acceptable variants, and rater training samples;
5. documented calibration against current official practice material; and
6. a preregistered book-only human-validation report with every skill reported.

Until the first four exist, reports may say **target contracted** only. Passing
both calibrated mocks supports **mock ready**. Only the human study supports
**learner proven**. Chapter count, vocabulary count, CEFR labels, and the
presence of this contract are not readiness evidence.
