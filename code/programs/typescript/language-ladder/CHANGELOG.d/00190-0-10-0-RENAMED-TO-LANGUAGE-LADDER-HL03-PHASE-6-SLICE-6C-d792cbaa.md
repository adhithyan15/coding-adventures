## 0.10.0 — renamed to **language-ladder** (HL03 phase 6, slice 6c)

- **The app is renamed `script-writing-visualizer` → `language-ladder`.** The
  name no longer described it: what began as the HL02 "break a script apart and
  write it" MVP has become the HL03 unified curriculum app, with Learn (the
  teaching sweep + review quiz) as its default and the old script/lesson/concept
  modes folded in as facets. `language-ladder` names what it now is — the
  language chain, climbed rung by rung.
- Directory `git mv`d (history preserved); `package.json`/`package-lock.json`
  name, `index.html` title, the in-app `<h1>`, and the `BUILD` header updated;
  cross-references in the HL03 spec and the Arabic/Russian curriculum docs
  repointed. No source logic changed — the engine and all five modes are byte-
  for-byte the same; 185 tests still pass and the app builds and renders
  unchanged (verified in a real browser). Earlier changelog entries keep the old
  name: they are the accurate record of what happened under it.

