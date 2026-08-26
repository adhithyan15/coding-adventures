# DELE A1 mock rubric — Spanish

This rubric governs `a1-mock-1` and `a1-mock-2`, the two timed full mocks the
assessment contract (`../../assessment.json`, level `A1`) has always named and
never had. It transcribes the awarding body's real rule; it does not improve it.

---

## 1. The target, and where its shape comes from

The target is **DELE A1** as administered under the *v2020* renewal, effective
April 2020. Everything in §2–§4 is transcribed from the official Instituto
Cervantes exam guide, cross-checked against the published Modelo 0 booklet and
the Cervantes exam pages.

| # | source | used for |
|---|---|---|
| S1 | [*Guía del examen DELE A1*, v2020, © 2019 Instituto Cervantes, NIPO 503-14-007-X](https://examenes.cervantes.es/sites/default/files/DELE_A1_v2020_Gu%C3%ADa%20de%20examen.pdf) | structure table, task descriptions, scoring formulas, group minimums |
| S2 | [Exámenes DELE A1](https://examenes.cervantes.es/es/dele/examenes/a1) | timings and points per prueba |
| S3 | [Renovación de los exámenes DELE A1 y A2](https://examenes.cervantes.es/a1a2/a1.htm) | the 2020 changes (3 options, not 4) |
| S4 | [Cómo son las pruebas DELE](https://examenes.cervantes.es/es/dele/como) | "APTO (30 puntos) en los dos grupos" |
| S5 | [DELE A1 Modelo 0, v2020](https://examenes.cervantes.es/sites/default/files/DELE-A1_v2020_Modelo0_0.pdf) | rubric wording, oral preparation, written word counts |
| S6 | [PCIC A1–A2 inventories](https://cvc.cervantes.es/ensenanza/biblioteca_ele/plan_curricular/niveles/) | the content sampled from — see §5 |

No exam item, text, or copyrighted content from any of these is reproduced.
Only the **structure** is transcribed. Every item in both mocks is original.

---

## 2. Structure

| Prueba | Duration | Tareas | Items | Points |
|---|---|---|---|---|
| 1 · Comprensión de lectura | 45 min | 4 | 5 + 6 + 6 + 8 = **25** | 25 |
| 2 · Comprensión auditiva | 25 min | 4 | 5 + 5 + 8 + 7 = **25** | 25 |
| 3 · Expresión e interacción escritas | 25 min | 2 | open | 25 |
| 4 · Expresión e interacción orales | 10 min (+10 prep) | 3 | open | 25 |

Pruebas 1 and 2 are administered as one combined booklet; the written session is
**95 minutes**. Listening audio plays **twice**. Multiple choice is always
**three** options. (S1 p. 6, S2, S3.)

---

## 3. The pass rule — scored in two groups, not as one percentage

This is the part most often got wrong, so it is stated exactly. (S1 p. 27, S4.)

> **Grupo 1** = Comprensión de lectura (25) + Expresión e interacción escritas (25) = **50**
> **Grupo 2** = Comprensión auditiva (25) + Expresión e interacción orales (25) = **50**

- Each group requires **≥ 30,00 points out of 50**, independently.
- The global result is **«Apto»** only if **both** groups reach 30. Otherwise
  **«No apto»**.
- **A total above 60 can still fail.** 40 in Grupo 1 and 25 in Grupo 2 is 65
  points overall and is `No apto`, because Grupo 2 missed its minimum. Any
  scoring that reports one 0–100 percentage has thrown the rule away.

> **Grouping trap.** The groups are **reading + writing** and
> **listening + speaking** — one receptive and one productive skill each. They
> are *not* "the two comprehension papers" versus "the two production papers".
> An automated read of S4 produced that second, wrong paraphrase during
> research; S1 and the Instituto Cervantes de Seúl page both give the
> reading+writing / listening+speaking split. S1 governs.

### Agreement with `task-shapes/a1.json`

`../../task-shapes/a1.json` was written independently, before this rubric, from
the same body of Cervantes sources. It **agrees with S1 on every checkable
point**: 45/25/25/10 minutes, 5+6+6+8 and 5+5+8+7 items, 25 points per skill,
95 written minutes, 10 minutes of oral preparation, two-replay audio, and — in
`passRule.note` — the reading+writing / listening+speaking group split with its
30/50 minimum. Its stimulus-length bounds (150–175, 175–210, 20–30, 160–185
words, etc.) match S1's published ranges. **No discrepancy was found, and these
mocks are built to it.** The two known wrinkles are recorded in §6.

---

## 4. Scoring

### 4.1 Pruebas 1 and 2 — objective

One point per correct answer, **no penalty for a wrong answer**, no partial
credit. The scaled score equals the raw item count: 20 correct → 20,00 points
(S1 §7). An omitted answer scores 0 and is never worse than a wrong one.

### 4.2 Pruebas 3 and 4 — rated on 0–3 bands

Each production tarea is rated on an ordinal band scale:

| band | meaning |
|---|---|
| **3** | comfortably above A1 |
| **2** | **meets the A1 (Acceso) descriptor** — this is the pass band |
| **1** | level not achieved |
| **0** | blank, off-target, ignores the prompts, irrelevant, or illegible |

The direct score has maximum 3 and converts by S1's formula:

```
final points = (direct score x 25) / maximum possible direct score
```

so a direct 2 yields **16,67** points and a direct 3 yields 25,00.

**Prueba 3 weighting** — the two tareas are weighted 50 % / 50 %.

**Prueba 4 weighting** — Tarea 1 = 20 %, Tarea 2 = 35 %, Tarea 3 = 45 %; within
each rating, *uso de la lengua* = 66 % and *cumplimiento de la tarea* = 34 %.
(The live exam averages a *calificador* at 60 % and an *entrevistador* at 40 %;
a single-rater mock collapses these, which is recorded as a deviation in §6.)

---

## 5. How the items were sampled — the anti-rigging rule

A mock written by looking at the corpus and asking "what can this book answer?"
measures nothing. The sampling rule was therefore fixed **before** any item was
drafted, and it draws on the syllabus, never on the book:

1. Every item realizes one or more **PCIC A1 inventory points** as enumerated in
   `../../../core/exam-inventory-es-a1.json` — the same 273 points the coverage
   report already measures. Items are indexed to those point IDs in the answer
   keys.
2. Topics are spread across the PCIC's **20 specific-notion areas** roughly
   evenly per mock, in the ámbitos *personal* and *público* that S1 assigns to
   each tarea. Areas the coverage report flags as unmapped are sampled at their
   natural rate — **neither avoided nor over-weighted**. Over-weighting them
   would manufacture a failure exactly as surely as avoiding them would
   manufacture a pass.
3. Item difficulty is set by the CEFR A1 descriptor and S1's published stimulus
   lengths, not by what the corpus happens to contain.

### 5.1 Every item declares what it requires

Each item in each answer key carries a **`requires:`** line naming the lexemes
and grammar points a candidate must hold to answer it. This is what makes the
sitting mechanical rather than a matter of the sitter's honesty: an item is
marked correct **only if every lexeme on its `requires:` line is in the set the
book teaches at or below A1**. The check is executed against the corpus, not
recalled from the sitter's own Spanish. See `sitting-2026-08-26.md` §2 for the
extraction and the audit.

---

## 6. Deviations from live administration, stated

A checked-in mock is not an exam centre. These are the gaps, so no reader
mistakes this for accreditation:

- **Listening is supplied as a transcript**, not audio. Nothing measures decoding
  of connected speech, speed, or accent. Scores here are therefore an
  **upper bound** on real listening performance.
- **Speaking is rated from a scripted candidate response**, with no live
  interlocutor, no unpredictability, and no *entrevistador*/*calificador* split.
- **One rater, not two.** S1 averages two independent raters on both production
  papers.
- **Tarea 1 word count.** S1 specifies 15–25 words for written Tarea 1, but the
  Modelo 0's own per-question guidance sums higher. These mocks follow S1.
- **Oral preparation scope.** S1 says the 10 minutes cover "las tres tareas";
  Modelo 0 says tareas 1 and 2. Both are official. These mocks follow Modelo 0,
  since Tarea 3 is unprepared interaction in either reading.
- Human validation with real learners (HL16) is **not** discharged by any of
  this.
