---
category: Repo policy / workflow reminders
---

# avoid shell regexes whose quoted character classes contain backticks

I embedded a backtick inside a shell regex whose quoting was already complex;
zsh treated the command as an unmatched quoted expression before `rg` ran.
Avoid shell-level character classes that mix Markdown metacharacters and
quotes. Use a simpler broad search and filter results separately, or place the
assertion in the TypeScript test where the regex is a normal language literal.
