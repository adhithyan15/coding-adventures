---
category: Repo policy / workflow reminders
---

# Inspect generated JSON keys before scripting against its schema

A one-off planner summary assumed the queue lived at `items` and called
`p.items.slice(...)`; the planner's actual top-level key is `head`, so the
measurement command failed after successfully producing valid JSON.

Before writing an ad hoc extractor, parse the artifact and print its top-level
keys (or read the typed producer). Then use the confirmed schema. This keeps a
consumer typo from being confused with a failed measurement.

The native31 read-only migration planner also assumed every independently
inventoried occurrence carried optional line hints. Six supplemental rows had
full function hashes without those hints, so it refused before candidate or
source writes. Inspect all row variants first; recover a missing coordinate
only through a unique immutable function name and its exact complete hash.
Preserve unaffected conditional-arm budgets rather than assuming every arm
has the quantitative boundary. No source mutation or acceptance followed from
the failed planner.
A second guarded attempt also found an integration source
constructed by source(26), rather than stored as a whole Rust literal. Bind
the unchanged factory's exact26/32 reconstructions and only its argument and
matching marker; do not search-and-replace unrelated literals globally.
