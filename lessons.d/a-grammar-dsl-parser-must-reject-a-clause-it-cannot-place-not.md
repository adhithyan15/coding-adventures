---
category: Compiler / VM / language pipeline
---

# A grammar DSL parser must reject a clause it cannot place, not fold it into a name

**What went wrong.** LANG84 E1/E8 added `@line_start` guards and `if`
conditions to the `.tokens` DSL. The first version split a transition head
on the literal `" if "` and took everything after `->` as the alias. So
`on X if -> refuse`, `on X⇥if space_before -> refuse` and
`X = /x/ ->Y@line_start` all parsed without error. The trigger became the
token name `X if` and the alias became `Y@line_start`. Neither can ever
match, so a *refusal* rule silently never fired and a *guard* silently
vanished: the lexer failed open. The pre-push security review caught it
(findings L1 and L3), along with two cousins:
- `refuse` rules in `mode: indentation` grammars, whose tokenizer never
  runs transitions (L2);
- a NEWLINE `space_before` condition that could never hold (L4).

**Fix.**
- Split on `if` as a whitespace-delimited word.
- Reject a bare `if`.
- Reject `@` inside an alias.
- Validate that every transition trigger names a token the grammar can emit.
- Reject E8 features where the engine cannot apply them.
- Do the same in every grammar-tools port.

**Do differently.** When a DSL gains a keyword or sigil that shares a line
with free-form names, ask of every new clause: "if this is malformed, does
it become an error, or does it become part of a name?" Separators must be
whole words. Names must be validated against what can actually occur. A
feature that a code path ignores must be rejected on that path. Test the
malformed spellings (tab, missing argument, no space) and not only the
happy path.
