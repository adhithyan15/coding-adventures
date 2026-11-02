---
category: Repo policy / workflow reminders
---

# pkill -f with a pattern that also appears in your own command line kills your own shell

To stop a hung `sir-conformance` run, `pkill -f 'zz_probe-'` was used.  The
pattern also matched the shell running the `pkill` itself (its command line
contained the same string), so the shell was killed (exit 144) and the
processes meant to be stopped kept running.

What to do: find the PIDs first (`ps aux | grep … | grep -v grep`) and
`kill` those PIDs, or use a pattern that cannot occur in your own command
line (for example `pgrep -f '^/path/to/binary'`).
