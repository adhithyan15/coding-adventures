---
category: Security boundaries
---

# A quadratic duplicate check in a shared parser becomes a denial of service the moment a document may pin a 100,000-row data file

`test-bootstrap`'s `tfv::parse_file` checked for a repeated record id by scanning
every earlier record (`records.iter().any(..)`).  That is fine for the small corpora
it was written for and quadratic (about 5 * 10^9 string comparisons) for the
100,000-row tables VER04j WX-E lets a document pin.  The pin only says *which*
file is read, so it is no defence against a file that is merely large.

When a limit in a spec grows the input of an existing parser, re-read that parser
for superlinear steps: use a hash set for uniqueness, count lines before parsing,
and refuse shapes the target never needs.  Here the TFV map-key check is also
pairwise, so the table loader refuses any map header with more than one entry (a
`Result` is the only map a Warrant column can hold) from a token scan before the
parser runs.  Write the regression test as a size test (100,000 rows parse, and a
duplicate is still reported on its own line) rather than a timing assertion alone.
