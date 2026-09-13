# Rewriting a CRLF file with Python can break a compiler — and `grep -c $'\r'` lies about it

Editing `TaskApp.light.msl` (CRLF in the working tree, `eol=lf` in `.gitattributes`)
by reading + rewriting it in Python produced a file the mosstyle lexer rejected:

```
mosstyle tokenization failed: LexerError at 8:1: Unexpected sequence '\r'
```

Two traps, one after the other:

1. **Mixed endings.** Python's text mode translates on read *and* write, so a
   partial rewrite can leave the original CRLF lines alongside freshly written LF
   ones. Some hand-written lexers (mosstyle's included) accept a consistent file
   but choke on the mix.
2. **The obvious check gives a false negative.** `grep -c $'\r' file` reported
   **0** under Git Bash while `od -c file | grep -c '\r'` reported **238**. I
   nearly concluded the file was clean. Verify with `od -c`, or
   `python -c "print(open('f','rb').read().count(b'\r'))"` — not `grep`.

Fix: normalize to LF on disk, which is what git stores and what CI sees anyway
(`git check-attr text eol -- <file>` to confirm). When scripting an edit to a
tracked file, read and write **binary** (`open(p,'rb')` / `'wb'`) so you never
silently re-encode line endings you weren't asked to touch.
