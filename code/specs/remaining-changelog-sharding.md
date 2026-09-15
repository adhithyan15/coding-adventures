# Sharding the Remaining Contended Package Changelogs

## Goal

Seven package changelogs at once, using the mechanism from
[`lang-aot-changelog-sharding.md`](lang-aot-changelog-sharding.md) and the
bullet entry shape from
[`doc-shard-bullet-entries.md`](doc-shard-bullet-entries.md).

## Why a batch, when the convention is one per PR

`DOC_SHARD_PLANS` says it "grows one entry per follow-on PR", and the first
four migrations followed that. It stops being the right rule here, for a reason
worth stating plainly:

**Every sharding PR edits the same three shared lists** — the registry,
`doc_shard_globs`, and `tracked_doc_monoliths`. So sharding PRs conflict with
*each other* and must land strictly one at a time. The work of removing
shared-file contention is itself serialized on a shared file, one level up from
the problem it solves.

With one document left that was tolerable. With seven it is the bottleneck, and
each costs a full CI matrix and a merge window.

The reviewable surface does not grow proportionally. A migration's
human-checked part is a plan row and three list entries; the shards are
generated and verified by byte-for-byte round trip, which the registry-driven
test loop runs per plan automatically. Seven plans means seven round trips, not
seven judgement calls.

## The seven, measured

`newestFirst` is established for each from **where commits actually insert**,
not copied from a neighbouring plan:

| document | mode | entries | last 6 commits insert at | lines |
|---|---|---:|---|---:|
| `mermaid-parser/CHANGELOG.md` | heading 2 | 166 | line 3, ×6 | 674 |
| `wasm-conformance/CHANGELOG.md` | heading 2 | 135 | line 3, ×6 | 4,995 |
| `diagram-ir/CHANGELOG.md` | heading 2 | 97 | line 3, ×6 | 399 |
| `diagram-to-paint/CHANGELOG.md` | heading 2 | 53 | line 3, ×6 | 322 |
| `spice-netlist-parser/CHANGELOG.md` | bullet | 182 | line 5, ×6 | 782 |
| `venture-browser-core/CHANGELOG.md` | bullet | 57 | line 5, ×6 | 196 |
| `oauth-broker/CHANGELOG.md` | bullet | 31 | line 5, ×6 | 151 |

Every one of the last six commits to each file inserts at the same line — the
same direct evidence that justified `adj-facts-stdlib`, reproduced seven times.

### Two shape notes, confirmed by round trip rather than reasoned about

- **`wasm-conformance`** has 135 `##` *and* 143 `###`. Split at level 2; the
  level-3 subsections ride inside their parent shard, the documented behaviour
  and the same choice Language Ladder made.
- **The three bullet documents** each retain a few `##` version markers (10, 9,
  and 1). Under bullet mode those attach to the end of the preceding entry's
  shard and return to the same place on rejoin — the "frozen version markers
  ride along" behaviour `human-language-data` already relies on.

## Readers checked first

The previous migration shipped two tests that read their target with
`readFileSync`, then made that target a generated gitignored file. They
ENOENTed on CI while passing locally, because an earlier `--unshard` had left a
rendered copy in the working tree — an ignored leftover that `git status` does
not mention.

So before turning seven more tracked files into generated ones, every committed
reference to each path was searched for, by full path and by relative
`](CHANGELOG.md` link from inside the owning package. **No tracked file
references any of the seven.** There are no readers to fix, which is a
measurement rather than an expectation.

## Deliberately excluded

The contention ranking also lists documents that are **not** entry lists:
`spice-full-implementation-plan.md` (5 `##` over 5,583 lines),
`RUST-CPU-SIMULATOR-BACKLOG.md`, `LANG-FULL-IMPLEMENTATION.md`, `oauth.md`, and
several READMEs.

Bullets in a spec are usually content inside an argument, not independent
appends. Sharding one would cut an argument in half, and their contention is
plausibly *semantic* — two people editing the same claim — which sharding does
not fix and would disguise. They stay whole until someone shows the conflicts
there are additive.

## A follow-on worth considering, not done here

The registry and the two guard lists remain hand-kept shared files. Deriving
the guard lists from `DOC_SHARD_PLANS` at CI time — rather than pinning
hand-written copies with a test — would remove the last shared surface this
effort keeps editing. That changes how the gate is built, not what it checks,
and belongs on its own.
