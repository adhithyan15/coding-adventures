# DB-E179 and DB-E180 — Validate scalar escapes in the Go FTL planning fixtures

On the exact candidate `d86af99d701`, both erase-victim readers
and both evacuation planning readers accepted seven malformed
names apiece. The altered declarations reached and passed model
validation. Seven valid alternate spellings also passed in each
reader with their intended decoded names.
[Planning fixture routing](DB-E44-E52-ftl-planning-fixture-routing.md) · [Relocation fixture checks](DB-E178-go-ftl-relocation-fixture-scalars.md)

Deliver two patches in dependency order. DB-E179 covers
`ftl-erase-victim.json` and `ftl-copy-aware-victim.json` in the
erase-victim package. DB-E180 covers `ftl-evacuation-plan.json` and
`ftl-retirement-drain-plan.json` in the evacuation planner package.
Each package uses a shared test-only check before token decoding.
Preserve strict limits, field validation, canonical vector bytes,
model-before-runtime replay and the existing runtime sources.

Run both build lists, race, vet, tidy, Linux/386 and 100,000 inputs
per existing fixture Go fuzz target in each affected module. Keep
all existing negative controls and add named witnesses showing
that a bypass admits the malformed names. Verify all four vectors
still replay their canonical declarations.
