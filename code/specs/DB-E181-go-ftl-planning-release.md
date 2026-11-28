# DB-E181 — Publish the Go FTL victim selector and evacuation planner

Publish the verified stale-only and copy-aware victim policies,
split destination fences, live-page evacuation plan and retirement
drain plan using the Go page map.
[Victim selection](DB05c1-ftl-erase-victim.md) · [Copy-aware selection](DB05c3c-ftl-copy-aware-selection.md) · [Evacuation](DB05c3a-ftl-evacuation-plan.md) · [Retirement drain](DB05d2b1-ftl-retirement-drain-plan.md) · [Scalar checks](DB-E179-E180-go-ftl-planning-fixture-scalars.md)

Carry the planner source from `74ac71c6bbc`, `5313acf23a9`,
`c59a70d8b4e` and the evacuation planner's scoped changes in
`be6f1d29805`, followed by the verified reader repairs. Introduce
contracts before source. Preserve the four canonical vectors.

Route `ftl-erase-victim.json` and `ftl-copy-aware-victim.json` to
`go/ftl-erase-victim`. Route `ftl-evacuation-plan.json` and
`ftl-retirement-drain-plan.json` to `go/ftl-evacuation-plan`.
Prove independent changes and deletions select each owning package,
use the exact cache inputs, and preserve every existing route.
Regenerate the checked registries on the current release base.

Each new graph row carries only its page-map dependency, two exact
vectors and the unchanged native recipe. Verify both Go lists, race,
vet, tidy and 386, every parser fuzz target and existing mutation
runner. Run the four build-tool fronts and verify the full submitted
head's package results and required gates before release.
