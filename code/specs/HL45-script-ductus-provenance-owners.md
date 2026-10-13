# HL45 — Direct owners for Script Ductus provenance

**Status:** specification, 2026-10-13

**Extends:** HL27's writing-system ownership boundary. Tracks #17442 and the
parent contention program #13193.

## 1. Problem

Script Ductus already owns source, executable evidence, registry pins, and
release notes below writing-system-specific paths. Its package README remains a
shared authoring surface: eight long source narratives for unrelated writing
systems live between the stable citation rule and the stable usage guide.
Adding or refining a source therefore edits the same file as every other
writing-system author.

The HL42 report at `5065225bab7840cf20372f7e12cff1dffb8ca75b` measured 20
touching commits for that README in its newest 200 scoped first-parent commits,
including 14 touches since 2026-10-07.

## 2. Stable README boundary

`code/packages/typescript/script-ductus/README.md` continues to own only the
package-wide contract:

- purpose and position in the stack;
- the font-outline validation model;
- the no-citation/no-pen-path rule;
- usage and the generated filmstrip-ledger workflow;
- the DOM and filesystem security boundary; and
- package and test guidance.

It links once to `docs/stroke-sources/`, but it does not list source owners,
scripts, source titles, glyph ranges, or source-specific exceptions. Those
facts would turn the link block into another shared index.

## 3. Provenance owner documents

Each independently reviewed provenance narrative lives in one regular Markdown
file under:

```text
code/packages/typescript/script-ductus/docs/stroke-sources/
```

Names use lowercase ASCII words separated by hyphens and end in `.md`. The name
identifies the writing-system owner and the source, for example
`bengali-lipitk.md` or `malayalam-moag.md`. A source used by more than one
script remains one owner document when its provenance and interpretation are
reviewed as one unit.

The directory itself is the discovery mechanism. It has no `README.md`,
`_meta.md`, manifest, generated aggregate, or hand-maintained index. Adding a
new provenance source creates one new file and changes no existing document.
Refining an existing source edits only that source's owner file.
The directory must itself be a real directory rather than a symbolic link, and
the lowercase basenames `readme`, `index`, and `manifest` are reserved so an
apparently valid owner filename cannot recreate the forbidden shared index.

Every owner file:

1. is a regular file rather than a symbolic link;
2. has a strict lowercase owner/source filename;
3. starts with one level-1 heading;
4. records the evidence, its licence or reuse boundary, and how the authored
   path interprets that evidence; and
5. preserves any confidence limits, exceptions, or attribution obligations.

## 4. Migration

The eight existing level-3 source sections move out of the README without
dropping their factual prose, source links, licence statements, confidence
limits, or documented exceptions. Their headings become the level-1 headings
of direct-owner files. The stable citation rule remains in the README.

This is a documentation ownership migration. It does not change stroke data,
citations, public APIs, generated ledgers, package exports, or validation
tolerances.

## 5. Anti-regression evidence

A dedicated test discovers owner documents from the filesystem rather than a
registry. It rejects unsafe names, non-files, symbolic links, a directory-local
index, missing level-1 headings, and a return of owner-specific provenance
subsections to the package README.

The test also copies the owner directory to a disposable location and adds two
different valid owner documents. Both must be discovered without changing the
README or any metadata file. That is the executable proof that parallel source
owners do not converge on a replacement aggregate.

## 6. Acceptance

Completion requires:

1. all eight existing source narratives are preserved in direct-owner files;
2. the README retains its stable package-wide guidance and only one directory
   link for source narratives;
3. adding two unrelated valid owners touches two distinct files and no shared
   metadata;
4. the focused ownership test, full Script Ductus suite, TypeScript build, and
   coverage gate pass;
5. doc-shard and lessons validation pass; and
6. an independent security/data-integrity review reports no unresolved issue
   before push.
