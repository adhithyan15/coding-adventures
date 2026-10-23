---
category: Repo policy / workflow reminders
---

# Pass the gh jq filter as one argument

The `gh` CLI's `--jq` option accepts exactly one filter argument. Supplying
the standalone `jq -c` flag after `--jq` makes `gh` treat the filter as an
unexpected positional argument, so a status poll produces no useful result.

Pass the filter directly to `--jq`. If compact JSON is needed, make the filter
emit the desired object and let `gh` serialize it rather than forwarding
standalone `jq` command-line flags.
