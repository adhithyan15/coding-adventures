# DB-E178 — Validate scalar escapes in the Go FTL relocation fixtures

All four readers in the `ftl-nand-relocation` package at `df42efb18b6`
admitted seven malformed case names. The JSON decoder replaced isolated
surrogates and reversed pairs with replacement characters, and each
fixture's model accepted the decoded name. Seven valid spellings passed
in each reader, including both supplementary-plane endpoints, literal
and escaped replacement characters, and an escaped backslash.
[Relocation fixtures](DB-E43-E50-E54-ftl-nand-relocation-fixture-routing.md) · [Page-map scalar checks](DB-E177a-go-ftl-page-map-fixture-scalars.md)

Run the scalar check once in the shared test-only JSON scanner before
it decodes tokens. Cover `ftl-nand-relocation.json`,
`ftl-block-reclamation.json`, `ftl-live-page-copy.json` and
`ftl-evacuation.json`. Preserve the reader limits, exact canonical
vectors, JSON-kind checks, and model-before-runtime replay order.

Refuse isolated low and high halves, reversed pairs, missing or wrong
low halves, incomplete escapes and nonhex digits. Preserve exact names
for valid pairs and escaped backslashes. Replay all existing declarations.
Keep the package runtime, module, build recipes, and all four fixture
vectors byte-identical.

Run both build lists, race, vet, tidy, Linux/386, 100,000 executions of
each of the four parser fuzz targets, and all existing mutation controls.
Add named clean tests per reader and demonstrate that bypassing shared
scalar validation fails all four witnesses.
