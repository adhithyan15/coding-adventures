# HL28 — Stable Tamil glyph ownership

**Status:** specification, 2026-08-27

**Extends:** HL25's shard-native script inventories and HL27's writing-system
owner modules. Tracks #13303, the parent contention program #13193, and the
Indian-language C2 prerequisite #13295.

## 1. Outcome

Two agents correcting different existing Tamil glyphs must not edit the same
canonical-inventory, Ductus-source, stroke-evidence, or filmstrip-evidence
file. Tamil therefore uses one stable ASCII code-point identity across those
four layers instead of treating the whole writing system as the smallest owner.

This is an ownership migration, not a curriculum or handwriting-data change.
It preserves the logical Tamil inventory, global Ductus registry, citations,
geometry, rendering, public exports, and Language Ladder bundle behavior.

## 2. Stable identities

Tamil owners use uppercase Unicode scalar filenames such as `U-B85` for `அ`.
Filenames remain ASCII so filesystem normalization cannot silently create two
spellings of one owner. A shard must prove that its filename identity matches
its decoded glyph.

The canonical inventory keeps letters and marks in distinct directories and
uses spaced numeric ordinals to preserve the existing authored order. Ductus
source and evidence use the same `U-<CODEPOINT>` identity without a second
hand-maintained glyph manifest.

## 3. Canonical inventory ownership

The editable `data/scripts/tamil.json` monolith becomes a shard directory whose
folded value is exactly equal to the pre-migration JSON object. One metadata
shard owns script-wide fields; each existing letter or mark owns one data shard
and one evidence module.

All readers must consume the same logical inventory through the supported shard
fold. TypeScript build plugins, Python authoring tools, and generators may not
introduce a second editable Tamil source of truth. Generated provenance keeps
the logical name `tamil.json`; the storage migration must not churn learner
data merely because the physical representation changed.

Ordinary edits to an existing letter or mark must not touch a shared manifest,
aggregate evidence file, or whole-inventory mutable-data hash. Shared gates may
pin shape, owner discovery, ordered identities, and counts.

## 4. Ductus ownership

`src/strokes/tamil.ts` is an infrastructure-owned assembly boundary. Each
authored record lives in:

```text
src/strokes/tamil/U-<CODEPOINT>.ts
```

and exports exactly one `[key, LetterDuctus]` entry. The assembly module imports
those entries in the existing order and must not contain native Tamil literals,
stroke coordinates, citations, `strokes:`, or `source:` records.

The historical Tamil placement remains observable. Entries that were in the
main Tamil owner sequence keep that position; the historical tail entry keeps
its later global position. Assembly must reuse each imported object rather than
clone it, and the global duplicate/prototype-safety contract from HL27 remains
in force.

## 5. Evidence ownership

Glyph-specific evidence is discovered under matching paths:

```text
tests/strokes/tamil/U-<CODEPOINT>.test.ts
tests/ductusview/tamil/U-<CODEPOINT>.test.ts
tests/script-inventories/tamil/U-<CODEPOINT>.evidence.ts
```

Every pre-migration assertion remains present. Shared helpers may provide
mechanics, but owner claims may not be registered through a central mutable
assertion list. Discovery gates compare source and relevant evidence identities,
reject filename/glyph mismatches and duplicates, and keep the old Tamil evidence
roots absent.

Exact per-glyph data hashes belong with the glyph they prove. A mutable global
serialized-data hash is forbidden because every legitimate glyph edit would
recreate the shared-file collision this specification removes. Shared migration
gates may retain global ordered-key and count pins.

## 6. Browser bundle boundary

Language Ladder imports Script Ductus TypeScript source through a `file:`
dependency. Its manual chunk rule must recognize bounded descendants beneath
`script-ductus/src/strokes/` on POSIX and Windows paths so per-glyph modules stay
in the existing handwriting-tools chunk. The path classifier is a pure tested
boundary rather than an untested inline regular expression.

## 7. Migration proof

Before removing the Tamil monoliths, record and verify:

- exact folded inventory equality: 25 letters, 9 marks, object SHA-256
  `ccc54a71df05146f36cdf78ade1e81c79124703ff763fbb36347839d64ebd339`,
  and ordered-identity SHA-256
  `12edac447f5a0ae29f2b89a88d326b7893cbd7176ac1809c9dffad1f1f260421`;
- exact global Ductus equality: 330 keys, Tamil count 25, ordered-key
  SHA-256
  `dfbc3a4264318948f47cd52a076282f03e69ce64dfbb98e2145a7c5fa8896542`,
  and serialized-data SHA-256
  `482c15657edc14bc02f3a07e7493f03202a4f5a7786125e9fa3fe309d25e7ffb`;
- exact Tamil Ductus entry identity and order, including the sourced short `ஒ`;
- every pre-migration Tamil stroke and view assertion;
- filename-to-glyph identity and one-to-one source/evidence discovery;
- no duplicate ordinal, glyph, owner, or registry key;
- no symlink escape from the shard roots; and
- unchanged public exports, shared object identities, and browser chunk
  placement.

The full Human Language Data, Script Ductus, and Language Ladder builds must
pass. An independent reviewer must compare the final runtime data with the
pre-migration baseline and audit duplicate/prototype handling before push.

## 8. Acceptance

1. existing glyph agents own disjoint inventory, source, and evidence paths;
2. assembly roots contain no authored glyph data or claims;
3. inventory and Ductus order/data are unchanged by the migration;
4. discovery is automatic for evidence and bounded for runtime assembly;
5. malformed, duplicate, mismatched, hostile, or escaping shards fail closed;
6. downstream TypeScript and bundle boundaries accept nested owners; and
7. this prerequisite lands before broad Indian-language C2 content rewriting.
