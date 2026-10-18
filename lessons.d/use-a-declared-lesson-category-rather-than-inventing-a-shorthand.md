---
category: Repo policy / workflow reminders
---

# Use a declared lesson category rather than inventing a shorthand

I passed `--category process` to `lessons.py new`; the command exited 1 and
created no lesson because that category is not declared. I used its printed
category list and reran with `Repo policy / workflow reminders`, then filled
the returned shard. Use a declared category from `_meta.md` or the CLI rather
than inventing a shorthand. A rejected lesson command is not a test result.
