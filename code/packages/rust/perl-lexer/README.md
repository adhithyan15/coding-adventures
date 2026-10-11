# Perl lexer (Rust)

Tokenizes LANG81's bounded Perl 5.38 `print` arithmetic subset using the
checked-in `code/grammars/perl/perl.tokens` definition. This is not a full
Perl tokenizer.

The pilot integer token admits `0` or a nonzero first decimal digit. It
rejects leading-zero multi-digit forms at parse time; octal literals remain
outside this bounded grammar.
