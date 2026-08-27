# HL24 — Parallel script-test ownership

**Status:** specification, 2026-08-27

**Extends:** HL21 and HL22, which remove shared curriculum-data and prose
ledgers. This specification applies the same ownership rule to executable
corpus evidence.

---

## 1. Problem

Sharding the authored data does not enable parallel work if every author must
still edit one shared test. The real-corpus integration suite accumulated exact
stroke-order, source, variation, pen-lift, and closure assertions for every
writing system inside:

```text
code/packages/typescript/human-language-data/tests/integration.test.ts
```

After the global BACKLOG/CHANGELOG monolith fix merged, 32 human-language
commits touched that file. Japanese, Urdu, Persian, Kannada, Telugu, Malayalam,
and Tamil changes were independent in the data tree but serialized again at
the test layer.

The conflict has no semantic value. A Japanese hiragana proof and an Urdu
Nastaliq proof do not need a shared edit, review, or merge decision.

## 2. Ownership boundary

Exact inventory evidence belongs to the inventory it proves:

```text
tests/script-inventories/devanagari.evidence.ts
tests/script-inventories/japanese.evidence.ts
tests/script-inventories/kannada.evidence.ts
tests/script-inventories/malayalam.evidence.ts
tests/script-inventories/perso-arabic.evidence.ts
tests/script-inventories/tamil.evidence.ts
tests/script-inventories/telugu.evidence.ts
tests/script-inventories/urdu-nastaliq.evidence.ts
```

`integration.test.ts` retains only genuinely cross-corpus evidence: the whole
corpus validates, every registered track loads, shared spine/curriculum/book
relationships close, and cross-language queries join correctly. It also owns a
stable eager glob that discovers every `*.evidence.ts` module and invokes it
against the already-loaded corpus. Adding a script therefore never requires an
edit to the integration gate or to a registry.

An inventory file may assert a relationship with a sibling inventory when the
same encoded glyph genuinely spans both. That is semantic coupling, not a
mechanical reason to put every script in one file.

## 3. Shared measurement, separate claims

The uncovered-glyph report is a corpus-wide computation. Its implementation
belongs in one test helper, while each inventory owns the assertions about its
own glyphs. The helper returns:

- missing characters grouped by inventory filename; and
- the number of lessons affected by each missing character.

It must use the production validator rather than duplicating its closure
algorithm. The integration gate computes it once and passes the result to each
inventory evidence module. This preserves separate edit ownership without
repeating the expensive `loadEverything()` corpus load for every script.

## 4. Coverage-preservation rule

This migration is structural. Every assertion present before the split must be
present afterwards. It may move into an inventory-owned file or into the common
measurement helper, but it may not be weakened, replaced with a snapshot, or
deleted merely because the split exposes duplication.

The proof is threefold:

1. the existing real-corpus integration target, including its evidence glob;
2. the package's `npm run validate` command, which names that target directly;
   and
3. the full package suite at its default timeouts.

Evidence discovery uses a stable `import.meta.glob` in `integration.test.ts`.
No hand-maintained file registry may replace one shared edit with another, and
evidence modules are deliberately not separate `*.test.ts` files: that would
reload and revalidate the complete corpus once per inventory.

## 5. Future inventory sharding

The JSON inventories themselves remain a later conflict-removal tranche. When
they become `X.d/` ledgers, their corresponding evidence filename remains
stable: agents add an inventory entry shard and edit only that inventory's
evidence module. A
future data-driven source/provenance schema may remove many exact assertions,
but it must fail closed with evidence at least as strong as the assertions it
replaces.

Tracking: #13191. Parent backlog: #13193.
