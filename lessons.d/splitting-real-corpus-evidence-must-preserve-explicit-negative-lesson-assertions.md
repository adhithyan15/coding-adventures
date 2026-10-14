---
category: Testing & coverage
---

# Splitting real-corpus evidence must preserve explicit negative lesson assertions

The first real-corpus split moved positive expectations by writing system but
dropped two named negative assertions: the joined Urdu word and the connected
Russian school-hand word must never be emitted as isolated-letter strips.
Generic unit tests and per-track counts were still green, but a compensating
target swap could satisfy a count while violating either real lesson.

Inventory every positive and negative assertion before deleting an aggregate
case. Give named refusals direct writing-system evidence owners, and have the
independent reviewer compare the old and new assertion surface as well as the
current generated target set.
