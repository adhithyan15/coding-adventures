---
category: Perl
---

# Perl modules must `use lib '../sibling/lib'`

themselves, not just from test files. `prove -l` only adds local `lib/`, and `use lib` in tests doesn't help if the module compiles `use Sibling::Module` at compile time.
