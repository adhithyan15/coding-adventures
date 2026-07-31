# OCAML01 — Emerging implementation lane contract

Status: complete in PR #9323

## Purpose

OCaml is the repository's next implementation lane, but an empty directory or
one experimental package must not silently turn every existing portable package
into a new missing slot. This contract introduces OCaml as an explicitly known
`emerging_implementation` bucket while preserving the established
15-language denominator until the promotion gates in the package-parity roadmap
are reviewed and satisfied.

This tranche owns only classification and reporter behavior. Scaffolds, package
metadata, dependency resolution, capability analysis, CI toolchains,
representative packages, a native build tool, and denominator promotion remain
separate dependency-shaped items.

## Lane states

An implementation lane has exactly one of these parity states:

| State | Reporter class | Counts toward portable parity |
|---|---|---|
| Established | `implementation` | Yes |
| Emerging | `emerging_implementation` | No |
| Promoted | moved atomically from emerging to established | Yes |

OCaml begins in the emerging state alongside C and C++. Its packages are
inventory evidence, not denominator evidence.

## Reporter invariants

The package-parity reporter MUST:

1. recognize `code/packages/ocaml/<package>/...` as a known bucket;
2. include OCaml in `bucket_classes.emerging_implementation`, the special-bucket
   summary, package presence rows, and the CSV matrix;
3. exclude OCaml packages from the established implementation union, per-lane
   coverage, high-consensus counts, singleton counts, completion bands, and
   missing-slot totals;
4. keep the established denominator explicit in report metadata;
5. derive the upper bound of the high-consensus completion band from the
   established-language count instead of embedding `15` in report logic or
   Markdown; and
6. continue to fail on genuinely unknown package buckets and within-bucket
   canonical identity collisions.

An OCaml-only package therefore has an implementation `language_count` of zero
while its complete `languages` list contains `ocaml`. A package present in all
15 established lanes remains complete even when an OCaml directory exists.
The JSON additions are report schema version 3; consumers must not treat the
earlier version 2 shape as interchangeable. The CSV matrix remains
self-describing through its header, and adding a recognized bucket appends that
bucket's column in bucket-class declaration order. Consumers must select CSV
columns by header name rather than fixed position.

Only packages present in at least one established lane belong to a completion
band. The band-classification helper rejects a zero established-language count
instead of silently treating an emerging-only package as a singleton.

## Promotion boundary

Promotion is a reviewed, atomic contract change. It must:

- satisfy every package, build, security, documentation, and three-platform CI
  gate in the roadmap;
- move `ocaml` from `emerging_implementation` to `implementation`;
- update the established-language count from 15 to 16;
- cause the reporter's top completion band to become `10-16`;
- recompute all missing slots and the portable backlog; and
- classify every new 16-lane gap or reviewed exception before completion is
  claimed.

No scaffold, package, resolver, analyzer, build-tool, or CI change may promote
the lane implicitly.

## Conformance cases

Reporter tests MUST cover:

- an OCaml package is classified and never reported as unknown;
- an OCaml-only identity does not enter established counts or completion bands;
- adding an OCaml copy of an established package does not change its established
  `language_count` or missing slots;
- Markdown names the actual established denominator; and
- the completion-band helper produces `10-16` when evaluated against a
  hypothetical 16-language established denominator.
