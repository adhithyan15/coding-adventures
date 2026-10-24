---
category: Repo policy / workflow reminders
---

# Architecture specification prefixes must stay unique across the simulator series

The first EDSAC draft reused the `07n` prefix already owned by the Motorola
68000 specification.  The full filenames differed, so ordinary file checks
did not report a collision, but the numbered architecture sequence became
ambiguous for readers and for future paired gate-level specifications.

Before assigning a spec identifier, list every existing file in that numbered
series and reserve the next unused prefix for both the functional and gate
partners.  A new architecture may share the broad Spec 07 family, but it must
not reuse another architecture's exact ordering prefix.
