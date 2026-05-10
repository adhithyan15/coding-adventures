# PR70 - Prolog Dialect Profile Convergence

## Overview

The Prolog-on-Logic-VM stack now has enough functionality that the next risk is
fragmentation: ISO, SWI, GNU, Scryer, Trealla, XSB, YAP, and Ciao cannot all be
handled by scattered `if dialect == ...` checks in individual packages.

This batch introduces a shared dialect profile model and routes the loader and
VM compiler through that model. The goal is convergence: one explicit policy
object should describe a dialect's grammar files, default operator family,
string policy, module policy, and currently enabled extensions.

## Scope

This PR adds:

- `DialectProfile` in `prolog-core`
- `known_dialect_profiles()`, `dialect_profile(...)`,
  `loader_dialect_profiles()`, `iso_dialect_profile()`, and
  `swi_dialect_profile()`
- tracked profile metadata for ISO/Core, SWI, GNU, Scryer, Trealla, XSB, YAP,
  and Ciao
- generic `load_prolog_source(...)`, `load_prolog_file(...)`,
  `load_prolog_project(...)`, and `load_prolog_project_from_files(...)`
- generic `compile_prolog_source(...)`, `compile_prolog_file(...)`,
  `compile_prolog_project(...)`, and `compile_prolog_project_from_files(...)`
- generic source/file/project runtime constructors in `prolog-vm-compiler`
- ISO/SWI end-to-end tests that compile and query through the same VM path

The existing SWI-specific wrappers remain public compatibility conveniences.

## Non-Goals

This PR does not claim that every tracked dialect is executable today.

Only `iso` and `swi` are marked as loader-supported. GNU, Scryer, Trealla, XSB,
YAP, and Ciao are represented as explicit future targets so later work can fail
closed with useful profile metadata instead of silently treating all dialects as
SWI.

This PR also does not add tabling, attributed variables, coroutining, foreign
predicates, stream I/O, or dialect-specific syntax such as SWI dicts.

## Semantics

`DialectProfile` is data, not a subclass hierarchy. A profile records:

- stable profile name and aliases
- token grammar path
- parser grammar path
- default operator-table family
- double-quote policy
- module policy
- whether the current loader can execute that dialect
- named extension flags

Loader and compiler entry points resolve aliases such as `swipl` and
`iso_core`, then dispatch to the implemented parser route. Unsupported but
tracked dialects raise a clear `ValueError`.

## Why This Is A Convergence Batch

The parser packages already have separate ISO and SWI lexer/parser packages.
The VM compiler still exposed mostly SWI-shaped top-level constructors. That
made SWI the accidental default even when the caller wanted ISO/Core behavior.

The new generic entry points let future dialect work plug into one path:

```text
DialectProfile
  -> dialect parser/loader
  -> loaded source/project
  -> Logic VM compiler
  -> stateful runtime
```

Later batches can add operator/directive depth, parser-to-instruction lowering,
module registry improvements, or tabling without inventing a new public routing
surface each time.

## Test Strategy

Coverage includes:

- profile registry and alias resolution tests in `prolog-core`
- loader routing for source strings, files, projects, and file graphs
- explicit failure for tracked-but-unimplemented dialects
- compiler routing for ISO and SWI source strings
- ISO runtime queries through the generic VM runtime path
- preservation of existing SWI wrappers and stress tests

## Follow-Up Batches

Recommended next convergence batches:

- parser-to-instruction lowering as the default frontend-to-VM boundary
- stronger module/predicate registry semantics keyed by dialect policy
- stream/file I/O and ISO-style error-term normalization
- attributed variables and generic constraint stores
- tabling as a distinct solver backend for XSB/YAP/SWI-compatible modes
