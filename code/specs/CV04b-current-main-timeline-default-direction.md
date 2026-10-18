# CV04b: reconcile the Timeline default direction

Status: specified before tests and implementation. This is bounded progress in
PR #17288 and provenance epic #15830; the full Closure completion contract is
open. Miri is **DEFERRED/PENDING**, never passed. Its frozen checkout, inputs,
target and receipts remain untouched; final-source acceptance follows delivery.

## Observable contract

A bare `timeline` header selects left-to-right layout, matching the selected
current-main default. `TimelineDirection::default()` is also `LeftRight`, so
typed construction and source ingestion agree. Explicit `timeline TD` remains
top-down and explicit `timeline LR` remains left-to-right. Header matching keeps
the existing case-insensitive grammar and whitespace rules.

Only the enum's default marker and the parser's initial/header direction choice
change. There are no fields, dependencies, font choices, layout formulas,
wrapping rules, colon splitting, color grouping or architecture ABI changes.
Owned configuration selection, accessibility-block token ownership, bounded
readers, checked arithmetic and all explicit-direction fixtures stay intact.
Raw main is not accepted by this slice.

## Evidence and retention

Before changing production, retain the old whole source files and inventory all
Timeline constructors, default calls, authored bare-header sources and corpus
consumers. Add complete semantic-IR tests for bare, TD, LR and mixed-case headers,
including empty documents, titles, accessibility labels, multiline descriptions,
configuration and misleading direction text inside owned content. Existing
sources and assertions are retained byte for byte.

Portable product tests declare independent complete IR, checked layout, Scene,
glyph/font/color/query and metadata expectations for bare, explicit LR and TD
one-period/one-event sources. At canvas width 640 and default configuration:

| Direction | Canvas | Period (x,y,w,h) | Event (x,y,w,h) | Link |
| --- | --- | --- | --- | --- |
| LR/default | 640 x 296 | 206,118,190,48 | 206,184,190,30 | 301,166 -> 301,184 |
| TD | 640 x 176 | 234,50,180,48 | 450,50,140,30 | 414,74 -> 450,65 |

LR's final spine is 301,106 -> 301,106. TD's leading spine is
206,66 -> 206,90. Period/event labels are `P`/`E`. TD event width is
`640 - 100 - 350 - 50 = 140`; do not copy an inferred width from a returned
layout. Expected painter instructions are declared before production calls.
The separate TD oracle prevents accidentally making every header LR.

Run meaningful focused red tests against unchanged production, then the exact
candidate's full affected tests, doctests and all-target strict lint. Derive the
affected set from every current Rust manifest and all dependency kinds: the
preliminary enum-root set has 14 reverse consumers and 50 forward inputs, not
merely the parser and painter. Preserve any existing ignored tests explicitly;
no new skips, allowances, weakened safety guards or registry dependencies.

Security review must inspect the exact candidate and retained-source diff and
independently audit actual command exits and artifact/source identities before
publication. Publish only to the dynamically rediscovered sole PR. Auto-merge
stays off while conflicts, missing checks or review concerns remain. This slice
does not establish Apple runtime, full Closure parity, transformation lineage,
current-main integration or Miri acceptance.
