---
category: Security boundaries
---

# Directive ownership must exclude multiline accessibility data after preprocessing

Timeline preprocessing blanks %%{...}%% globally before ACC_DESCR_BLOCK lexing.
A physical init-looking line inside a description therefore leaves a valid
semantic block but can still be selected by a raw-source configuration helper.
The first timeline contract reused the owned Venn root envelope without modeling
this composition; independent contract reviewe1f29b91 correctly withheld it.
No Rust witness was executed at that point, so retain this as source evidence.

Use successful semantic token ownership to blank those inclusive physical lines
only in the timeline-local config-selection copy. Preserve original tokens,
metadata, source/newline lengths, LF/CRLF and shared helper/preprocessor bodies.
Merge ordered intervals in a linear traversal; do not scan the whole source for
every block. Test both config families and a genuine later init root so an inert
first line cannot suppress real configuration. Commit this boundary before tests
or implementation; a helper proven in another dialect is not a proof of the new
preprocessor/token composition.
