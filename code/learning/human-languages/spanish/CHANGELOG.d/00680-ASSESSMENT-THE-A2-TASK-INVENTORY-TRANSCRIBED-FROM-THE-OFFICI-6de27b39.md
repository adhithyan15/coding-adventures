## Assessment: the A2 task inventory, transcribed from the official guide

`task-shapes/a2.json` is checked in, and Spanish's artifact ceiling falls again.
Like `a1.json` it **transcribes** DELE rather than approximating it: every
number below is read out of the *Guía del examen DELE A2, versión 2020*
(© 2019 Instituto Cervantes), fetched and parsed rather than recalled.

| prueba | time | tareas | items | inputs |
|---|---:|---:|---:|---|
| Comprensión de lectura | 60 min | 4 | 25 (5 · 8 · 6 · 6) | 250–300 · 50–80 each · 100–120 each · 375–425 words |
| Comprensión auditiva | 40 min | 4 | 25 (6 · 6 · 6 · 7) | 50–80 · 40–60 · 225–275 · 30–50 words |
| Expresión e interacción escritas | 45 min | 2 | — | candidate writes 60–70 then 70–80 words |
| Expresión e interacción orales | 12 min (+12 prep) | 3 | — | 2–3 min · 2–3 min · 3–4 min |

**The grouped pass rule is the counter-intuitive half, and it is pinned.** The
obvious guess — and what one secondary summary asserted while this file was
being written — is that DELE groups the two *receptive* skills together and the
two *productive* ones together. It does not. **Grupo 1 is lectura + escritas;
Grupo 2 is auditiva + orales.** Reading is paired with writing. A candidate who
reads well and writes badly can fail Grupo 1 outright while clearing 60 points
overall, so 60/100 alone is never sufficient. The secondary source was checked
against the official guide before anything was changed, and the repo's existing
A1 file turned out to have had it right all along.

Cervantes publishes **no** four independent per-skill thresholds, so the file
leaves them `null` rather than inventing them and attributing them to the
awarding body. The project contract adds its own.

**What declaring the inventory immediately revealed.** `reading-reach` can now
measure the A2 rung, and the first measurement is not flattering: the track's
longest comprehension passage is **61 words**, and only **one of the four
reading tareas** has a minimum input that short — **1/4 parts in reach**. That
is the honest state of an A2 rung whose reading content has not been written
yet, and it is now a number with a floor under it instead of a blank.

