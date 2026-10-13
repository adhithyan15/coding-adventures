# HL46 — Direct owners for Human Language Data feature contracts

**Status:** specification, 2026-10-13

**Extends:** HL22's direct-document ownership model. Tracks #17448 and the
parent contention program #13193.

## 1. Problem

The Human Language Data package README combines its stable package contract
with more than one thousand lines of feature-specific operating contracts.
Independent work on assessment inventories, writing evidence, modality,
generated books, figures, and script filmstrips therefore converges on one
file. The HL42 report at `10d3ecd2cbfe9a72058848b792036839e370b57e`
measured 18 touching commits in its newest 200 scoped first-parent commits.
Eleven recent edits landed in the figure and script-filmstrip narrative alone.

## 2. Stable README boundary

`code/packages/typescript/human-language-data/README.md` continues to own only
the package-wide contract:

- purpose, installation, and the stable authoring workflow;
- basic API and command usage;
- the package-wide validation boundary; and
- the package scope note.

It links once to `docs/feature-contracts/`, but does not list feature owners or
summarize each feature. The directory listing is the discovery mechanism; a
table of contents in the README would recreate the shared edit surface.

## 3. Feature-contract owner documents

Each independently reviewable feature contract lives in one regular Markdown
file under:

```text
code/packages/typescript/human-language-data/docs/feature-contracts/
```

Names use lowercase ASCII words separated by hyphens and end in `.md`. The
directory has no `README.md`, `_meta.md`, index, manifest, generated aggregate,
or hand-maintained registry. Adding a feature creates one file and changes no
existing document. Refining a feature edits only its owner file.

The directory must be a real directory rather than a symbolic link. The
lowercase basenames `readme`, `index`, and `manifest` are reserved so a valid
looking owner filename cannot recreate a shared index. Every owner is a regular
non-symbolic-link file with exactly one CommonMark level-1 heading.

## 4. Ownership granularity

The migration preserves the existing prose, commands, links, failure
semantics, and safety boundaries while promoting each feature heading to its
owner document's level-1 heading.

The figure narrative is deliberately finer-grained than its old README
section. Generic figure generation, generic filmstrip derivation, placement,
script inventory closure, and independently evolving writing-system adoption
notes have distinct owners. Tamil, Gujarati, Devanagari, Bengali and Telugu,
Malayalam, Kannada and Chinese, Gurmukhi, and the Latin family therefore no
longer collide when one source or coverage count changes.

No feature-specific replacement monolith is allowed. An owner should be split
again when its independently edited concerns begin to converge.

## 5. Anti-regression evidence

A dedicated test discovers owner documents from the filesystem rather than a
registry. It fails closed when the root is a symbolic link or non-directory,
when a child is not a regular file, when a child is a symbolic link, when a
name is unsafe or reserved, when an owner does not contain exactly one
CommonMark level-1 heading, or when feature sections return to the shared
README.

The test pins only a baseline subset of migrated owners, so future additions do
not edit the test. It also copies the directory to a disposable location and
adds two valid owners. Both must be discovered while the README remains
unchanged and no metadata file is created. That is the executable proof that
parallel feature work has distinct file owners.

## 6. Acceptance

Completion requires:

1. the existing feature narratives are preserved in direct-owner documents;
2. the README retains stable package-wide guidance and only one directory link
   for feature contracts;
3. the hot figure and filmstrip narrative is split at independent script and
   subsystem boundaries;
4. adding two unrelated owners touches two files and no shared metadata;
5. the focused ownership test, full Human Language Data suite, TypeScript build,
   and coverage gate pass;
6. doc-shard and lessons validation pass; and
7. an independent security/data-integrity review reports no unresolved issue
   before push.
