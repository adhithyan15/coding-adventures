---
category: Compiler / VM / language pipeline
---

# A fail-closed lexer must refuse every token that contains a literal opener, not only the openers themselves

**What went wrong.** Ruby R1b made the 3.3.x lexer fail closed around
line-start `=begin`/`__END__`. Strings were never lexed, and it refused
every regexp, `%`-literal and heredoc opener. The argument that "Ruby is
never inside a multi-line literal at one of our line starts" still had a
hole: the CHAR token (`?c`) inherited a character class that included `"`,
`'`, `` ` `` and `#`. Ruby reads `?"` as a character literal only outside
EXPR_END. Where our flat modes disagree with Ruby's state (after `.end`,
after a local variable), the quote or `#` became a string or comment
boundary on one side only, and a document line could then hide code. The
R1b security review found it with two programs; parsing still rejects both
today, so it was latent.

**Fix.** The four characters were removed from every CHAR class. A
`QUESTION_OPENER` token (`/\?["'`#]/`) is refused in every mode, and a test
pins both programs.

**Next time.** When a fail-closed argument says "every literal opener is
refused", grep the token file for every pattern that can consume an opener
character (`"`, `'`, `` ` ``, `#`, `/`, `%`, `<<`), not just the tokens
named after openers. Character-literal, symbol and `%` classes are the usual
hiding places.
