---
category: Repo policy / workflow reminders
---

# Changelog shard ranks chosen on a prep branch can collide with a PR that merges first

The Flutter ENV4 change (UI48 §7.8) was prepared on a local branch while the
XAML platform library (#16302) was still in CI. Both added a
`mosaic-app-bindings/CHANGELOG.d/` shard. The prep branch checked that its
rank, 00280, was free **on main at the time**. #16302 merged first with 00280
and 00290, and cherry-picking the prep commits onto the new main produced two
`00280-…` shards. The files have different names, so git reported no
conflict, and nothing failed. The duplicate only showed up because the
changelog directory was listed before pushing.

**Fix:** `git mv` the later shard to the next free rank (00300). The rank is
not part of the heading digest, so the content stays the same.

**Do instead:** after rebasing or cherry-picking any branch that adds a
changelog shard, list the directory and look for a repeated rank before
pushing:

```bash
ls CHANGELOG.d | cut -c1-5 | sort | uniq -d   # must print nothing
```

Choose ranks at push time, not at prep time. When several prep branches stack,
pick ranks with a gap between them.
