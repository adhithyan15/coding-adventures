---
category: Repo policy / workflow reminders
---

# An unquoted heredoc runs the backticks in a changelog as commands

**What happened:** I wrote the package `CHANGELOG.d` shard for Latin's A2 pin
with a shell heredoc opened as `<<EOF`. The shard names files in Markdown code
spans, such as `` `tests/level-gate-attainment/latin.json` ``. Bash treats
backticks inside an unquoted heredoc as command substitution. So it tried to
*run* each path, printed "No such file or directory", and wrote the shard with
the paths cut out: "-  moves from A1 to A2".

The stray output gave it away before anything was committed. The shard was
rewritten with a quoted heredoc.

**What to do differently:**

1. Open any heredoc that writes Markdown, LaTeX or other prose as `<<'EOF'`,
   with the delimiter quoted. Then `$`, backticks and `\` pass through
   literally.
2. If the text has to interpolate a shell variable, build it in Python or with
   `printf` rather than unquoting the whole heredoc.
3. Treat any "command not found" or "No such file" line printed by a command
   that only writes files as a sign that the written text is damaged. Re-read
   the file.
