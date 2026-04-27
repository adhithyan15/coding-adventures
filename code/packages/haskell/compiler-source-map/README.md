# compiler-source-map

Source mapping chain for compiler pipelines.

## What it does

- tracks `source -> AST -> IR -> machine code` relationships
- supports forward lookup from source positions to machine code spans
- supports reverse lookup from machine code offsets back to source
- preserves optimiser-pass mappings so compiler debugging stays explainable

## Status

This Haskell package now provides a real source-map chain with tests covering segment lookups and end-to-end composition.
