---
category: Testing & coverage
---

# The lessons validator already validates the quirks shards

During Warrant source validation, `lessons.py validate` succeeded and printed
both the lessons and quirks shard results. An additional guessed `quirks.py`
invocation failed because that script does not exist. A later successful
command made the shell's final exit code zero; it did not erase that failure.

Use the repository's actual combined validator and check dependent exit codes.
Inspect available entrypoints with `rg --files code/scripts` before guessing a
script name. The corrected final validation runs `lessons.py validate` alone
and verifies its status; no separate quirks-script success is claimed.
