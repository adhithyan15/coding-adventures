---
category: BUILD files & dependency management
---

# The Go build tool previews affected packages with dry-run, not list-affected

The current Go build tool has no `--list-affected` flag. Passing that remembered
name exits at argument parsing and validates nothing. Use
`--diff-base origin/main --dry-run` to inspect the affected plan, then run the
same command without `--dry-run` for the actual build.
