## Nine A1 points were taught and unprobed: 33/74 → 42/74, and Lexique de base closes

`core/exam-inventory-french-a1.json` had **41 points with `probe: null`**, and 32
of them carried no note. `exam-inventory.ts` documents that value as "no atom in
the corpus corresponds to this point, which is a finding rather than a gap in the
data", so all 41 scored uncovered and the track read **33/74 (45%)**.

**Nine of the 41 are taught in full.** They were never findings; they were points
nobody had written a probe for, and the fourteen generated chapters that closed
them landed after the inventory was authored.

| point | what already teaches it |
|---|---|
| A1-V-01 present of *être* | chapter 19 — **one lesson per person**, all six |
| A1-V-02 present of *avoir* | chapter 17 does the same, all six |
| A1-V-14 *passé composé* with *avoir* | chapter 18, the frame plus both participle families |
| A1-V-15 *passé composé* with *être* | chapter 20, including `FR-C16-accord-unifie` |
| A1-P-01 subject pronouns | six pronouns across six lessons, plus the rule that makes them obligatory |
| A1-N-02 gender is unpredictable | `FR-C01-le-la`: "the gender can't be guessed" |
| A1-A-03 adjectives follow the noun | `FR-C13-vin-rouge`, which states the rule *and* its exception |
| A1-PRON-05 the cedilla | `FR-W02-cedille` |
| A1-LEX-08 weather | `FR-C21-le-temps` — *le temps*, *il fait chaud*, *il pleut* |

### The paradigms could not have been one atom

`maxNewGrammarCellsPerLesson` is **1**. That is why chapter 19 teaches *être* one
form per lesson, and it means a French verb paradigm **cannot** be a single atom
— *être* is six, *avoir* is six. An inventory written by looking for
`FR-VERB-ETRE-PRESENT` finds nothing and writes `null`, and the corpus then reads
as missing exactly the material it is most careful about.

The two failures compound: the gate that forces the gentle shape is the same gate
that makes the gentle shape invisible to a probe.

### Lexique de base is now 10/10, and that is the finding

The vocabulary column has run out of room. **Every remaining uncovered point in
this inventory is grammar or a function word** — the four prepositions points, the
four sentence points, three of four adverb points, six of eight determiners. No
quantity of headwords moves any of them.

### What the other 32 now say

Each carries a note naming what the track holds. They are specific rather than
"untaught":

- **A1-D-04** — *du* and *de l'* are owned with the contraction that produces
  them; *de la* and *des* are not shown, so two of the point's four forms exist.
- **A1-P-03** — *et toi ?* appears in **two practice dialogues**, glossed as a
  phrase. The tonic pronouns are in the reader's mouth and not in the corpus.
- **A1-ADV-02** — *à demain* and *à bientôt* are owned as farewells, so *demain*
  is likewise available only inside a fixed phrase.
- **A1-A-04** — `FR-C13-vin-rouge` names the pre-posed class and gives *bon* as
  its only member: "a few others you will meet." **Five of the six are unwritten.**
- **A1-PREP-01** — the largest single gap: *à*, *de* and their four contractions
  sit under the place, country and time points as well, so one lesson set unblocks
  a whole column.

### Both halves falsified

Adding `FR-LEX-FABRICATED-99` to A1-LEX-08 fails the "probes only atoms that
EXIST" test. Nulling A1-V-15 drops the total to 41 and fails the pin, *and* fails
the new note gate — French is now held to the rule Marathi has had since
HL-C290: an unmapped point must say why.

