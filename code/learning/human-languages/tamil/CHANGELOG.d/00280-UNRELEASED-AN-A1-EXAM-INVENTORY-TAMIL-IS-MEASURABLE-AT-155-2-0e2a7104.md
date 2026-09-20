## Unreleased — an A1 exam inventory: Tamil is measurable, at 155/262 (59%)

`core/exam-inventory-tamil-a1.json` enumerates **262 things an A1 Tamil
candidate must be able to handle**, each probed by the knowledge atoms whose
presence would demonstrate the track teaches it. Tamil is the seventh track with
one and the first Dravidian track measured against a derived point set rather
than against itself.

**The number: Tamil covers 155 of 262 points (59%).** Zero points are partly
taught — every probe names atoms that actually exist, checked mechanically
against the 442 atoms the 359 lesson files introduce — so the shortfall is 107
points with nothing in the corpus corresponding to them. By category:

     33/49  Toḍarbāḍal (communicative functions)
     38/64  Solvaḷam (core lexis)
     20/26  Vinaiccol (the verb)
      9/14  Tamiḻ eḻuttu (script and orthography)
      6/7   Iruvaḻakku (the two registers)
      6/11  Vināccol (asking questions)
      6/9   Vinaiyaḍai (adverbs)
      5/8   Kālam maṟṟum tēdi (time and date)
      5/7   Oli (pronunciation)
      4/9   Pratippeyar (pronouns)
      3/8   Vēṟṟumai (case endings and postpositions)
      0/7   Iṇaittoḍar (joining clauses)

**The shape is the finding.** Tamil greets, introduces, takes its leave, names a
great deal of the physical world, and runs one genuinely well-taught grammar
system — chapter 32's three-slot verb and chapter 6's dative subject are the
best pieces of teaching in the track. What it cannot do is join two clauses:
there is no `-um … -um` for and, no `ānāl` for but, no `alladu` for or, and no
quotative `eṉṟu`, so the verb column and the lexis column can never become a
sentence. It also cannot ask a yes-or-no question, because the `-ā` suffix that
forms one is untaught, which means the learner can ask the five questions the
course supplies and not one of their own.

**Two cheap points and one expensive one**, named because the notes are meant to
be an authoring brief rather than a scoreboard:

- `vilai` ('a price') and `evvaḷavu` ('how much') are BOTH taught, in different
  chapters, and nothing joins them into a question. Asking a price is one lesson
  away.
- The Tamil digits ௧–௰ are taught across eight lessons in chapter 7 and every
  one of them declares an empty `introduces` list. The material exists and
  cannot be probed; one frontmatter field per lesson closes two points.
- No numeral above twenty exists. That single gap blocks stating a price, giving
  an adult age, saying a year, and reading a telephone number.

**The script column is nearly closed, and that is unusual.** Walking every
character in the Tamil block of Unicode that any lesson prints against the set
its script lessons teach returns **52 shown, 52 taught, zero untaught** — all
eighteen core consonants and ten of the twelve independent vowels. No other
Indic track in this repository is closed. The gaps left are the grantha letters
(ஜ and ஹ), ஈ and ஔ, and punctuation.

**Nothing is attributed to anybody.** `core/exam-levels.json` records Tamil as
having no single international ladder and names Singapore MOE Tamil and the
Tamil Nadu state syllabi as the nearest things; the file disclaims both by name
in its first sentence, and disclaims DELE, the Instituto Cervantes and the Plan
Curricular in the same breath. The point set is derived structurally from the
project's restatement of the DELE/PCIC A1 inventory, used as a **proxy for
level** — asked what an A1 learner must handle, never what Spanish grammar it
names. All 273 proxy points are accounted for: 270 derived, 3 named as
untransferable with a reason. Per BACKLOG HL-C287 and HL-C290 no fresh external
search was run, and the file says so rather than implying one happened.

**Tamil's diglossia forced a column the proxy could not supply.** `exam-levels`
carries a caveat for this track and for no other Dravidian one: the literary and
spoken registers diverge sharply and this curriculum teaches spoken Tamil first.
So the inventory has an `Iruvaḻakku` column, six of whose seven points ARE
covered — the corpus is unusually good at register, teaching the `-uṅgaḷ`
politeness ending, the `nī / nīṅgaḷ` choice, the two grammars of the age
question and the native-against-Sanskrit axis. The one uncovered point is the
biggest: no lesson ever tells the learner that Tamil is diglossic at all.

`npm run plan` now names Tamil exam-point work where it previously reported
Tamil among the tracks that could not be measured, and the corpus-wide
uncovered-point total moves 686 → 793 across 8 written inventories — re-measured
on the merged tree, not derived, because the Telugu inventory landed on main
while this one was being written and both moved the same counter. Tamil is the
seventh TRACK to have one and the eighth inventory, since German has both an A1
and an A2. The count of tracks that cannot be measured at all falls 17 → 16.
Nothing regressed: the backlog grew by exactly the debt that was previously
invisible.

Logged as HL-C303.

