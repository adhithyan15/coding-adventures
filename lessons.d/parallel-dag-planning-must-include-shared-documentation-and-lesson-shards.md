---
category: Repo policy / workflow reminders
---

# Parallel DAG planning must include shared documentation and lesson shards

The LANG VM loop serialized independent streams through CI waits even though
the work could use separate packages and branches. The user clarified that a
second independent PR is authorized. A code-only ownership map would still
conflict through the shared backlog and existing lesson shards, so the DAG now
includes those writes and reserves them to their current branch. The parallel
Python stream uses its own spec/package docs, a dedicated DAG and this shard.

Preparation also tried an invented lessons_meta.md path and a flattened Python
grammar filename before using the discovered lessons.d/_meta.md and
code/grammars/python/python3.12.tokens. The first regression command ran cargo
from the checkout root, which has no Cargo.toml, and selected no tests because
it failed before compilation. It was rerun from code/packages/rust; that root
is part of the stream's execution plan, and only a captured test failure counts
as regression evidence.

The first transitive-dependency audit assumed the default `python` provided
tomllib. It was Python 3.10 and failed at import. `py -0p` identified Python
3.13, and the audit was rerun with `py -3.13`; the failed import did not count
as an overlap audit. Pin the validation runtime along with its working path.

Represent each work item with depends_on, owned code/document/lesson paths,
worktree, validation working directory and live PR/check state. Permit parallel
publication only after checking both file intersections and semantic
prerequisites against every open PR. Read-only shared dependencies are safe;
changes to them introduce an edge and require renewed downstream validation.
