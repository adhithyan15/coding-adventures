---
category: Mosaic compiler pipeline
---

# XAML expression tests must use supported source syntax

An authored-row-name regression initially used `row-index + 16`, but the XAML
expression tokenizer rejects arithmetic. The application supplies row labels
as a list indexed by the local row instead. Exercise that supported list/index
expression in compiler tests, and change the owner list to test viewport shifts.

The full emitter suite also reads sibling Mosaic application fixtures. A sparse
checkout containing only VisiCalc failed on missing TaskApp.mil; include
code/programs/mosaic before running package integration tests.

Review found that lowering a computed label alone does not register outer
projection dependencies. Register SlotRef and Expr dependencies explicitly; test
a label-only slot callback as well as retained-cell refresh, since manually
reading the projection can mask a missing notification path.
