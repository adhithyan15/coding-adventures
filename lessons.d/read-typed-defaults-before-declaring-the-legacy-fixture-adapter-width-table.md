---
category: Testing & coverage
---

# Read typed defaults before declaring the legacy fixture adapter width table

The first debug/default adapter used 1600 as the old default fixture width
because it was the new stroke-scaling denominator. The typed VennConfig default
is actually 800. Its strict width-table guard therefore failed one complete
scene test while the other six paint functions and all15 geometry tests passed.

Read the actual typed default and the fixture loop before declaring the table.
Retain that failed receipt, use the existing 400/800 fixture widths with explicit
1.25/2.5 expected strokes, and keep the guard narrow. Do not replace it with
production-derived expectations or weaken the whole-scene assertion. Separate
new 800/1600 scene fixtures verify the new default/override equations directly.
