# DB-E177a — Validate scalar escapes in both Go FTL map fixtures

On the exact page-map candidate `d3ae98ed9e1`, an isolated high
surrogate, isolated low surrogate and reversed pair in a case
name were converted to replacement characters. The original
page-map reader and the reclamation reader each accepted the
altered fixture and passed it to their model validation.
[Page map](DB05a-ftl-page-map.md) · [Reclamation](DB05c2a-ftl-page-reclamation.md) · [Admitted NAND scalar rule](DB-E176a-go-nand-block-erase-fixture-scalars.md)

Share scalar admission across the two fixture readers in
`go/ftl-page-map`. Run it before JSON token decoding and model
admission. Preserve the existing strict fixture limits, exact
canonical declarations and model-before-runtime order.

Refuse isolated and reversed surrogate escapes, an absent or
wrong low half, incomplete sequences and bad hexadecimal
digits. Preserve valid scalar pairs, the replacement character
in literal and escaped form, and an escaped backslash followed
by ordinary surrogate-shaped text. Check the identical set
against each reader and assert the decoded names on success.

Keep the runtime and both shared vectors byte-identical. Run
both lists, race, vet, tidy, Linux/386, 100,000 executions of
each fixture Go fuzz target and the existing and new clean
mutation controls. A bypass of either adapter's scalar check
must fail its named witness.
