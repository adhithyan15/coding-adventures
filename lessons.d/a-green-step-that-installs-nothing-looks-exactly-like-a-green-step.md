# A green step that installs nothing looks exactly like a green step

Two CI sites ran `sudo apt-get install -q libreadline-dev libncurses-dev`. `-q`
is **quiet**; `-y` is **assume-yes**. A non-interactive `apt-get install` cannot
complete an install that requires changes without `-y` — it prompts and aborts.

Those jobs were green. Which means the step was not installing anything: it
read as a dependency step and behaved as a no-op, and it would have started
failing the day the runner image stopped shipping those packages — as a
confirmation prompt, not a missing dependency, so the error would not have
pointed at the cause either.

Nothing observable distinguishes "installed the dependency" from "the
dependency happened to be there already" in a green log. The flag is the only
place the difference is written down.

Two notes on how it was handled:

- **I could not verify apt's behaviour locally** (no Docker on this machine)
  and said so rather than dressing up the reasoning as a measurement. The fix
  does not depend on the demonstration: `-y` is required for a non-interactive
  install regardless of what the runner happens to have preinstalled.
- **The guard is written against the flag, not the spelling.** It accepts
  `--yes` as well as `-y`, checked in both directions — a test that only knew
  `-y` would fail a correct future edit, which is the same false-positive trap
  as the allowlist.
