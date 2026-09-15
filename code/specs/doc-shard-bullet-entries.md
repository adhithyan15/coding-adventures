# Bullet Entries for the Document Sharder

## Goal

`doc-shard` splits a Markdown document at ATX headings — `headingLevel: 2 | 3`.
That covers every document currently in `DOC_SHARD_PLANS`, and it cannot cover
the next one.

`code/specs/data/adj-facts-stdlib/CHANGELOG.md` is the repo's second-worst
conflict generator by time-clustered contention (63 close touches of 111 over
21 days, behind only the now-sharded `lang-aot/CHANGELOG.md`). It is 10,541
lines with exactly **one** `##` heading — `## Unreleased` — above **323**
top-level `- ` entries, each several paragraphs long.

Measured rather than described:

| split | sections |
|---|---:|
| `splitDocument(text, 2)` | **1** |
| `splitDocument(text, "bullet")` | **323** |

So the existing tool would emit one shard containing the whole document.
`docShardContents` refuses that outright, which is the correct behaviour and
also a dead end.

## Model

`DocShardPlan` gains an optional field:

```ts
readonly entryShape?: "heading" | "bullet";
```

Absent — as it is in all existing plans — nothing changes. When `"bullet"`, an
entry starts at a `- ` in the **first column**, and `headingLevel` is not
consulted: the preamble becomes everything above the first top-level bullet,
which carries the `# Title` and the lone `## Unreleased` along with it (161
bytes, for the target file).

Everything downstream is untouched. `splitDocument` already works in line
indices, so only its definition of "a line that starts a section" moves; the
preamble slice, the newline-preserving `lineRange`, the round-trip assertion,
the rank/slug/digest filename and the join all behave identically.

### One pattern, two callers

`splitDocument` and the shard-body check in `readDocShards` each previously
built their own `headingPattern(plan.headingLevel)`. With a second entry shape
that duplication is a place for the writer and the reader to disagree — the
sharder would emit bullet shards that the validator then rejected for not
starting with a heading. Both now call one `entryStartPattern(at)`.

### Two things that are not entries

- **An indented `- `** is a nested list item belonging to the entry above it.
  Splitting there would cut one entry in half and file the halves under
  different names. Only column 0 counts.
- **A `- ` inside a fenced block** is YAML, a diff hunk, or shell output.

The fence hazard is the dangerous one, and the module header already explains
why: a partition is byte-exact *no matter where you cut it*, so the round-trip
check passes while the shards are nonsense. Fence tracking is the one piece a
`--check` can never catch on its own. Bullet mode reuses the existing tracker
rather than adding a second one.

## Verification

- Byte-for-byte round trip on the **real** 10,541-line target, through the plan
  API, before any migration commits to it.
- A test asserting the heading split yields exactly one section — the reason
  this mode exists, asserted rather than asserted about.
- The default is pinned: a plan without `entryShape` still splits on headings.
- Mutation-tested. Allowing indented bullets to start an entry fails the nested
  -list test; dropping fence tracking fails three tests including the bullet
  one.

## Non-goals

This adds the capability and no plan. `adj-facts-stdlib` is migrated
separately, because that change must also extend `doc_shard_globs`,
`tracked_doc_monoliths` and `.gitignore` — and those three live in files that
other in-flight sharding PRs are editing, so combining them would manufacture
the conflict this whole effort removes.
