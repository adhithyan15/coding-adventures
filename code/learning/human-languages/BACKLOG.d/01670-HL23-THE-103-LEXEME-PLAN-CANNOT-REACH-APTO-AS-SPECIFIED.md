## The 103-lexeme authoring plan cannot reach APTO as specified

**DECIDED — both, and recorded as `HL23` §12.** An A1 rung for qualities will be built,
justified by the unmapped inventory points `A1-NG6-03`, `A1-NG6-08` and `A1-NG6-10` rather than
by convenience; and the confusability screen disambiguates a required candidate rather than
dropping it. This entry is kept for the measurements that forced the decisions. The rung itself
is tranche B/C work.

`HL23` §11 measured that the ~103 authored lexemes close the gap to `APTO`, and §11.5 ends
"the bundle table says they close it exactly." The bundle proof at the head of the first
authoring tranche reproduces that result **and then finds two conditions on it that the
table does not state.** Both are owner decisions. Neither is a tooling problem.

Measured on `origin/main` @ `7cab28d912`, with the calibrated harness, ceiling A1:

| granted | mock 1 G1 | mock 2 G1 | |
|---|---|---|---|
| nothing (baseline) | 4,00 → **12,33** | 5,17 → **7,17** | as §11.5 published |
| all 103 | **31,33** | **32,33** | **APTO** both |
| 103 minus the 10 that fail the confusability screen | 30,33 | **28,33** | **NO APTO** |
| 103 minus the pure adjectives | 30,33 | **26,33** | **NO APTO** |
| 103 minus both | 29,33 | 22,33 | NO APTO |

So `APTO` requires **both** the adjectives **and** most of the confusability-flagged words.
The plan as written assumes all 103 are authorable. Ten are not, on the project's own rules.

### 1. The adjectives have no honest A1 rung, and they are load-bearing

`barato`, `gratis`, `mayor`, `importante`, `favorito` (and `menor`, `caro`, `recto`) are
adjectives. The A1 spine has `SPINE-NAME-EVERYDAY-THINGS` and
`SPINE-NAME-EVERYDAY-ACTIONS` and nothing that hosts a quality. §11.4 already refused to
mint an adjectives rung, and gave the right reason: it declined to widen a `canDo` to hold
three namespaced lessons.

What is new is the **price of that refusal, now measured**. Of the fourteen mock-2 reading
items still failing after a full noun-and-verb tranche, **seven are blocked by an adjective
and nothing else**: `caro` (items 2 and 3), `barato` (8), `importante` (10), `mayor` (15
and 22), `gratis` (6), `adulto` (11). Mock 2's Grupo 1 plateaus at **20,33** against a bar
of 30,00 no matter how many nouns and verbs are authored — a greedy search over the whole
clean noun/verb pool cannot beat it.

Tranche 6 filed its adjectives under `SPINE-COUNT-ONE-TO-FIVE` ("the cardinal numbers one
through five"), which is the standing fiction `01360` already records. That option is
available and should not be taken.

The decision is therefore: **an A1 rung stating one honest capability about qualities**
— something on the order of *"I can say what something is like"* — or **`NO APTO` on
mock 2, permanently.** The rung is a 23-track realization ledger, the same shape as
`SPINE-SAY-WHAT-I-HAVE-AND-CAN-DO` in §10. It is now justified by measurement rather than
by needing somewhere to put a word, which is the test §10.1 set.

### 2. The confusability screen has no substitution budget left

Tranche 6 derived the rule — *a same-length pair differing in one position is a drop only
when the differing position is not the first* — and could afford it, because it screened
roughly a hundred candidates to place thirty-five. **The exam-driven list has no such
surplus: every one of the 103 is there because a measured item requires it.**

Ten of the 103 flag against a taught headword form:

| candidate | collides with | position |
|---|---|---|
| `costar` | `contar` | 2 |
| `caro` | `cero`, `cara` | 1, 3 |
| `tren` | `tres` | 3 |
| `gorro` | `gordo` | 3 |
| `mayo` | `mano`, `malo` | 2 |
| `menor` | `menos` | 4 |
| `playa` | `plaza` | 3 |
| `pollo` | `polvo` | 3 |
| `recto` | `resto` | 2 |
| `amigo` | `amiga` | 4 |

Dropping a word here is not free the way it was in tranche 6; it forfeits the item. And
several are core A1 vocabulary that a Spanish course cannot honestly omit — refusing to
teach `tren` because `tres` exists is not a defensible curriculum.

The decision is whether the screen's verdict is **drop** or **disambiguate**. A third
option exists and may be the right one: teach the word and contrast the neighbour
explicitly in the prose, which is what a good course does with a minimal pair anyway. That
would make the screen a routing rule rather than a veto, and it should be written down
either way, because at present the rule reads as an unconditional drop and the arithmetic
says an unconditional drop fails.

### 3. `amigo` is a fourth already-owned trap, of a fifth kind

§11 and the sitting screen against headwords, atom ids and the root ledger, and the
`dar` / `llover` cases established that a headword-only screen is not enough. `amigo`
escapes all three: it is owned by **`ES-C09-falsos-amigos`**, whose headword is the
two-word term of art *falsos amigos*, at **A2**. Only a screen that decomposes multiword
headwords into their component words finds it.

It is also the case where "already owned" is arguably wrong: the corpus teaches the
metalinguistic term *false friends*, not the word *friend*. So `amigo` is simultaneously a
duplicate by the ledger and a genuine gap by the syllabus, and which it is depends on a
judgement no ledger records.

### The general form

**A screen calibrated on a surplus of candidates changes meaning when the candidate list
becomes exactly the requirement.** Tranche 6's confusability rule was a tie-breaker among
interchangeable options; applied to an exam-derived list where every entry is load-bearing,
the identical rule becomes a veto on passing. The rule did not change. What changed is that
there is no longer anywhere to substitute from — and nothing in the rule's statement says
it depended on that.
