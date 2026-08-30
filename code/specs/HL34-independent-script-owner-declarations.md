# HL34 — Independent script-owner declarations

## 1. Outcome

The Japanese, Perso-Arabic, Tamil, and Urdu-Nastaliq inventories retain their
one-glyph-per-file authoring boundary while `check:shards` gains an independent,
exact owner set. Deleting an otherwise valid inventory owner is therefore an
error instead of a smaller but still structurally valid inventory.

This closes #13381 and extends HL21's removed-monolith completeness contract and
HL25's shard-native script inventories. It does not claim that any of these
deliberately growing inventories is a complete Unicode repertoire.

## 2. Canonical ownership

Declarations live outside the inventory tree so they cannot be reconstructed
from the owners they prove:

```text
data/script-owner-declarations/<script>/
  letters/U-<CODEPOINT>[-U-<CODEPOINT>...].json
  marks/U-<CODEPOINT>[-U-<CODEPOINT>...].json
```

There is no script-sized metadata file, expected-id list, or digest. One
declaration owns one stable identity. Adding one glyph changes its inventory
owner and its matching declaration; two additions to the same script touch four
distinct files.

The four fixed declaration roots are:

| Language | Script | Inventory owners | Declaration owners |
|---|---|---:|---:|
| Japanese | `japanese` | 49 letters + 3 marks | 49 + 3 |
| Persian | `perso-arabic` | 24 letters + 1 mark | 24 + 1 |
| Tamil | `tamil` | 25 letters + 9 marks | 25 + 9 |
| Urdu | `urdu-nastaliq` | 29 letters + 2 marks | 29 + 2 |

The counts record the migration boundary, not permanent ceilings. The exact
identity equality is derived from the independently authored files.

## 3. Declaration shape and binding

A letter declaration contains exactly:

```json
{
  "language": "japanese",
  "script": "japanese",
  "kind": "letter",
  "glyph": "あ"
}
```

A mark declaration replaces `glyph` with `mark` and uses `kind: "mark"`.
The reader derives the code-point identity with the same `scriptEntryId`
algorithm as HL25 and requires it to equal the filename. The directory kind,
body kind, identity field, fixed language, and fixed script must all agree.
Letters and marks share one logical identity namespace, so the same code-point
sequence cannot be declared in both sections.

## 4. Strict read boundary

The declaration reader is deliberately separate from the permissive generic
shard reader. Before opening any declaration bytes it:

1. resolves a fixed declaration root beneath the configured curriculum root;
2. rejects symlinked path components, roots, section directories, and files;
3. requires exactly the direct `letters` and `marks` directories;
4. rejects nesting, non-regular entries, malformed or case-fold-colliding
   filenames, Windows-reserved identities, and unexpected files;
5. records the complete filename identity set; and only then
6. parses canonical JSON with dangerous-key rejection and binds every body to
   its path.

The reader is read-only. Check mode never creates a declaration, repairs an
owner, follows a legacy aggregate, or infers an absent declaration from the
surviving inventory.

## 5. Exact completeness

Each script `ShardPlan` replaces `structural-only` with a
`script-owner-declarations` source carrying its fixed language and script.
During `check:shards`:

- declaration letter identities must exactly equal reconstructed inventory
  letter identities;
- declaration mark identities must exactly equal reconstructed inventory mark
  identities; and
- missing, unexpected, duplicate, case-fold-colliding, or cross-kind identities
  fail with the affected script and section named.

The declaration set is established independently before inventory comparison.
The existing whole-inventory test hashes remain a separate static migration
defence until #13449 replaces their shared mutable evidence; they are not the
runtime completeness source introduced here.

## 6. Acceptance

Completion requires:

1. 142 canonical declarations reconstruct the exact four current owner sets;
2. all four script plans use exact declaration completeness and none remain
   `structural-only`;
3. focused tests cover deletion on either side, an unexpected owner, duplicate
   and case-fold identity failure, filename/body mismatch, malformed JSON,
   dangerous keys, traversal/reserved names, symlinks, nesting, and non-regular
   entries;
4. a test proves two additions to one script resolve to disjoint inventory and
   declaration paths;
5. `check:shards`, the Human Language Data build and suite, source-importing
   Script Ductus and Language Ladder builds, and affected-package validation
   pass; and
6. an independent security review finds no unresolved filesystem trust-boundary
   defect before publication.
