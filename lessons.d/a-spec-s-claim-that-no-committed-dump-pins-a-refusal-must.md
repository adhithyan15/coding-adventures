---
category: Testing & coverage
---

# A spec's claim that no committed dump pins a refusal must be re-grepped against every dump that landed after the claim was written

VER04r R31 (c) opened the record and `Option` payloads of `missing-error` and said "checked: no `*refusals*` dump of the
oracle names `missing-error`". The check was true on the day it was written; the next slice (S11f-1) then committed
`scalar-faults-v1`, whose refusals dump pins `Result<Option<U8>,E>` as refused ("stays refused until S11c"). When S11c-1
implemented the template, the fixture's own `--check` line failed, and the acceptance rule "no committed byte moves" forbade
regenerating it. The oracle had to keep the older rule for a caller that does not say what the document declares
(`fault_row(..., records=None)`), and the conflict had to go to the owner.

What to do differently: a spec sentence of the form "no committed X pins Y" is a claim about a moving tree. Before the slice
that depends on it starts, grep every dump and fixture directory on the base for the refusal's message and its signature, and
if a later slice pinned it, write the regeneration (and the generator's mirror) into the slice's plan instead of discovering
it at the first `--check`. A refusal row that says "until slice N" is a promise to move that row in slice N.
