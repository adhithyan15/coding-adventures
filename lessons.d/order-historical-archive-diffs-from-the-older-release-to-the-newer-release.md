---
category: Perl
---

# Order historical archive diffs from the older release to the newer release

Adapting the Perl71 evidence script by replacing 5.004_70 with 5.004_72
left its comparison tuple ordered 72 then 71. Hashes and byte-identity checks
were valid, but the displayed additions and removals described a rollback.
Correct the tuple to 71 then 72 and regenerate the diff before interpreting
changes. Here the forward diff proves that 72 enables TMP_CRLF_PATCH and
adds CR normalization; the unchanged decimal scanner does not imply unchanged
whitespace semantics. Also discover lesson filenames before reading them:
a guessed archive-audit lesson path in this run did not exist.
