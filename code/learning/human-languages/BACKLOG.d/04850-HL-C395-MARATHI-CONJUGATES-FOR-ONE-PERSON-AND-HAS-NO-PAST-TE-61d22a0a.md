## HL-C395 — Marathi conjugates for one person and has no past tense

Found while scoping `MR-A1-NT-02` (today / yesterday / tomorrow). That tranche
turned out to be **partly blocked by this**, and this is much larger.

### The verb column is 12 of 20 open, and it is the wrong 12

Open: `V-01` present habitual (all persons), `V-04` present continuous, `V-05`
past habitual, `V-06` simple past, `V-07` ergative `-ne`, `V-08` perfect, `V-09`
irregular past stems, `V-12` conjunctive participle, `V-13` past participle,
`V-17` future, `V-18` ability, `V-19` wanting and needing.

**There is no past tense and no future tense on this track at all.** Measured
across 360 lesson files rather than inferred: **आलो, गेलो and होतो each return
zero**, and no atom matching `MR-GRAMMAR-*PAST*` exists.

### The present tense is taught for one person

`V-01`'s note says it exactly, and the corpus agrees:

> *"Only the 1SG masculine and feminine forms carry atoms. A candidate can say
> what HE does and nothing about anyone else."*

| form | person | files |
|---|---|---|
| **येतो** | 1SG masc — **and 3SG masc, the same form** | 15 |
| **येते** | 1SG fem, 3SG fem | 16 |
| **येता** | 2PL / polite | 7 |
| **येतोस / येतेस** | 2SG | **0** |
| **येतात** | 3PL | **0** |

**The pronouns are not further along than the verbs.** A first pass at this
counted substrings and got that backwards — तो matched inside तोंड, ते inside
तेरा and आवडते. Counted as whole tokens across the same 360 files:

| pronoun | tokens | files |
|---|---|---|
| मी | 154 | 67 |
| तुम्ही | 37 | 18 |
| ते | 20 | 12 |
| ती | 14 | 9 |
| तू | 14 | 9 |
| तो | **8** | 6 |
| आम्ही | **0** | 0 |
| त्या | **0** | 0 |

And the third-person counts are softer than even those numbers: **तो appears in
a sentence exactly once**, as a table row `तो जातो` in `MR-C07-jane`, and most
of ते's twenty tokens are not the pronoun at all but the **range** word — *एक
ते पाच*, "one **to** five".

So a reader can address somebody with **तू** and cannot conjugate for them, and
has no third-person subject to conjugate for at all.

### Why this outranks the temporal nouns

`MR-A1-NT-02` wants **आज** (zero) and **काल** (one occurrence, inside a
parenthetical gloss explaining a different word). आज is fine — *मी आज येतो*
works with the present. **काल is not**: a word for *yesterday* on a track with
no past tense is a time word with no verb to use it with, which is the ramp
inverted in the same way `HL-C393` described for Malayalam's unprinted future.

Teach the paradigm first, then the past, and the temporal nouns land on
something.

### The past has a gate, and the note already names it

`V-07` is explicit, and it should be read before anyone scopes `V-06`:

> *"It is not a refinement of the past tense but the GATE on it: a Marathi past
> transitive clause without it is ungrammatical, so `MR-A1-V-06` cannot be
> closed independently of this point."*

Marathi is **ergative in the past**. That is not a chapter to bolt onto a
tranche; it is its own unit, and `V-06`, `V-07` and `V-09` (irregular stems
*gelaa*, *khaallaa*, *kelaa*) are one piece of work.

### Suggested order

1. **`MR-A1-DEM` first — it is the gate on the verb work.** The column is
   **3 of 3 open** (`DEM-01` the forms हा/ही/हे and तो/ती/ते in three genders,
   `DEM-02` the two-way near/far against Spanish's three-way, `DEM-03`
   prenominal position). Those six words are Marathi's third-person pronouns as
   well as its demonstratives, so `V-01` cannot teach a third-person cell
   without them. One chapter can close all three points.
2. **`V-01`, the present for all persons** — once there are subjects. It is two
   endings and one missing pronoun: **-स** for तू (येतोस / येतेस), **येतात**
   for ते, and आम्ही with its 1PL form. Everything conversational is downstream
   of it.
3. **`V-04`**, the present continuous. Its note points out *"what are you
   doing?"* is an A1 interview question, and it needs the paradigm first.
4. **`V-06` + `V-07` + `V-09` together**, the past with its ergative gate. The
   A1 listening paper's third part is *"a personal account"*, which the note
   observes is past-tense by genre.
5. Then `MR-A1-NT-02`, which now has tenses to sit in.

### The NT-02 note is also wrong and should be corrected when that lands

It says *"udyaa occurs only inside fixed farewells."* उद्या **arrives** inside
one — `MR-C04-udya-bhetu`, whose `concept_tag` is `FAREWELL-TOMORROW` — but it
is taught there as a word in its own right with its own atom `MR-LEX-UDYA`, and
`MR-C38-kadhi` answers *when?* with it **standing alone** while `MR-R46-from`
uses **उद्यापासून** with a postposition. Arriving in a farewell is not the same
as being confined to one.
