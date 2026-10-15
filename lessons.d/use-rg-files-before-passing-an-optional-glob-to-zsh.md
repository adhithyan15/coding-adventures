---
category: Repo policy / workflow reminders
---

# Use rg --files before passing an optional glob to zsh

I passed an optional `*.json` path glob directly to zsh while looking for Sanskrit assessment metadata; because no top-level file matched, zsh aborted the whole diagnostic command before later checks ran. Discover candidate files with `rg --files` first, then pass only concrete paths to readers or searches.
