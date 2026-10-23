---
category: Compiler / VM / language pipeline
---

# An F10 start_mode inherits the default patterns only if some rule set-modes to it

`GrammarLexer` decides whether a group is a *flat mode* (its own patterns,
then the default group's) or an *exclusive nested region* purely from the
transition table: a group is flat iff some rule `set-mode`s to it and no rule
`push`es it. `start_mode:` does not count. So a grammar with
`start_mode: beg`, a `group beg:` holding only its overrides (say
`CHAR = /\?[a-z]/`), and rules that `set-mode default` but never
`set-mode beg` starts in an **exclusive** `beg` — the very first identifier
fails with `Unexpected sequence`.

Hit while writing the NEWLINE-transition tests (lexer 0.8.0): the grammar
only reached `beg` through `start_mode`, and every non-`beg` token errored.

What to do: when a mode is meant to be a flat line-start or expression mode,
make sure at least one rule returns to it with `set-mode` (for Ruby-style
grammars `on NEWLINE -> set-mode beg` does this naturally). If the mode is
only ever the start mode, either add such a rule or declare its full pattern
set in the group.
