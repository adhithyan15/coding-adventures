---
category: Testing & coverage
---

# A CRLF test fixture is normalized to LF on commit under eol=lf unless its path is marked -text

LANG84 phase 4c added `code/grammars/perl/corpus/pl-int-print/crlf_layout.pl`,
a corpus case whose whole point is its CRLF line ends: it proves a carriage
return is whitespace in some Perl release pairs and a lex error in others.
The repository's root `.gitattributes` says `* text=auto eol=lf`. Under that
rule `git add` stores a CRLF text file with LF line ends, and every checkout
writes it back with LF. The file on disk at authoring time is still CRLF, so
every local test passes. The committed case would have tested nothing, and
the release sweep would have put the CR-refusing releases into the wrong
identity class on the next fresh checkout.

The fix was a path-specific `-text` line in the root `.gitattributes`, with a
comment saying why. `git ls-files --eol <path>` then showed `i/crlf w/crlf`.

What to do differently: before committing a fixture whose bytes are the test
(CRLF, a BOM, trailing whitespace, NUL), check `git check-attr text eol <path>`
and `git ls-files --eol <path>` after `git add`. A green local run proves only
what is on disk, not what the index holds.
