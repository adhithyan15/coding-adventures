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

A native31 green-v2 read-only preflight compared a closed1293 Rust-root
manifest graph to all1473 repository Cargo manifests. The extra180 foreign-root
manifests caused refusal before Cargo. Restrict the comparison to the exact two
reviewed Rust roots, then require complete path/blob identity; preserve v2 as
unexecuted WITHHELD and run a reviewed v3 successor. In PowerShell, pass explicit
path arguments rather than Bash brace-expansion syntax, which fails to parse.
Inspect historical receipt keys before adding gates: the pinned native25 docs
receipt uses accepted/source_unchanged/utf8_strict, not a complete field. The
unexecuted v2 docs checker was withheld rather than changing that receipt.
Use explicit UTF8 when copying or parsing helper files on Windows; default
cp1252 decoding can corrupt punctuation or refuse an otherwise valid file.
