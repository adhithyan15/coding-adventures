### Kiln: the merge job's steps export is keyed by `rel_path`, not by package name

- `record.Merge` keyed the build tool's steps export (B6) through a
  name → path map, which was wrong: a steps entry's `package` field already
  holds the graph's `rel_path` (`legacy.Steps.Lines(relPath)` is the
  authoritative lookup). With the map, every real shard would have been
  refused with "steps export names ... which the plan does not hold", so the
  merge job could never have emitted anything.
- The plan is still consulted, for the thing it is actually for: an export that
  names something no shard runs is a broken input and is refused, as is a name
  that is not a valid Kiln path or one listed twice.
- Found by the new end-to-end test in the `kiln-sandbox` job, which is why that
  test exists. The `Partition.Paths` field added in the same series was unused
  once this was fixed and is removed.
