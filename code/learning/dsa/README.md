# Data Structures and Algorithms

A curriculum that takes someone who can already program, but has never
studied data structures and algorithms, all the way to solving every problem
on LeetCode, the problems on similar sites such as GeeksforGeeks, and the
problems interviewers actually ask.

It is **language-agnostic**: the teaching is written once and published as one
edition per programming language, each adding its own code, built-ins and
traps. Python is the first edition.

The framework and its rules live in
[`DSA00`](../../specs/DSA00-data-structures-and-algorithms-curriculum.md).
Read that first. This page is the track's front door and will record progress.

## What makes this track different

- **Why are we solving this?** Every structure and pattern opens with the
  real systems that depend on it — spanning trees keep network switches from
  looping traffic forever, tries power autocomplete and router lookups — so
  the puzzles stop feeling arbitrary.
- **You build every structure yourself first.** When a problem needs a heap,
  you learn the heap's one rule, build it from an empty list, break it on
  purpose, and only then use the language's built-in.
- **Plain language.** Every idea is explained in everyday words and traced by
  hand before it gets its technical name. No proofs in the main text.
- **Big O as a skill.** You learn to *calculate* the cost of any code, not
  memorise a table, and practise it after every lesson.
- **Patterns, mastered.** Each pattern has signal words, a template, and
  drills where you name the pattern from the problem statement alone.
- **Dynamic programming without the scary name.** "Remember answers to
  smaller questions", learned from stairs, coins and road trips before code.
- **Language traps.** Built-ins that look free but are not (`list.pop(0)`,
  `x in some_list`, `s += c` in a loop) are taught just before they would bite.
- **Every problem has a place.** The problem atlas puts each problem — from
  LeetCode, GeeksforGeeks and others, listed once even when several sites
  carry it — at the earliest point where you have learned everything it needs.

## Layout

```text
dsa/
  README.md          this file
  CHANGELOG.d/       one file per change
  spine.d/           the ordered list of ideas — the ramp itself   (coming)
  chapters.d/        one file per chapter                          (coming)
  lessons/           one file per shared, language-free lesson     (coming)
  languages/<lang>/  one edition per language: code slots, traps,
                     build-it code (Python first)                  (coming)
  atlas.d/           one file per LeetCode problem                 (coming)
```

## Progress

| Stage | Topic | Status |
|---|---|---|
| 0 | Measuring work, and Big O | not started |
| 1 | Arrays, strings, and hashing | not started |
| 2 | First patterns | not started |
| 3 | Recursion and trees | not started |
| 4 | Priorities and graphs | not started |
| 5 | Dynamic programming | not started |
| 6 | Greedy and advanced patterns | not started |
| 7 | Range structures and rare techniques | not started |
| 8 | LeetCode's non-algorithm sections | not started |

Problems placed in the atlas: 0 of about 4,000 on LeetCode alone.
