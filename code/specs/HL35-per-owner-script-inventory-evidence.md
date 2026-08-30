# HL35 — Per-owner script-inventory evidence

## 1. Outcome

Japanese, Perso-Arabic, Tamil, and Urdu-Nastaliq retain static mutation
evidence without asking every glyph addition to rewrite one whole-inventory
digest. This closes #13449 after HL34 establishes the independent runtime owner
set.

The static evidence is a second, independently stored projection. It does not
replace HL34 declarations, and `check:shards` does not consult it. HL34 proves
which owners must exist; HL35 proves that the canonical bytes of each owner
still match the reviewable evidence accepted for that owner.

## 2. Stable ownership

Evidence lives under a static data tree, one file per stable code-point identity:

```text
data/script-owner-evidence/<script>/
  letters/U-<CODEPOINT>[-U-<CODEPOINT>...].json
  marks/U-<CODEPOINT>[-U-<CODEPOINT>...].json
```

Each evidence file contains exactly the fixed script, the section kind, the
embedded glyph or mark, and the SHA-256 digest of that inventory owner's
canonical bytes. Filename, section, body identity, and inventory owner must all
agree. Evidence and declarations use separate directories and separate readers.

Adding one glyph changes three disjoint stable paths: its inventory owner, its
HL34 declaration, and its HL35 evidence. Two additions to the same script do not
touch one shared expected table, digest, manifest, or test line.

## 3. Exact static comparison

The focused test reads the four fixed inventory roots and four fixed evidence
roots. For every script it requires:

1. the evidence letter identity set exactly equals the reconstructed inventory
   letter identity set;
2. the evidence mark identity set exactly equals the reconstructed inventory
   mark identity set;
3. every evidence body's glyph or mark hashes to its filename identity;
4. every referenced inventory owner has the same identity and canonical bytes;
   and
5. the SHA-256 digest of those bytes equals the tracked per-owner digest.

A clean owner deletion leaves evidence behind and fails exact equality. A clean
evidence deletion leaves an inventory owner without evidence and also fails.
Mutation fails the byte digest. Duplicate, case-fold-colliding, cross-kind, and
filename/body mismatches fail before comparison.

## 4. Read boundary

The evidence reader is read-only in check and test mode. It uses fixed language
and script configuration and rejects:

- unsafe or Windows-reserved slugs;
- missing, empty, extra, nested, symlinked, or non-regular roots and entries;
- malformed or case-fold-colliding filenames;
- malformed JSON and dangerous keys;
- extra or missing body keys, non-canonical bytes, and cross-script or
  cross-kind bodies; and
- filename/body code-point mismatches.

As with the other strict corpus readers, `lstat` and `realpath` reject links and
escapes at each opened component. This closes ordinary repository-controlled
path substitution; it does not claim a filesystem transaction against a
privileged concurrent process that can swap entries after inspection.

## 5. Authoring command

The package exposes a deterministic writer for migration and new owners. It
refuses unknown scripts and unsafe roots, writes canonical JSON, and changes
only the evidence files whose inventory bytes changed. CI and focused tests use
the read-only checker; they never repair missing evidence.

The initial migration creates one evidence file for each of the 146 owners at
the HL34 boundary:

| Script | Letters | Marks | Total |
|---|---:|---:|---:|
| Japanese | 49 | 3 | 52 |
| Perso-Arabic | 24 | 1 | 25 |
| Tamil | 27 | 9 | 36 |
| Urdu-Nastaliq | 31 | 2 | 33 |

## 6. Acceptance

Completion requires:

1. the mutable whole-inventory digests and counts leave
   `tests/script-shards.test.ts`;
2. all 146 current owners have separate canonical evidence files;
3. a normal single-owner addition needs no shared-file update;
4. clean deletion on either side, byte mutation, duplicate/case-fold identity,
   filename/body mismatch, malformed JSON, dangerous keys, traversal,
   symlinks, nesting, and non-regular entries have focused coverage;
5. the existing shard reconstruction and aggregate-removal assertions remain;
6. build, shard/doc checks, the focused and full package suites, Script Ductus,
   and Language Ladder validations pass; and
7. security review finds no unresolved trust-boundary defect before publication.
