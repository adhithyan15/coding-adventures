## HL-C394 — an A1 core spine node is empty in ten of twenty-three tracks

Found while looking for Marathi's highest-value gap. It is not a Marathi gap.

`core/spine.d/0080-SPINE-TIME-OF-DAY.json` is `"core": true`, sits at stage A1
with only `SPINE-MEET-GREET` as a prerequisite, and carries nine concepts:

```
TIME-DAY  TIME-MORNING  TIME-AFTERNOON  TIME-EVENING  TIME-NIGHT
GREETING-DAY  GREETING-MORNING  GREETING-AFTERNOON  GREETING-EVENING
```

Every track has a `curriculum.d/spine/0080-SPINE-TIME-OF-DAY.json`. Measured
across all twenty-three:

| segments | omits | tracks |
|---|---|---|
| **0** | **9** | **bengali, gujarati, japanese, marathi, marwadi, persian, punjabi, russian, sanskrit, urdu** |
| 1–3 | 6–7 | chinese, italian, portuguese |
| 5 | 4 | french |
| 5–7 | 1 | kannada, latin, malayalam, telugu, tamil, hindi |
| 6 | 5 | arabic |
| 14 | 2–4 | german, spanish |

**Ten tracks realize none of it and omit all nine concepts.** Six of those ten
are languages the owner has said they most want to learn: bengali, gujarati,
marathi, marwadi, punjabi, urdu — plus sanskrit.

The healthy tracks converge on the same shape: five to seven path segments and
a single omit, `GREETING-DAY`. That is the target, and it is a solved problem
six times over, so the ten empty tracks are not blocked on design.

### Why it matters more than its size suggests

- It is **core**. Nothing else in the spine is marked core and left wholly
  unrealized in ten tracks.
- It is **early** — its only prerequisite is greeting somebody, so it is
  reachable almost immediately and its absence is a pre-A1/A1 ramp hole, not a
  tail-end one.
- The **listening paper is built on it.** Marathi's `MR-A1-NT-01` note says so
  directly: *"The listening paper's announcements are about times."*

### The measured instance

Marathi's entire `MR-A1-NT` column — *Temporal notions* — is **6 of 6 open**:
the clock and parts of the day, today/yesterday/tomorrow, the days of the week,
the months and writing a date, the seasons, and ordering two events. Verified in
the corpus rather than taken from the notes: across 347 lesson files, **आज
(today), सोमवार (Monday), वाजले (o'clock), सकाळ, संध्याकाळ and रात्र each return
zero**, and no lesson carries `spine_node: SPINE-TIME-OF-DAY` at all.

### One inventory note is wrong and should be corrected when this is worked

`MR-A1-NT-02` says *"udyaa occurs only inside fixed farewells."* It does not.
`MR-C38-kadhi` teaches **उद्या** as a standalone answer to *when?*, with its own
atom `MR-LEX-UDYA`, and `MR-R46-from` uses **उद्यापासून** productively with a
postposition.

What is true is sharper, and is the usual shape: **one third of the set was
handed over and the missing piece is named in passing without being taught.**
`MR-C04-udya-bhetu` writes *"**काल** (kāl, yesterday). No ambiguity about which
way time is pointing"* — so काल appears in the corpus exactly once, inside a
gloss explaining a different word, and आज never appears at all.

### Scope note for whoever picks this up

Do not try to close `NT-01` in one chapter. It is the clock **and** the parts of
the day, and the clock needs the numbers. Marathi has cardinals at least to
बारा, so the clock is reachable — but the parts of the day (five concepts) and
the time-appropriate greetings (four) are the natural first two chapters, in
that order, and they realize the node.

Marathi also has the greeting problem worth teaching honestly rather than
calquing: **नमस्कार** is taught in fourteen files and covers every hour, while
शुभ, सुप्रभात and दिवस return **zero**. A chapter that hands over four शुभ-forms
as if they were everyday speech would be teaching a register the reader will
rarely hear.
