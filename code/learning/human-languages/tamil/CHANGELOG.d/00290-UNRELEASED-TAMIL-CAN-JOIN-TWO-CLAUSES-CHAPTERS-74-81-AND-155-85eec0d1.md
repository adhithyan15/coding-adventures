## Unreleased — Tamil can join two clauses: chapters 74-81, and 155/262 -> 174/262 (66%)

`core/exam-inventory-tamil-a1.json` reported `Iṇaittoḍar (joining clauses)` at
**0 of 7** and named the consequence in its own words: a well-taught verb column
and a well-taught lexis column cannot become a sentence. Thirty-one lessons
across eight chapters close that column and eleven other points beside it.

    tamil A1 exam coverage       155/262 (59%) -> 174/262 (66%)
    Iṇaittoḍar (joining clauses)     0/7  ->  7/7
    Vināccol (asking questions)      6/11 ->  9/11
    Iruvaḻakku (the two registers)   6/7  ->  7/7
    Eṇṇuppeyar (numerals)            4/8  ->  5/8
    Tamiḻ eḻuttu (script)            9/14 -> 10/14
    Toḍarbāḍal (functions)          33/49 -> 37/49
    Etirmaṟai (negation)             2/5  ->  3/5
    Vinaiyaḍai (adverbs)             6/9  ->  7/9
    tamil lessons                      359 -> 390
    tamil chapters                      73 -> 81
    atoms introduced                   442 -> 477
    reinforcement window misses       1200 -> 1200  (held)
    forward references                   7 -> 7     (held)
    taught glyphs / shown glyphs     52/52 -> 52/52 (held)
    script-closure violations            0 -> 0     (held)
    measurement-blind lessons            6 -> 2
    corpus exam-point backlog          793 -> 774

Every figure was re-measured against the merged tree, not derived by arithmetic.

### The nineteen points, and the eight chapters that close them

| chapter | node | what it gives |
|---|---|---|
| 74 And, and Also | SPINE-RESPOND-BASIC | `-um … -um`, additive `-um`, `-um … -um illai` |
| 75 Yes or No | SPINE-NEGATE-AND-ASK | the polar `-ā`, answering it, `varalāmā?` |
| 76 But, and Or | SPINE-SAY-WHAT-I-WANT | `āṉāl`, `alladu`, offering a choice |
| 77 Saying, and Thinking | SPINE-SAY-WHAT-I-DO | `sol`, the quotative `eṉṟu`, an opinion |
| 78 Why, and Because | SPINE-SAY-WHY | `ēṉ`, `ēṉeṉṟāl`, `adaṉāl`, `-kkāga` |
| 79 When | SPINE-TIME-OF-DAY | `eppōdu`, `appōdu`, `-um pōdu` |
| 80 One, the Other, and the Price | SPINE-DEFINITE-REFERENCE | `maṟṟadu`, `oṉṟu … maṟṟadu`, `vilai evvaḷavu?` |
| 81 Two Tamils | SPINE-COURTESY-THANK | the diglossia, and which form this book prints |

Closed: **CJ-01** through **CJ-07** (the whole column), **Q-01** (the polar
suffix, which the inventory called the most consequential single gap in the
grammar column), **Q-08** `eppōdu`, **Q-09** `ēṉ`/`ēṉeṉṟāl`, **ADV-08** the
additive `-um`, **NEG-05** negative coordination, **F-27** offering
alternatives, **F-32** asking and granting permission, **F-39** asking a price,
**F-44** giving an opinion, **NUM-08** and **L-10** the Tamil digits, and
**REG-01**, which the file called the single most Tamil-specific point in it.

### The digits cost no content at all

`TA-A1-NUM-08` said it in as many words: eight chapter-7 lessons teach ௧–௰ and
every one of them declared an empty `introduces` list, so the material existed
and could not be probed. Four atoms — one per teaching lesson — now make it
measurable, and the copy, dictation and practice lessons declare that they
practise them. Not a line of content was written. `TA-A1-L-10` closed with them.

The one cost is recorded rather than hidden: chapter 7 was already sitting
exactly on `maxNewAtomsPerChapter` at 12, so four honest atoms push it to 16 and
`atomChapterSpikes` moves 0 -> 1. The fix is a chapter split, which renumbers
every chapter after it and every prose cross-reference in the track, so it is
filed as HL-C305 rather than smuggled into a content tranche.

### Register: what is taught, and what is only named

The curriculum teaches **spoken** Tamil, and `core/exam-levels.json` carries a
diglossia caveat for Tamil and no other Dravidian track. Joining words are
exactly where the two registers separate, so every joiner here carries both
forms: the standard one as the headword, because it is what a sign and a
dictionary use and because it PREDICTS the clip, and the everyday clip named in
the body — `āṉāl`/`āṉā`, `ēṉeṉṟāl`/`ēṉṉā`, `eṉṟu`/`-ṉṉu`. Chapter 81 then says
the whole thing out loud, which is what closes REG-01. That direction is the
argument: from `eṉṟu` you can predict `-ṉṉu`; from `-ṉṉu` you could not have
rebuilt `eṉṟu`.

### Retrieval at three distances, and why the append cost nothing

The two-back rollback from chapters 67-73 holds R1 and nothing else: a chain of
adjacent lessons revisits an atom at distance one and two and never again. A
naive append would have taken the reinforcement finding from 1200 to **1300** —
+37 on the tranche's own atoms, +8 on the four numeral atoms, and +55 on
PRE-EXISTING atoms whose R3 and R4 windows only became judgeable because the
track grew from 359 lessons to 390.

Every lesson therefore ends with a second retrieval line at three distances:

    - [YOU RETURN TO: say *nēṟṟu*, *kuṭai* and *tarai* — three distances back
      — then join two of them with -உம்]

One item from a few lessons back, one from around chapter 70, one from around
chapter 62 — and the task is always this chapter's own pattern, so the joiners
are practised on vocabulary the reader already owns, which is the entire point
of teaching them. Ninety-two window misses were placed that way and the finding
came back to **1200 exactly**, with thirty-one lessons on top of it. The two
distant numeral retrievals went into `TA-C18-mani` and `TA-C39-evvalavu`, which
are the lessons where a numeral is genuinely what the answer is made of.

### What was refused

* **A verb-purpose clause.** `-kkāga` is taught on a NOUN — `uṅgaḷukkāga`. The
  verbal-noun form the full purpose clause needs is not in the track, and
  `TA-A1-CJ-05`'s note now says so rather than implying the point is wholly
  closed.
* **`alla` against `illai`** (`TA-A1-NEG-02`). `alladu` is taken apart as
  "it is not that", which puts `alla` in front of the reader, but teaching the
  two-way negative properly is a lesson of its own and is not in this tranche.
* **A past or future tense.** `varum` appears inside `varum pōdu` and is
  explained as the verb's own ending, not as a tense. `TA-A1-V-06` and
  `TA-A1-V-07` stay uncovered, honestly.

