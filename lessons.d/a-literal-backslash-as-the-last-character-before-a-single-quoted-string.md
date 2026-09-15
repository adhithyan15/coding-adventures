---
category: Perl
---

# A literal backslash as the last character before a single-quoted string's closing `'` is silently absorbed as an escaped quote

, not "backslash then end of string" — `'...)\/\'` does NOT end where it looks like it does; Perl keeps scanning for the real closing quote, silently swallowing subsequent lines into the string literal and producing a confusing downstream syntax error many lines later (not at the actual mistake). Any content ending in a real backslash needs `\\` before the closing quote: `'...)\/\\'`. Bit a cowsay `.cow`-art string embedded in a test file (`'            (__)\       )\/\'` needed to become `'...\\'`). When embedding cow-art or other backslash-heavy literal text in Perl single-quoted strings, check for a lone (un-doubled) `\` immediately before the closing `'`.
