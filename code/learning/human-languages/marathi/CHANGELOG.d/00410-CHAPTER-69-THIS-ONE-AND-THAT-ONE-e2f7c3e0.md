## Chapter 69 — this one and that one

All three `MR-A1-DEM` points close: `DEM-01` (the six forms in three genders),
`DEM-02` (two-way near/far against Spanish's three-way) and `DEM-03`
(prenominal). Marathi **168/301 → 171/301**.

**"Demonstratives" leaves the empty-column list** in `tests/exam-inventory`,
which is the movement rather than the number.

| metric | before → after |
|---|---|
| exam-point coverage | 168/301 → **171/301** |
| lessons | 360 → 364 |
| atoms taught | 344 → 349 |
| `forwardReferences` | 4 → 10 — **read the note below** |

### It was worked before the verb column, and that was the point

`HL-C395` found that Marathi conjugates for one person. The obvious next unit
was `MR-A1-V-01`, *the present habitual, all persons* — and it is **blocked**,
because **तो / ती / ते are Marathi's third-person pronouns as well as its
demonstratives**. Counted as *tokens* rather than substrings, तो appeared in a
sentence exactly once and most of ते's occurrences were the **range** word
(*एक ते पाच*, one **to** five). There was no third person to conjugate for.

This chapter supplies it, so `V-01` is now reachable.

### Two fronts and three endings make six words

| | masculine | feminine |
|---|---|---|
| **near** | **हा** | **ही** |
| **far** | **तो** | **ती** |

The neuter completes each row — **हे**, **ते**. The front carries the distance
and is the speaker's; the ending carries gender and is the **noun's**. Neither
decision touches the other, which is why six words cost four.

Taught on **मित्र**, **खोली** and **घर**, whose genders the corpus *states*
(`MR-C41-kholi`: "ghar is neuter"; "खोली is feminine"; माझा मित्र gives the
masculine). **चहा was dropped from a draft** because its gender is stated
nowhere.

### The homograph is flagged on the page

The reader has already met **ते** — as *"to"* in **एक ते पाच**. Nothing in the
shape tells them apart; what does is the neighbours. The lesson says so rather
than letting it ambush them later.

### On `forwardReferences` 4 → 10

Not a regression to hide. **The metric counts uses of material a *later* lesson
teaches — a word that is never taught is invisible to it.** Giving these six
words an owner converted *invisible* debt into *visible* debt.

Five of the six are **metalinguistic**: `MR-C01-yeto` writes ये + तो → *yeto* to
decompose a syllable; `MR-C07-yene` and `MR-C07-asne` name हे as the stroke
inside आहे. No placement fixes those — they are in chapters 1 and 7.

**One is genuine and pre-existing**: `MR-C37-kon` asks **तो कोण आहे?** — *who is
he?* — with a pronoun the track never taught. Fixing it means relocating the
demonstratives before sequence 1510, and घर and खोली sit at 1640 and 1650, so
that is a deliberate restructure rather than a rider on this tranche. Recorded
in `HL-C395`.

