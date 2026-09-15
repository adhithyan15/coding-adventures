---
category: Repo policy / workflow reminders
---

# A generated directory's cleanup step will delete the hand-written file living beside its output

`migrate_lessons_to_shards.py` began with the ordinary idempotence idiom:

```python
if TARGET.exists():
    shutil.rmtree(TARGET)
TARGET.mkdir()
```

`lessons.d/_meta.md` is hand-written and lives in that same directory, because
it is the entry point for the shards around it. The first run of the migration
destroyed it. It had not been committed yet, so it was simply gone — no git
object, no backup, nothing to restore from.

`rmtree` on a directory you only PARTLY own is the bug. The script generates
`*.md` shards; it does not own `_meta.md`, and "clear the output directory" was
never the same statement as "clear my output".

```python
TARGET.mkdir(exist_ok=True)
for stale in TARGET.glob("*.md"):
    if stale.name != "_meta.md":
        stale.unlink()
```

**How to apply.** Before a script deletes anything, ask what else lives at that
path. If a generated directory has a hand-written index, a README, or a
`.gitkeep` in it, a wholesale wipe takes those too. Delete by the pattern you
WRITE, never by the directory you write into.

This is also the repo's standing rule — never force-delete, always leave things
retrievable from git — reached from the other direction: the rule is usually
read as being about `git rm -f` and `rm -rf` typed at a prompt, but a migration
script is where it actually bit.
