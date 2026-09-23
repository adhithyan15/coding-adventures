## Six second passes — the reinforcement blocker closes, and a zero-revisit atom turns out to cost two

Six `review` lessons. None of them teaches anything.

| metric | before → after |
|---|---|
| pre-A1 atoms revisited fewer than twice | 28 → **0** |
| ladder blockers | 3 → **2** (`verb-vocabulary 5`, `vocabulary 116`) |
| lessons | 353 → 359 |
| atoms introduced | **0** |
| headwords | **0** — `review` is not a `CONTENT_TYPE` |

### Twenty-eight atoms, forty-five retrievals

Every track before this one in the reinforcement programme was answered at one
retrieval per thin atom, because every one of them met only atoms already
revisited **once**. Sanskrit's twenty-eight split **17 at zero, 11 at one**, and
the criterion asks for two. A second mention inside the same lesson buys
nothing — the credit is per lesson — so the seventeen each needed a retrieval in
two *different* lessons:

```
17 x 2  +  11 x 1  =  45 retrieval slots
```

Forty-five slots, at about ten atoms a lesson, is six lessons.

### Where the six sit, and why they could not sit anywhere else

```
SA-R06  ch 6  where the words came from
SA-R11  ch11  the whole first meeting
SA-R12  ch12  what Sanskrit leaves out
SA-R13  ch13  five parts, three kinds of ancestry
SA-R29  ch29  how this book argues
SA-R30  ch30  from the door to the promise
```

Two orderings had to be satisfied at once, and they are not the same ordering.
In **book order** a retrieval must follow the sequence that taught the atom —
which is why the two script signs, taught at sequences 1281 and 1341, can only
be retrieved by the chapter-29 and chapter-30 lessons. In **curriculum-graph
order** prerequisites are judged on path rank, and those same script lessons sit
at rank 40, near the front. A lesson can be late in the book and early in the
graph, and both facts constrain it.

A third constraint fixed the rest: a lesson's `spine_node` equals its path
segment's in 353 of 353 existing lessons, so choosing where a lesson hangs
chooses what it is *about*.

### The lessons are not filler, and two of them are only possible late

Chapter 29's courtesy words end on the same visarga the first greeting did, and
*praṇāmaḥ* is built on the same *nam* that bows in *namaste* — so the chapter
that teaches politeness is the right place to retrieve the masculine-singular
ending and the four claims this book makes about word origins. Chapter 30 names
a door, a seat, a flower, a garland and the two cupped hands; the welcome owed
to a guest and the ending that turns *seeing* into *for the sake of seeing*
belong in the chapter that furnishes the room.

The chapter-13 lesson sorts the five body words by **how secure their ancestry
is** rather than by where they sit on a body — from *hṛdayam*, the cleanest
descent the book owns, to *karṇaḥ*, which Sanskrit never settled. That ordering
is the lesson: kin is not descent, and *nāsikā* is the cousin of its modern
look-alikes rather than their parent.

### What it cost the graph

The curriculum digest moves `87df31a5`/7520 → `aa7d8235`/7526 on a **36-line**
structural diff — the cheapest in this pin's history per lesson added. Every one
of the six reuses the extension its path segment already carried, so no new
extension node appears at all.
