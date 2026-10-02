---
category: CI & GitHub Actions
---

# A manifest change that alters generated code must be checked against CI greps of that generated code

#16490 added `kinds = ["importAnki", "exportAnki"]` to Engram's `[host_effects]`
handlers. It changed nothing at runtime, but it changed the generated Qt entry
point: `installMosaicPlatformEffects(mosaicHost, std::nullopt)` became
`installMosaicPlatformEffects(mosaicHost, QSet<QString>{...})`. The Mosaic Qt
runtime lane in ci.yml greps Engram's generated `main.cpp` for the exact old
line, and the lane's acceptance test pins the same string, so `build
(ubuntu-latest)` failed with no message beyond the grep's exit status.

The local checks were the package's cargo tests and a fresh emit. Both passed,
because neither knows what CI expects the generated text to be.

Do: when a change alters what an emitter writes (a manifest field, a template,
an emitter flag), grep `.github/workflows/` and `code/scripts/tests/` for the
generated lines it changes (here `installMosaicPlatformEffects`) before
pushing. Update every pinned copy in the same PR.
