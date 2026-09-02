## HL-C303 — the second Dravidian inventory, and what a diglossic language does to a proxy built from Spanish

`core/exam-inventory-tamil-a1.json` is the eighth written inventory and the
fourth derived from the Spanish proxy, after Hindi, Marathi and Telugu. HL-C290's method was followed rather than reinvented,
and the useful finding is what it cost: **the derivation took a fraction of the
time the first one did, and the two places it did not fit were both properties
of Tamil rather than of the method.**

### What transferred without argument

262 Tamil points from 273 Spanish ones. 270 of the source points derive into a
target point and 3 are dropped with a reason — superscript letters in
abbreviations, and the two written-accent points, none of which Tamil writes.
The lexical domains transferred best, exactly as HL-C290 predicted: education,
work, media, services, documents, the arts, government and religion are all
domains an editorially-chosen Tamil list would never have thought of, and Tamil
holds almost none of them.

**Collapse is normal and should be recorded, not resisted.** Spanish's four
article points collapse into two Tamil ones, because the interesting fact is
that Tamil HAS no article and the corpus never says so. Its nine punctuation
points collapse into one, because modern Tamil uses the same marks as English.
Its eight past-tense points collapse into a single uncovered point, because
Tamil's past is one slot in one machine and the corpus fills that slot only with
the present. A collapse is a finding as long as `derivedFrom` lists every source
point that went into it, which is what makes the totality test meaningful.

### The first place the proxy did not fit: diglossia

`core/exam-levels.json` carries a caveat for Tamil that it carries for no other
Dravidian track — the literary and spoken registers diverge sharply, so no CEFR
mapping is clean, and this curriculum teaches SPOKEN Tamil first. **A proxy
built from a language without that split cannot generate a single point about
it.** So the file has a register column that no proxy-derived inventory has had,
and its first point is that the corpus never tells the learner any of this. A
reader who finishes 359 lessons and opens a Tamil newspaper will not recognise
the verb endings.

The general rule: read the track's `exam-levels.json` caveat BEFORE walking the
proxy, and treat whatever it names as a category the proxy cannot supply. Tamil's
caveat is the longest in that file and it was worth every line.

### The second: the script column came back nearly CLOSED

Marathi's and Hindi's script columns are the debt columns. Tamil's is the
opposite, and it was measured rather than assumed: walking every character in
the Tamil block that any lesson prints against the set the script lessons teach
returns **52 shown and 52 taught, zero untaught** — all eighteen core consonants
and ten of the twelve independent vowels. No other Indic track here is closed.
The shape assertions in the test therefore name different empty columns from
Marathi's, and an inventory that had assumed the Indic pattern would have written
several false gap notes.

### A near miss worth copying: material that exists and cannot be probed

Eight lessons in Tamil chapter 7 teach the Tamil digits ௧–௰ and **every one of
them declares an empty `introduces` list.** The material is taught, in the book,
and unprobeable. Telugu's inventory found the same class of thing from the other
side — chapters that carry only ETYMON or PRAGMATICS atoms, so their probes name
an etymology atom rather than a lexis one.

Both are the same lesson: **before writing a gap note, check whether the word is
taught under an atom kind you did not look at, or under no atom at all.** The
first case is a probe you can write today; the second is one frontmatter field
away from being one, and saying which it is in the note turns the inventory into
a work queue instead of a scoreboard.

### The number

**155 of 262 (59%)**, zero partials. Marathi 29%, Telugu 52%, Hindi 61%. The
shape is the finding rather than the score: the verb column is 20/26 and the
script column 9/14, and `Iṇaittoḍar (joining clauses)` is **0 of 7**. Tamil
cannot join two clauses at all — no `-um … -um`, no `ānāl`, no `alladu`, no
quotative `eṉṟu` — which is the same empty column Hindi and Telugu reported, and
it is what stops a well-taught verb column and a well-taught lexis column from
ever becoming a sentence.
