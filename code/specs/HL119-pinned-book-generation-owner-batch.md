# HL119: Pinned book-generation owner batches for the full shard check

## Problem

The all-plan `shard-cli --check` remains close to its unchanged 30-second
Windows test budget (#18957). On the 63,257-lesson checkout, a direct check
passed in 22.24 seconds, but previous covered full-suite runs timed out under
contention. Phase timing attributes about 5.4 seconds to reconstructing the
12,358 `core/book-generation.d/targets.d` owners; cross-ledger identity and
filename comparisons account for only a small fraction of that phase.

## Contract

* The full `--check` may read the book-generation owner tree through a bounded
  batch transport. The existing synchronous `readBookGenerationOwners` API and
  single-plan `--check` keep their behavior.
* Pin the trusted root and each descendant directory handle before listing or
  reading. Refuse a resurrected monolith, missing or extra top-level entries,
  symlink/junction/reparse owner directories and files, malformed names, and
  non-regular files. In particular, a FIFO must fail without a blocking open;
  the child transport also has a finite deadline. A path swap after
  prevalidation must not redirect an owner read outside the checkout. Keep at
  most sixteen owner files open per section.
* Return raw UTF-8 owner bytes and exact relative names. TypeScript remains the
  authoritative parser and applies the same metadata, record-name/identity,
  canonical order, duplicate, cross-ledger, and source-path checks as the
  synchronous reader. No generated summary or owner-derived expected set may
  replace an independent registry, chapter, or hash ledger.
* Prove batch/sync document and source-path parity on a fixture and the real
  corpus. Exercise missing, extra, corrupt, wrong-identity, link, and swapped
  directory adversaries. Retain the all-plan check and its 30-second test
  budget, complete corpus, >=85% line-coverage gate, and published bytes.
* Verify focused tests, the full unchanged Windows coverage suite, and
  terminal exact-head CI before the change is merge-ready. A direct standalone
  timing improvement alone is not acceptance.

## Out of scope

This does not alter books, lessons, the pre-A1-to-C2 ramp, writer behavior, or
other shard plans. Any further curriculum or chapter I/O optimization requires
separate parity and security proof.
