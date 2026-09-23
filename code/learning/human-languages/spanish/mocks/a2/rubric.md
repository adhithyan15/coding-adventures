# DELE A2 mock rubric — Spanish

This rubric governs `a2-mock-1` and `a2-mock-2`, the two timed full mocks the
assessment contract (`../../assessment.json`, level `A2`) has always named and
never had. It transcribes the awarding body's real rule; it does not improve it.

---

## 1. The target, and where its shape comes from

The target is **DELE A2** as administered under the *v2020* renewal. Everything
in §2–§4 is transcribed from the sources already recorded in
`../../task-shapes/a2.json`, which is where this file takes its structure from
rather than re-deriving it.

| # | source | used for |
|---|---|---|
| S1 | [*Guía del examen DELE A2*, versión 2020, Instituto Cervantes (2019)](https://examenes.cervantes.es/sites/default/files/DELE_A2_v2020_Guia_de_examen.pdf) | structure table, task descriptions, item counts, group minimums |
| S2 | [Exámenes DELE A2](https://examenes.cervantes.es/es/dele/examenes/a2) | timings and points per prueba |
| S3 | [Cómo son las pruebas DELE](https://examenes.cervantes.es/es/node/265480) | the two-group pass rule |

Sources accessed 2026-09-14, as recorded in `task-shapes/a2.json`.

**No exam item or stimulus text is reproduced. Every item in both mocks is
original**, and all names, addresses, phone numbers and email addresses in them
are invented and use reserved domains that cannot resolve (RFC 2606 `.invalid`).
What is taken from these sources is the **structure** — paper names, timings,
item counts, task formats and scoring rules.

The same precision the A1 rubric makes applies here: the papers'
task-instruction lines (*Lea el texto y responda a las preguntas*, *Va a
escuchar seis conversaciones*) follow the standard published rubric phrasing
rather than being independently invented. That is functional boilerplate and is
named here rather than swept under a blanket claim of originality.

---

## 2. Structure

| Prueba | Duration | Tareas | Items | Points |
|---|---|---|---|---|
| 1 · Comprensión de lectura | 60 min | 4 | 5 + 8 + 6 + 6 = **25** | 25 |
| 2 · Comprensión auditiva | 40 min | 4 | 6 + 6 + 6 + 7 = **25** | 25 |
| 3 · Expresión e interacción escritas | 45 min | 2 | open | 25 |
| 4 · Expresión e interacción orales | 12 min (+12 prep) | 3 | open | 25 |

The written session is **145 minutes** (60 + 40 + 45), which is the figure
`task-shapes/a2.json` records under `administration.writtenMinutes`. Listening
audio plays **twice**. Multiple choice is always **three** options. Dictionaries
and phones are forbidden throughout.

**A2 is a step up from A1 in text length, not only in item count.** The longest
A1 reading stimulus is 175–210 words; A2's Tarea 4 is **375–425**, and Tarea 1
is 250–300 where A1's is 150–175. That is the single biggest difference a
candidate feels, and it is why the reading paper grows from 45 to 60 minutes.

---

## 3. The pass rule — scored in two groups, not as one percentage

Stated exactly, because it is the part most often got wrong. (S1, S3.)

> **Grupo 1** = Comprensión de lectura (25) + Expresión e interacción escritas (25) = **50**
> **Grupo 2** = Comprensión auditiva (25) + Expresión e interacción orales (25) = **50**
>
> A candidate must score **at least 30 of 50 in each group**, and must attempt
> every section.

Two consequences worth spelling out:

- **Reception is not paired with reception.** Reading goes with *writing*;
  listening goes with *speaking*. A candidate who reads well and writes badly
  can fail Grupo 1 while holding a comfortable total.
- **60/100 alone is not sufficient.** Sixty is the smallest possible passing
  total, but a 35/50 and a 25/50 is sixty points and a **fail**.

`task-shapes/a2.json` leaves `independentSkillThresholds` **null** for all four
skills, and deliberately: Instituto Cervantes publishes no per-skill threshold,
so none is invented here. The project assessment contract adds its own 0.6
per-skill threshold on top, which is a *project* rule and is labelled as one in
`../../assessment.json` — it is not attributed to the awarding body.

---

## 4. Scoring the objective pruebas

Pruebas 1 and 2 are scored against the answer key: **one point per correct
answer, no penalty for a wrong answer**. There are 25 items in each and one
point each, so the raw score is the scaled score.

---

## 5. Scoring the open pruebas

Pruebas 3 and 4 are not objectively keyed and are **not** covered by the
book-bounded audit (§6), which reads only `## Prueba 1` and `## Prueba 2` from
the answer keys. They are scored by a human against the descriptors in the
answer key's own rubric section.

---

## 6. The book-bounded audit, and what it is for

`book-bounded-audit.json` in this directory is generated, never hand-edited:

```
npm run generate:spanish-a2-mock-audit
npm run check:spanish-a2-mock-audit      # verifies the committed file is current
```

It reads the **last column** of each answer-key table row — the lexemes that
item requires — and checks each against everything the book teaches up to A2.
An entry prefixed with `!` is waived and must carry a reason in the key.

**The check asserts the audit is not stale; it does not assert zero failures.**
That is deliberate. Until the A2 vocabulary is taught, this file is the repo's
honest, machine-checked statement of exactly how far Spanish A2 is from
passable, and the number falls as the vocabulary tranches land. A2 was 817 of
its 1,200-headword target when these mocks were written.

This is also the mechanism that *chooses* the vocabulary. Words are not picked
by theme — a trial set of 35 thematically chosen A2 headwords collided 27/35
with words already taught, because at 817 headwords the obvious concrete
domains are saturated. The audit derives the list from the exam instead.
