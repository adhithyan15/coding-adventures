# DSA00 — Data Structures and Algorithms Curriculum Framework

## Overview

**DSA** is a curriculum that takes someone who can already program, but has
never studied data structures and algorithms, all the way to being able to
solve every problem on LeetCode — roughly 4,000 of them as of late 2026 — the
problems on similar sites such as GeeksforGeeks, and the problems that
interviewers actually ask.

The curriculum is **language-agnostic**. The teaching is written once, without
depending on any programming language, and is published as one **edition per
language** ("DSA — Python edition", later Java, JavaScript, …), each adding
that language's code, built-ins and traps. Python is the first edition.

It is three things at once, deliberately:

1. **A real data structures and algorithms course.** When a problem needs a
   heap, the reader does not just call `heapq`. They first learn what promise a
   heap makes, build one from an empty list, break it on purpose to see why the
   rules matter, and only then use the built-in — knowing what it costs.
2. **A gentle ramp through the whole LeetCode problem set** (and other
   sites'). Every problem is placed at the earliest point in the course where
   the reader has been taught everything it needs. Nothing is ever asked
   before it has been taught.
3. **An answer to "why am I solving this weird problem?"** Every structure
   and pattern is introduced through the real systems that depend on it —
   spanning trees through the protocol that stops network switches from
   flooding a network with looping traffic, tries through autocomplete and
   router lookup tables — so the puzzles stop feeling arbitrary. See
   [Why are we solving this?](#why-are-we-solving-this-real-world-context).

This is the first spec in the **DSA** series. It is a sibling of the human
language curriculum ([`HL00`](./HL00-human-language-curriculum-framework.md))
and borrows its strongest ideas — one new thing at a time, chapters that each
unlock a capability, spaced revisiting, and a hard rule against assuming
knowledge the reader was never given — but it is its own track with its own
content.

```text
                     ┌──────────────────────────────┐
  Shared spine ──────►  concepts, structures,        ├──► Markdown lessons
  (language-free)    │  patterns, in teaching order │     (read, build, solve)
                     │                              │
  Language layer ────►  traps + idioms + build-it    ├──► Runnable build-it code
  (Python first)     │  code per language           │     with tests, per language
                     │                              │
  Problem atlas ─────►  every LeetCode problem,      ├──► "What can I solve now?"
  (one file each)    │  placed after its prereqs    │     list per chapter
                     └──────────────────────────────┘
```

## Audience — who this is for

**The reader can program.** They can write a loop, a function, a class, and
read an error message. They have shipped software.

**The reader has not studied algorithms.** They may have heard "Big O" or
"dynamic programming" but could not explain either one, and theory-heavy
textbooks bounce off them because of the notation.

Three rules follow from this, and every lesson is checked against them:

- **No assumed algorithms knowledge.** No lesson may use an idea that an
  earlier lesson did not teach. "As you know, a BFS…" is forbidden unless BFS
  was taught earlier on the ramp — and then the lesson links back to it.
- **No mathematical jargon without a plain-language explanation first.** See
  [The plain-language rule](#the-plain-language-rule).
- **No assumed knowledge of the language's hidden costs.** Knowing how to
  write Python is not the same as knowing that `list.pop(0)` quietly walks the
  whole list. See [The language trap layer](#the-language-trap-layer).

## Motivation

Existing resources each solve part of the problem:

| Resource | What it does well | What is missing for this reader |
|---|---|---|
| Blind 75, Grind 75, NeetCode 150, LeetCode Top Interview 150 | Pick the problems most worth doing first | No teaching; assumes you already know the structures |
| Pattern courses (14–28 "patterns") | Teach how to recognise problem shapes | Usually skip building the structures; stop at a few hundred problems |
| CLRS-style textbooks | Complete, rigorous | Proof-and-notation heavy; not connected to interview problems |
| LeetCode itself | 4,000 problems, judged automatically | No order, no teaching, tags are coarse |
| GeeksforGeeks | Huge library of topic articles and practice problems, "must do" lists | Uneven depth and quality; articles assume the notation; no single ordered path |
| *Cracking the Coding Interview* and its companion site CareerCup (Gayle Laakmann McDowell), and the 2025 follow-up *Beyond Cracking the Coding Interview* | Interview process, company-tagged questions, a practical problem-solving method | A book, not a ramp; covers a few hundred problems |
| HackerRank, InterviewBit, CSES, Codeforces | More judged problems; CSES is a well-ordered set for depth | Little or no teaching |

None of them answers the question a learner keeps asking: *why would anyone
need this?* DSA connects them: the textbook's depth, in plain language,
ordered like a pattern course, grounded in real systems, covering the whole
problem set across sites.

## Divergence from the standard package process

DSA is a curriculum, not a package, so it lives under
`code/learning/dsa/` rather than `code/packages/`. Like HL, it has no
publishable artifact of its own.

What still applies:

- **Build-it code is real code.** Every "build it yourself" lesson ships a
  runnable implementation with tests, and that code follows the repo's normal
  standards for its language (ruff for Python, literate comments, high
  coverage). The exact home for this code is open question Q3.
- **Changelog.** The track has a sharded `CHANGELOG.d/`.
- **Specs first.** This spec lands before any lesson content.

What does not apply: BUILD files and package metadata for the Markdown lessons
themselves.

Where this repo already has a literate implementation of a structure (for
example `heap`, `trie`, `fenwick-tree`, `avl-tree`, `skip-list`, `hash-map`,
`directed-graph` under `code/packages/`), the build-it lesson links to it as a
"how a production version differs" reference, and the existing
[algorithms learning notes](../learning/algorithms/README.md) (Kahn's
algorithm, trees and indexes) are linked from the matching chapters instead of
being rewritten.

## The structure of the curriculum

### Three layers

1. **The shared spine and lessons** — the language-independent ordered list
   of every idea the course teaches: concepts ("how work grows as input
   grows"), structures ("a heap"), and patterns ("sliding window"), plus the
   lessons that teach them. The spine *is* the ramp.
2. **The language editions** — for each programming language: its code for
   every lesson, its traps, its idioms, which built-in matches each structure,
   and the runnable build-it code with tests.
3. **The problem atlas** — one entry per problem saying where on the spine it
   becomes solvable. See [The problem atlas](#the-problem-atlas).

### Language-agnostic by design

The rule: **a shared lesson never depends on a particular language.** It
explains the idea in prose, hand traces and pictures, and shows algorithms in
plain step-by-step pseudocode (numbered steps in English, not a fake
language with its own syntax to learn). Wherever real code belongs, the shared
lesson leaves a named **slot**, and each edition fills that slot:

```text
lessons/heap-build.md                 shared: story, rule, trace, pseudocode,
                                      cost, real-world context, "break it"
  └─ slot: heap-build/code            ┐
  └─ slot: heap-build/builtin         ├─ filled per edition
  └─ slot: heap-build/traps           ┘
languages/python/slots/heap-build.md  Python: code, heapq, its traps
languages/java/slots/heap-build.md    Java: code, PriorityQueue, its traps
```

Publishing an edition means weaving the shared lessons together with that
language's slot files. An edition is publishable when every slot used by the
chapters it covers is filled; a missing slot is a build error, not a silent
gap. Adding a new language therefore never edits a shared lesson, and fixing
a shared lesson fixes it in every edition at once. (This mirrors how the HL
curriculum keeps one shared spine across many language tracks.)

Some lessons are inherently language-specific (the trap cards, and notes
like "this language has no built-in heap"). Those live only in the edition.
The shared lesson may say "your edition covers what your language provides
here" and leave a slot.

### Units

| Unit | What it is | Typical size |
|---|---|---|
| **Stage** | A big phase of the course, e.g. "Trees and recursion" | 5–10 chapters |
| **Chapter** | One capability the reader gains | 3–8 lessons + its problems |
| **Lesson** | One idea | 5–15 minutes of reading |
| **Problem** | One problem from LeetCode or another site (or an original one) placed in a chapter | — |

### Lesson kinds

Every lesson is exactly one of these kinds, so the reader always knows what
the lesson wants from them:

- **Idea** — a concept with no code yet. "What does it mean for work to
  'double when the input doubles'?"
- **Build** — construct a structure from scratch: where it lives in real
  systems, its promise, a hand trace, the code, tests, then a "break it" section that violates one rule and shows
  what goes wrong.
- **Pattern** — a reusable problem shape: how to recognise it from the problem
  statement, the template, and the classic mistakes.
- **Trap** — a language-specific hidden cost or surprise (lives only in a language edition).
- **Worked problem** — a complete solution walked through step by step,
  including the wrong first attempts and why they fail.

### Chapter shape

Every chapter starts with a **real-world hook** (where this shows up — see
[Why are we solving this?](#why-are-we-solving-this-real-world-context)) and a
one-sentence **capability promise** ("After this chapter you can solve any
problem that asks for the best contiguous stretch of an array"), and ends with
its problems in three bands:

1. **Worked** — 1–2 problems solved fully in the text.
2. **Faded** — 2–4 problems where the text gives the approach and a skeleton;
   the reader fills in the rest.
3. **Solo** — the rest of the chapter's placed problems, with hints hidden
   behind a fold, ordered easiest first.

This "worked → partly worked → on your own" fade is the main mechanism that
keeps the ramp gentle.

## The ramp rules

These are the invariants — promises the curriculum keeps at every step:

1. **One new thing at a time.** A lesson introduces at most one new idea. A
   problem may be new in *either* its structure *or* its pattern, never both.
   Someone who fails a problem that is new in two ways cannot tell which one
   they failed. (This is the same rule as HL12's script/meaning frontier.)
2. **Nothing before it is taught.** Every lesson and every placed problem
   lists its prerequisite spine nodes, and all of them come earlier on the
   ramp. This is checkable by a script once the spine is machine-readable.
3. **Every chapter unlocks something usable.** If a chapter's capability
   promise cannot be stated in one plain sentence, the chapter is too big or
   too vague.
4. **Spaced revisiting.** Each chapter's solo band includes 2–3 problems from
   earlier chapters, disguised in a new setting, so old patterns get
   practised again just as they start to fade. Recognising a pattern in
   disguise is the actual interview skill.
5. **Build before borrow.** The first time a structure is needed, the reader
   builds it. After that, they may use the language's built-in, and the
   language edition tells them exactly what that built-in costs.
6. **Wrong attempts are taught, not hidden.** Worked problems show the
   natural first attempt, why it is too slow or wrong, and how the better
   idea is found. Interviewers grade the path, not just the answer.

## The plain-language rule

Theoretical computer science writes in the language of mathematics. This
curriculum does not. The rule:

> A technical term may appear only after the idea has been explained in plain
> words, and the first time it appears it is glossed. The idea is the thing
> being taught; the term is just a name to recognise it by in interviews and
> on other websites.

Interviewers will say "Big O" and "amortized", so the reader must learn the
words — but as labels for ideas they already understand, never as the way the
idea is introduced.

### Replacement table

| Instead of introducing… | Teach first… | Then name it |
|---|---|---|
| Asymptotic complexity, O(n) | "If the input doubles, does the work double, quadruple, or stay the same?" — measured by counting steps on real inputs | "People write this as O(n)" |
| Logarithm, O(log n) | "How many times can you cut it in half before one is left?" (1,000,000 → about 20) | "That count is called log n" |
| Amortized analysis | "Usually cheap, occasionally expensive, but the expensive times are paid for by the cheap ones before them" — a savings-account story | "Amortized cost" |
| Invariant | "A promise the code keeps true after every step" | "An invariant" |
| Proof by induction | "It works for the smallest case, and each step keeps it working, so it works for every size" | "Induction" |
| Recurrence relation | A drawn tree of calls, counting the work at each level | "A recurrence" |
| NP-hard / NP-complete | "Nobody knows a fast method; the best known ones try a huge number of possibilities" | "NP-hard" |
| Optimal substructure / overlapping subproblems | "The best answer for the big problem is built from best answers to smaller ones, and the same small ones keep coming up" | The DP terms |
| Graph G = (V, E) | "Things (cities) and connections between them (roads)" | "Vertices and edges" |

### Allowed tools instead of proofs

Correctness is explained with **"why it works" stories**, **hand traces in
tables** (the state of every variable, step by step), **pictures**, and
**small counterexamples** that show why a tempting shortcut fails. Formal
proofs never appear in the main text. Where a reader might want one, a short
"for the curious" box may point to a textbook.

### Example: how the heap build lesson opens

> You run a hospital waiting room. Patients arrive all day, each with an
> urgency number. Whenever a doctor is free, you need the *most* urgent
> patient — fast. You do not care about the order of everyone else.
>
> A sorted list would work, but every new arrival means shifting people over
> to make room: slow when the room is full. A plain unsorted list makes
> arrivals instant, but finding the most urgent person means checking
> everyone.
>
> A heap is the in-between: it keeps the list *just sorted enough* that the
> most urgent patient is always at the front, and both arriving and leaving
> take only about as many steps as you can halve the room's size — about 20
> steps for a million patients.

The lesson then states the heap's one rule ("every parent is at least as
urgent as its children"), traces inserts and removals in a table, builds it in
code, and breaks it: a version that forgets to sift down after removal, run
on real data, so the reader *sees* the wrong patient come out first.

## Big O, taught as a skill

Big O is taught fully — interviewers ask "what's the complexity?" after
almost every problem, and a reader who can only recite "O(n log n) for
sorting" cannot answer for their own code. The aim is that the reader can
**work it out** for code they have never seen.

### The order it is taught in

1. **Count, don't guess.** Run a function on 1,000, 2,000 and 4,000 items and
   count its steps (with a counter variable, then with a timer). Watch the
   count double, quadruple, or barely move.
2. **Name the shapes.** Only now introduce the labels, each with a picture and
   an everyday example:

   | Name | Plain meaning | Everyday example |
   |---|---|---|
   | O(1) | Same work no matter how big | Looking at the first page of a book |
   | O(log n) | Add one step each time the input doubles | Finding a word in a dictionary by opening it in the middle |
   | O(n) | Work grows in step with the input | Reading every page once |
   | O(n log n) | A little more than reading everything once | Sorting a deck of cards well |
   | O(n²) | Double the input, four times the work | Everyone at a party shaking hands with everyone else |
   | O(2ⁿ) | One more item doubles the work | Trying every combination of toppings |
   | O(n!) | Try every possible order | Every possible seating order at a dinner |

3. **Calculate it with a handful of rules**, each taught with code examples
   and then practised on short snippets:
   - Steps one after another **add**; then keep only the biggest part
     (O(n) + O(n²) is O(n²)).
   - A loop inside a loop **multiplies** (n × n = n²) — but only if the inner
     loop really runs n times each time; the course shows the classic
     two-pointer loop that *looks* nested and is actually O(n).
   - **Ignore fixed multipliers.** Going through the list twice is still O(n).
   - **Watch for hidden loops.** A built-in can hide a whole walk through the
     list (ties into the [language trap layer](#the-language-trap-layer)).
   - **Halving means log.** If each step throws away half of what is left,
     count how many halvings fit.
   - **Recursion: draw the tree of calls**, count the work on each level, add
     up the levels. This replaces "the master theorem" entirely.
   - **Memory counts too:** new lists, maps, and the depth of the call stack.
4. **Best, average and worst case**, with quicksort and hash maps as the
   examples where they differ, and **"usually cheap"** costs (the growable
   array's append) told as the savings-account story.
5. **From Big O to "will it pass?"** — the constraint table, using Python's
   real speed.

### Practice format

Big O gets its own drill lessons: short code snippets where the reader writes
down the time and memory cost before revealing the answer and the reasoning.
Some snippets are traps on purpose (the loop that looks O(n²) and is not; the
one-liner that hides an O(n) built-in inside an O(n) loop).

## Dynamic programming without the scary name

Dynamic programming has a reputation for being confusing, and part of the
reason is the name, which describes nothing. The course says so plainly and
then never relies on the name to explain anything.

### Start with the name's story

The lesson opens by taking the name apart: in the 1950s "programming" meant
*planning a schedule* or *filling in a table* (as in a TV programme listing),
not writing code. The mathematician who coined the term, Richard Bellman,
later wrote in his autobiography that he picked the name partly to disguise
mathematical research from a government official who disliked the word
"research", and chose "dynamic" because it was hard to use as an insult. So
the name is camouflage, not a description. The course's working name for the
idea is:

> **Remember answers to smaller questions so you never work them out twice.**

### Build the idea from everyday situations first

Before any code, the idea is met in situations the reader already
understands:

- **Climbing stairs.** "How many ways can you climb 10 stairs taking 1 or 2
  steps at a time?" You only need the answers for 9 and 8 stairs — and those
  need 8 and 7, and so on. Write the answers on sticky notes as you go.
- **Making change.** "What is the fewest coins to make 37 cents?" If you know
  the best way to make every amount below 37, the answer is one coin plus the
  best way to make what is left.
- **A road trip.** The cheapest route to the final city goes through *some*
  city just before it; if you already know the cheapest route to each of
  those, you just pick the best one plus the last leg.
- **Spell-check suggestions.** "How many single-letter edits turn *kitten*
  into *sitting*?" Answered by filling in a grid where each square only
  looks at its neighbours.

### Then the same three steps every time

Every DP lesson follows the same recipe, so the technique stops feeling like
magic:

1. **Write the slow recursive version** that asks smaller questions (the
   reader already knows recursion from Stage 3). Time it and watch it crawl.
2. **Add a notebook** (memoisation): before working out an answer, check if
   it is already written down. Time it again — the speed-up is the lesson.
3. **Fill the notebook in order** (a table, bottom-up) and, where possible,
   keep only the last few entries.

Each problem's lesson always answers the same two plain questions before any
code: *"What smaller question would help me answer this one?"* and *"What do
I need to write down to describe a smaller question?"* (the second is what
textbooks call the "state"). The DP chapters are ordered so that only one of
those two questions gets harder at a time.

## Why are we solving this? Real-world context

Interview problems look like puzzles invented to torment people. Most of them
are small, cleaned-up versions of problems real systems solve every second.
A learner who knows that a spanning tree is how network switches avoid
flooding a network with looping traffic stops fighting the problem and starts
caring about it. The curriculum therefore never introduces a structure or
pattern without saying where it lives.

### The rules

1. **Motivation before mechanics.** Every chapter opens with a short "where
   this shows up" story drawn from real software or hardware, *before* any
   definition. The reader should want the tool before being handed it.
2. **Every build lesson has a "where it lives" box** listing two or three
   real systems that use the structure, and, where this repo implements it,
   a link to the repo's own use (for example, this repo's build tool uses
   topological sort to decide build order).
3. **Every worked problem says what it is a toy version of** — or says
   honestly that it is a pure training exercise for a particular skill. Not
   every LeetCode puzzle has a real-world twin, and the course never invents
   one.
4. **Claims must be true.** Like the HL curriculum's rule that etymologies be
   traceable, every real-world claim names the real system and is checkable.
   "Routers use tries" is acceptable only with which routers and for what
   (longest-prefix matching of IP addresses in forwarding tables).

### Initial real-world map

| Structure or pattern | Where it shows up |
|---|---|
| Spanning tree | The Spanning Tree Protocol keeps Ethernet switches from forwarding traffic around loops; the *minimum* spanning tree is the cheapest way to connect everything — laying cable or pipe, grouping similar data points |
| Shortest paths (Dijkstra) | Internet link-state routing (OSPF, IS-IS) runs it on every router; map directions (with speed-ups) |
| Trie | Autocomplete and spell-check; IP routing tables (longest-prefix match) |
| Heap | Timer queues in event loops and language runtimes; Huffman coding in compression (this repo's `huffman-tree`); merging sorted runs in database sorts; "top trending" lists |
| Hash map | Caches, deduplication, database hash joins, every language's dictionary/object type |
| Topological sort | Build systems (this repo's build tool), package managers, spreadsheet recalculation, course prerequisites |
| Union-find | Kruskal's spanning tree; finding connected regions in images; type inference in compilers |
| Binary search | `git bisect` finding the commit that broke a test; searching inside database index pages |
| Sliding window | TCP's flow control window; rate limiters; moving averages on dashboards |
| Stack | Undo, the browser back button, function calls, parsers (this repo's parser packages) |
| Queue and ring buffer | Print queues, network and audio buffers, log buffers |
| Breadth-first search | "Degrees of separation" in social networks, web crawlers, garbage collectors finding reachable objects |
| Backtracking | Sudoku and constraint solvers, regular-expression engines |
| Edit distance / longest common subsequence (DP) | `diff` and code review tools, spell-check suggestions, DNA sequence alignment |
| Line-breaking (DP) | TeX's paragraph layout chooses line breaks with dynamic programming |
| Graph colouring | Compilers assigning variables to CPU registers |
| Fenwick tree | Invented for running totals in data compression (arithmetic coding); leaderboards and range counts |
| Interval scheduling | Calendar apps finding free meeting rooms |
| Matching and max flow | Assigning drivers to riders, workers to shifts |
| LRU cache | CPU caches, operating-system page caches, CDNs |

This table seeds the "where it lives" boxes; each entry is checked and
expanded when its chapter is written.

## Mastering patterns

Patterns are the bridge between knowing the structures and solving unseen
problems, and mastering them is an explicit goal, not a side effect.

- **Each pattern gets its own chapter** with a recognition section: the
  words in a problem statement that hint at it. For example: "contiguous
  subarray / substring" → sliding window; "sorted array" → binary search or
  two pointers; "k largest / k closest" → heap; "all combinations / all
  arrangements" → backtracking; "fewest steps" on a grid → breadth-first
  search; "number of ways" or "cheapest way" → dynamic programming;
  "next greater element" → monotonic stack.
- **Recognition drills.** Short lessons that show only a problem statement
  (written for the course) and ask "which pattern, and why?" — no code. These
  need no keyboard, so they are tagged as listen-only candidates.
- **Mixed review sets.** From Stage 3 on, every few chapters there is an
  unlabelled set of problems from all the patterns taught so far, because in
  an interview nobody tells you which chapter a problem came from.
- **Pattern map.** A one-page reference that grows as the course goes on:
  every pattern, its signal words, its template, its cost, and its common
  mistakes.

## The language trap layer

Many interview bugs, and many "correct but too slow" submissions, come from
built-ins that look free but are not, or that behave differently than the
reader assumes. Every language edition teaches its own traps explicitly; the
card format below is shared, and the catalogue is per edition.

### Trap card format

Every trap is one lesson in a fixed shape:

1. **What it looks like** — the innocent code.
2. **What it really does** — the hidden cost or behaviour, in plain words.
3. **See it yourself** — a short script the reader runs, timing or printing
   the surprise (for example, `pop(0)` on 10,000 versus 1,000,000 items).
4. **Why** — how the built-in is implemented underneath, tied to a structure
   the course has built (or will build).
5. **Use instead** — the replacement, and when the original is actually fine.
6. **In an interview** — whether using the built-in is normally acceptable,
   worth asking about, or likely to prompt "can you implement that yourself?"

Each trap is placed on the spine just before the first problem where it would
bite, so it arrives when it matters rather than as a list to memorise.

### Python edition trap catalogue (initial)

**Hidden walks through the whole list (looks like one step, is not):**

- `list.pop(0)` and `list.insert(0, x)` shift every element → use
  `collections.deque` for queues. Second trap: indexing into the *middle* of
  a deque is slow too.
- `x in some_list`, `list.index`, `list.count`, `list.remove` all check items
  one by one → use a `set` or `dict` when you look things up repeatedly.
- Slicing (`s[1:]`, `nums[:k]`) makes a copy → recursion that passes
  `s[1:]` at every call quietly costs a full copy each time; pass an index.
- `bisect.insort` finds the spot fast but still shifts everything after it.
- Calling `sorted()` or `min()`/`max()` inside a loop turns one pass into many.

**Strings:**

- Strings cannot be changed in place; `s += c` in a loop may rebuild the whole
  string each time. CPython sometimes optimises this, but it is not
  guaranteed → collect pieces in a list and `"".join` them once.
- `str.replace`, `split`, `strip` and slicing all create new strings.

**Structures that do not behave as expected:**

- `heapq` only does smallest-first. For largest-first, store negated values.
  It has no "change this item's priority" operation, and when two entries tie
  on priority it compares the next element of the tuple — which crashes if
  that element is something like a dict or a node object. Add a counter as a
  tie-breaker.
- `[[0] * cols] * rows` makes every row the *same* list → use a list
  comprehension.
- Mutable default arguments (`def f(seen=[])`) are shared between calls.
- Changing a dict or set while looping over it raises an error (or skips
  items, for lists).
- Shallow vs deep copy: `list(grid)` copies the outer list only.

**Numbers:**

- Python integers never overflow. Problems such as "Reverse Integer" that ask
  you to detect 32-bit overflow need you to check bounds yourself.
- `-7 // 2` is `-4` (rounds down, not toward zero) and `-7 % 2` is `1`;
  `int(-3.5)` is `-3`. Languages disagree here and binary search midpoints
  and digit problems depend on it.
- `0.1 + 0.2 != 0.3`; compare floats with a tolerance or avoid them.
- `is` versus `==`: small integers happen to be shared objects, so `a is b`
  "works" in testing and fails on larger numbers.

**Recursion:**

- Default recursion limit is about 1,000 calls; a depth-first search on a
  10,000-node path crashes. Raising the limit can crash the interpreter
  outright → learn to convert to an explicit stack.
- Python does not optimise tail calls.
- `functools.cache` makes memoised recursion easy but keeps every result in
  memory and still recurses deeply.

**Speed in general:**

- CPython runs roughly ten million simple steps per second. A method that is
  fine in C++ can time out in Python, so the constraint table
  (Stage 0) is given with Python numbers.

Later languages each get their own catalogue — for example Java (`==` on
boxed `Integer`, the legacy `Stack` class, `int` overflow), JavaScript
(`sort()` comparing as strings by default, `shift()` being slow, no built-in
heap), C++ (`size_t` underflowing in `i - 1`, `vector<bool>`), Go (`append`
sharing backing arrays, random map order).

## The stages

This is the initial shape of the spine. Chapter counts are estimates; the
spine shards (see [Storage](#storage-layout)) are the source of truth once
written.

### Stage 0 — Measuring work, and Big O

How to count steps; how work grows as input grows; memory as a cost; Big O
notation itself, and how to **calculate** it for any piece of code (see
[Big O, taught as a skill](#big-o-taught-as-a-skill)); the **constraint
table** — reading the input limits in a problem statement to know how fast
your method must be (for example, "up to 100,000 items" rules out checking
every pair). Solving one problem slowly, then quickly, and timing both.

Big O does not end in Stage 0. Every build lesson and every worked problem
afterwards ends with a short "what does this cost?" section where the reader
works out time and memory themselves before the text shows the answer, so
the skill gets hundreds of repetitions across the course.

### Stage 1 — Arrays, strings, and hashing

Build a growable array (and see where the "usually cheap" append comes from);
strings and why they cannot change in place; build a hash map (chaining, then
open addressing) and a hash set; counting with maps; prefix sums.

### Stage 2 — First patterns

Two pointers; sliding window (fixed size, then variable size); sorting (build
insertion sort, merge sort, quicksort, counting sort — and learn when sorting
first makes a problem easy); binary search on arrays, then on the answer;
build a stack (bracket matching, undo); build a queue as a ring buffer; build
a linked list (reversal, merging); fast and slow pointers; intervals.

### Stage 3 — Recursion and trees

Recursion by tracing the call stack by hand; binary trees and the three walk
orders; build a binary search tree; depth-first patterns on trees; level-order
(breadth-first) walks; backtracking (subsets, permutations, combinations,
pruning); divide and conquer.

### Stage 4 — Priorities and graphs

Build a heap; top-k, two heaps for a running median, merging k sorted lists;
graphs as adjacency lists and grids; breadth-first and depth-first search on
graphs; dependency order (topological sort, linking the existing Kahn's
algorithm note); build union-find; shortest paths (Dijkstra, 0-1 BFS,
Bellman-Ford); minimum spanning trees; build a trie.

### Stage 5 — Dynamic programming ("remember answers to smaller questions")

Approached slowly, from recursion the reader already knows: memoise a slow
recursive solution, then flip it into a table. One-dimensional DP; grids;
knapsack-style choices; two-string problems (longest common subsequence, edit
distance); interval DP; DP on trees; bitmask DP; state machines (stock
problems); digit DP last. How the confusing name is taken apart is in
[Dynamic programming without the scary name](#dynamic-programming-without-the-scary-name).

### Stage 6 — Greedy and advanced patterns

Greedy choices and the "swap argument" for why they work (told as a story,
not a proof); monotonic stack and monotonic queue; bit manipulation; the
small amount of number theory interviews use (gcd, prime sieve, working
modulo 1,000,000,007); design problems (build an LRU cache, iterators,
rate limiters); string algorithms (KMP, rolling hash, Z-function).

### Stage 7 — Range structures and rare techniques

Build a segment tree (then lazy updates); build a Fenwick tree; sparse tables;
ordered sets and balanced trees (build an AVL tree); square-root
decomposition; advanced graphs (bridges, strongly connected components, Euler
paths, max flow, bipartite matching); game theory; computational geometry;
randomised techniques; suffix arrays; Manacher's algorithm. This stage exists
so that *every* LeetCode Hard has a home.

### Stage 8 — LeetCode's non-algorithm sections

LeetCode also has Database (SQL), pandas, Shell, Concurrency, and JavaScript
sections. They are separate mini-tracks with their own ramps. Database links
to the existing [SQL and storage](../learning/sql-storage/README.md) learning
material. These come after the algorithm stages are well under way.

### Alongside: interview craft

A small parallel track, unlocked gradually: restating the problem and asking
about constraints; thinking out loud; testing your own code by hand before
running it; handling "can you do better?"; timed mock rounds; and the newer
formats reported in 2026 — rounds where you code with an AI assistant, and
rounds where you read and fix an unfamiliar codebase.

### Milestones

Doing all 4,000 problems is a multi-year goal (at two problems a day, about
five and a half years). The ramp is ordered so that the reader becomes
interview-ready long before that:

| Milestone | Reached after | Roughly |
|---|---|---|
| **Interview-ready core** | Stages 0–4 plus the curated interview lists placed so far | ~250 problems |
| **Strong** | Stage 5 and 6 | ~700 problems |
| **Expert** | Stage 7 | ~1,500 problems |
| **Every problem** | The full atlas sweep | ~4,000 |

## The problem atlas

### What an atlas entry holds

One file per **problem**, not per website. The same task often appears on
several sites under different names (LeetCode's "Two Sum" is also a
GeeksforGeeks practice problem and a classic book exercise). The atlas keeps
one entry for the task and lists every site that carries it, so the reader
never meets "a new problem" that is really one they solved last month:

```json
{
  "id": "two-sum",
  "title": "Two Sum",
  "sources": [
    {"site": "leetcode", "number": 1, "url": "https://leetcode.com/problems/two-sum/",
     "difficulty": "easy", "premium": false, "section": "algorithms"},
    {"site": "geeksforgeeks", "url": "https://www.geeksforgeeks.org/..."}
  ],
  "variant_of": null,
  "real_world": "Matching pairs that cancel out, e.g. reconciling debits against credits.",
  "primary": "SPINE-HASH-MAP-LOOKUP",
  "requires": ["SPINE-HASH-MAP-BUILD", "SPINE-ARRAY-WALK"],
  "chapter": "0120-hash-maps",
  "band": "worked",
  "lists": ["blind75", "grind75", "neetcode150", "top-interview-150", "lc75"],
  "notes": "First problem where the constraint table rules out checking every pair."
}
```

**Same problem or a variant?** Two listings are the same entry when they ask
for the same output from the same input, whatever the wording. A listing that
changes the task (sorted input, return all pairs, count pairs) is its own entry
with `variant_of` pointing at the original, so the ramp can place it right
after it as a disguised revisit.

**Sources covered.** LeetCode is the backbone and the completeness target.
GeeksforGeeks practice problems, *Cracking the Coding Interview* exercises
(referenced by chapter and number only), HackerRank, InterviewBit and CSES are
attached as further sources over time.

### Placement rule

A problem is placed in the **earliest chapter after which every idea in its
`requires` list has been taught**. If placing a problem needs an idea the
spine does not have yet, that is a gap: it goes into the backlog as a new
spine node, not into the nearest chapter as a stretch.

### What the atlas never contains

- **No problem statements copied from any site or book.** The atlas stores
  numbers, titles, links and our own placement notes. Worked problems in the lessons use
  original problems written for this course, or describe a LeetCode problem in
  our own words alongside the link.
- **No premium content.** Premium problems are listed (with the flag set) so
  coverage is complete, but nothing behind the paywall is reproduced.
- **No solutions to copy-paste into LeetCode** outside the worked band.
  Faded and solo bands give approaches and hints, because the reader learns
  by solving.

### Filling the atlas

1. **Seed** with the union of the curated interview lists (Blind 75, Grind 75
   and 169, NeetCode 150 and 250, LeetCode Top Interview 150, LeetCode 75):
   about 300 problems that get placed first.
2. **Sweep chapter by chapter.** As each chapter is written, place every
   problem whose main technique is that chapter's, using LeetCode's own topic
   tags as a starting point and checking each one by hand.
3. **Close the gaps.** Problems that fit nowhere drive new spine nodes,
   mostly in Stage 7.

How the problem metadata is collected is open question Q4.

## Storage layout

Following the repo's sharding convention (one owner per file, ordinals spaced
by ten so new items slot in without renaming), so many agents and branches can
add lessons and atlas entries at once without conflicting:

```text
code/learning/dsa/
  README.md                      what the track is, how to use it, progress
  CHANGELOG.d/                   one file per change
  spine.d/
    _meta.json                   stages, version
    0010-SPINE-COUNT-STEPS.json  one spine node per file, in teaching order
  chapters.d/
    0010-measuring-work.md       capability promise, lesson list, problem bands
  lessons/
    <slug>.md                    one lesson per file (idea/build/pattern/worked)
  languages/                     one directory per edition
    python/
      README.md                  edition front matter: version, conventions
      slots/<lesson-slug>.md     this language's code for each shared lesson
      traps/<slug>.md            trap cards
      build/                     runnable build-it code and tests (see Q3)
    java/ …                      later editions, same shape
  atlas.d/
    two-sum.json                 one problem per file, every site listed
```

### Lesson front matter

```yaml
---
id: heap-build
kind: build            # idea | build | pattern | trap | worked
spine: SPINE-HEAP-BUILD
requires: [SPINE-ARRAY-INDEXING, SPINE-COUNT-HALVINGS, SPINE-BINARY-TREE-SHAPE]
minutes: 15
slots: [code, builtin, traps]   # what each edition must fill for this lesson
---
```

## Delivery

- **First:** Markdown lessons and runnable code, readable on GitHub.
- **Later (not this spec):** a book build like HL's, and possibly a practice
  app with spaced review. Lessons that need no code (idea lessons, many
  pattern-recognition drills) are tagged so they could become listen-only
  lessons in the HL commute style.

## Roll-out plan

Each item is one PR:

1. **DSA00** — this spec.
2. Track skeleton: `README.md`, `CHANGELOG.d/`, empty shard directories, and
   a checker that every lesson's `requires` come earlier on the spine.
3. Stage 0 — measuring work: shared lessons plus the Python edition's slots
   and constraint table.
4. Python trap cards needed by Stages 0–2.
5. Stage 1, one chapter per PR.
6. Atlas seed: the curated-list union, placed against the spine written so far.
7. Stages 2 onward, one chapter per PR, each placing its atlas problems.

## Open questions

- **Q1. Next editions.** The curriculum is language-agnostic and Python is the
  first edition. Which editions come next, and when?
- **Q2. Book and commute formats.** Is a book or a listen-only version wanted,
  and when?
- **Q3. Where build-it code lives.** Inside the track
  (`code/learning/dsa/languages/python/build/`, tested by a small local
  harness), or as small packages under `code/packages/python/` that follow
  the full package standard?
- **Q4. Problem metadata.** The atlas needs every problem's number, title,
  difficulty, premium flag and tags. LeetCode's terms of service should be
  checked before any automated collection; the fallback is entering it by
  hand chapter by chapter as the sweep reaches each topic.
- **Q5. System design.** Out of scope for DSA. A separate track if wanted.
