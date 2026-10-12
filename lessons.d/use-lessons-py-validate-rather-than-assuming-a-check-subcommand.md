---
category: Repo policy / workflow reminders
---

# use lessons.py validate rather than assuming a check subcommand

I invoked `lessons.py check` from memory, but the tool exposes `validate`, not
`check`, so the validation command stopped before running the following diff
checks. Read the command list from `--help` or the usage emitted by the tool and
use `python3 code/scripts/lessons.py validate`; do not transfer a conventional
`check` subcommand name from unrelated repository tools.
