## HL23 — the ability rung, and the finding that verbs and nouns block the same items

`SPINE-SAY-WHAT-I-HAVE-AND-CAN-DO` (A1, FUNCTION) now carries `VERB-HAVE` and
`VERB-CAN`, and chapter 389 authors `saber` so that PCIC `A1-F2-16` and
`A1-F2-17` can be mapped honestly rather than by pointing them at `poder`, which
the inventory's own notes had already refused. Spanish goes 621 → 624 headwords
and 44 → 47 verbs at or below A1; pre-A1 stays at 304.

Four things this leaves behind.

**Verbs and nouns are multiplicative on the same items, and that changes the
sequencing.** Releasing `tener` (the most-missed verb, 6 objective items) and
`poder` (3) moved **Grupo 1 by zero on both mocks** and bought one item in
*auditiva*. Every reading item wanting one of them wants a noun too: mock 1 #2
also wants `terraza` and `habitación`, #13 also wants `aeropuerto` and
`necesitar`, #23 also wants `ordenador` and `internet`. The remaining verb
backlog — `gustar` (9 blocked), `querer` (4), `preferir` (4), `porque` (4) —
should therefore ship **inside** the noun tranches rather than as a slice of its
own, because a verb-only slice will keep returning a single-figure Grupo 1.

**HL23 §9.1 over-prices `VERB-HAVE`, and the reason generalises.** It predicted 3
lesson migrations, two `misplaced-shared-realization` repairs, and an emptied
`GE-PATH-018`. None of the last two happened: a concept can move without its
lesson moving if the lesson's whole **segment** is retargeted, because level
derives from `segment.spine_node` and a segment's path position is independent of
its node's stage. `ES-PATH-030-HACER-CH12` already demonstrated this at path
index 61. **Any future concept release should first ask whether the segment can
move instead of the lesson** — it is one line, and it cannot break
`curriculum-prerequisite-order` because nothing moves.

**The orthography points are the only exam entries vocabulary cannot buy.**
`!mayúsculas-en-nombres-propios`, `!arroba-y-punto-en-una-dirección-de-correo`
and `!punto-final` each block two production tareas, and they are exactly what
stands between the written paper's band 1 and band 2. Granting all the missing
vocabulary and nothing else leaves *escritas* at 8,33 on both mocks; adding the
orthography points lifts it to 16,67. `phonology-orthography` is still `partial`
and still an owner decision.

**The scoring harness is still not committed code.** It has now been rebuilt
twice from `sitting-2026-08-26.md` §8's prose description, and the second rebuild
disagrees with the first by one objective item on mock 1 *lectura* — the second
reads 4 where §9.4 read 5. Five mock-1 items are blocked by one lexeme apiece
(`universidad`, `médico`, `barato`, `llevar`, `actividad`) and none of the five
exists in the corpus at any level, so the disagreement could not be resolved
without inventing evidence and was left standing in the strict direction. **A
third rebuild will disagree again.** The harness should be checked in beside the
mocks, with its calibration targets as tests, or the mock scores will keep being
measurements on an instrument nobody can reproduce.
