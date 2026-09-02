## HL-C242 — a tidy chapter can still be the wrong size, and the giveaway is items per lesson

German chapter 6 is generated. **9 hand-written German chapters remain.** The
earlier entries in this series were about chapters whose `.tex` hid something.
This one had nothing hidden and was still four times too small.

### Every instrument agreed, and every instrument was wrong

`handwritten_parity.py` scored a gap of **1 block**. `grep -l '^chapter: 6$'`
found **2 lessons**, and the `.tex` rendered exactly **2 sections**. No writing
lessons off the page, no untaught words in the closing dialogue, no
disagreement anywhere.

The chapter taught **ten numbers in two lessons**. Five items per sitting,
against a rule of one — and no gate in the repo reports items per lesson,
because "item" is a judgement about content. The parity script counts prose
blocks, and two well-written blocks holding five words each look exactly like
two well-written blocks.

**So the question a green sizing check does not answer is: how many things does
one lesson ask the reader to learn?** Count the headwords inside a lesson, not
the lessons inside a chapter.

### A four-column table is a fifth signal, and this one is mechanical

Both of the old tables were `German | English twin | shared ancestor | Latin
cousin`. `maxLinearisableTableColumns` is 3, and `narration.test.ts` keeps a
corpus-wide count of tables the narrator refuses to speak — which moved
**51 -> 49** when they went.

That count is worth checking *before* authoring rather than after: a hand-written
chapter's refused tables are invisible to it until the chapter generates, so
retiring a chapter with wide tables always moves the pin. French chapter 6 and
Kannada chapter 1 are the same story in the same assertion.

### A dropped column is not always a dropped claim

The `shared ancestor` column held ten Proto-Indo-European reconstructions. All
ten are gone and no teaching went with them, because the reconstructions were
evidence for a rule the chapter never stated. Stating the rule — a sound law
changes every word at once, so the differences are predictable — costs one
lesson and is worth more than the column was.

**Ask what the decoration was evidence for.** If the answer is a rule, teach the
rule and cut the evidence.

### Keep an old lesson id if anything points at it

`GE-C06-zahlen-1-5` and `GE-C06-zahlen-6-10` are named as prerequisites or
reviews by six lessons in chapters 7, 8, 9, 12 and 31. Splitting ten numbers
into ten lessons did not need those ids destroyed: each became the **recap** of
the five before it, which is a real thing to practise — counting is one motion,
not five remembered words — and every reference still resolves.

### German still omits its own counting concept

`curriculum.d/spine/0110-SPINE-COUNT-ONE-TO-FIVE.json` declares
`NUMBER-ONE-TO-FIVE` **omitted** for German, while chapter 6 plainly teaches one
to five. That predates this work and was left alone here: correcting it needs a
new path node under that spine segment, which renumbers every later path shard,
and a renumber was already in flight on chapter 5. **It should be corrected by
whoever next has a reason to touch German's path shards.**
