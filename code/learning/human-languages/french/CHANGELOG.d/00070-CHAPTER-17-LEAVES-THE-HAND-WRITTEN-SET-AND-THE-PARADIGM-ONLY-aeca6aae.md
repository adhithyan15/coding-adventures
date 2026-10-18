## Chapter 17 leaves the hand-written set, and the paradigm only the tables showed

Chapter 17 (To Have, and How Old You Are) was hand-written LaTeX over **two**
schema-v1 lessons. Counting words would have said this chapter was nearly done.
It was not, and the reason is that **its cost is a paradigm**.

### The fourth reading is the one that found it

The `.tex`'s opening table has **one row per person** — *j'ai, tu as, il a, nous
avons, vous avez, ils ont*. That is **six grammar cells**, and
`maxNewGrammarCellsPerLesson` is **1**. Two lessons held all six, plus the age
idiom on top.

Counting blocks gave 8. Counting words gave four or five. Counting the lessons
that own the chapter gave 2. Only reading the table gave the real answer:
**fourteen lessons**, one thing each.

| lesson | the one new thing |
|---|---|
| `FR-C14-jai` … `FR-C14-ont` | the six forms, one per lesson |
| `FR-C14-ai-as-a` | *ai*, *as*, *a* are homophones — **why French keeps its subject pronouns** |
| `FR-C14-habere` | the root under *avoir* **and** *habiter* |
| `FR-C14-practice-avoir` | the six together, as a recap rather than an introduction |
| `FR-C14-an` | *un an*, and *annual*, *anniversary*, *annals* |
| `FR-C14-jai-vingt-ans` | age is something you **have**; *ans* is compulsory |
| `FR-C14-quel-age` | *Quel âge as-tu ?* |
| `FR-C14-avoir-etre-langues` | the Romance/Germanic split |
| `FR-C14-practice` | the chapter payoff |

Eleven atoms, under the ceiling of twelve.

### The fact the homophone lesson made sayable

*ai*, *as* and *a* are three spellings of one sound. Given its own lesson, that
stops being a pronunciation footnote and becomes an explanation: **French cannot
drop its subject pronouns and Spanish can**, because the endings that would carry
*who* went silent in one language and not the other. That is one of the most
useful facts in beginner French and it was a `sounds` block.

### The chapter's two halves live at different CEFR levels

`FR-PATH-A1-AVOIR` is A1 and `FR-PATH-016` is A2, with **different spine nodes**
— `SPINE-SAY-WHAT-I-HAVE-AND-CAN-DO` and `SPINE-SAY-WHAT-I-DO`. Schema-v1 lessons
declare no `spine_node` so nothing checked this before; schema-v2 lessons must
match their placement, so the nine verb lessons and the five age lessons declare
different nodes. The A1 path had no extension node to hold its support lessons,
so one was appended (**appended**, not inserted — extension shard filenames are
positional, and an insertion renames every following shard).

`VERB-HAVE` is a **canonical** concept and exactly one lesson may realize it.
`FR-C14-jai` carries it; the other thirteen use namespaced tags. Without that the
omission ledger would have recorded French as no longer teaching "have."

### A ratchet that held

`info-dump`'s rule-statement ceiling is **30**, and this work first pushed it to
31. The offender was mine: `FR-C14-avoir-etre-langues` said *"there are two ways
to think about age"*, which is the *there-are-n-kinds* shape the gate exists to
catch. Rewritten as an observation — *"French treats age as something you carry;
English treats it as something you are"* — the count returns to 30. The ceiling
was **not** reseated, and the prose is better for it.

