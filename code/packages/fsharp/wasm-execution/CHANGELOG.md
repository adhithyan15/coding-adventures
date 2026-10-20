# Changelog

## Unreleased

### Fixed

- BUILD now passes `--artifacts-path .artifacts` instead of
  `/p:BaseIntermediateOutputPath=.../\$(MSBuildProjectName)/` and
  `/p:BaseOutputPath=...`. MSBuild does not expand property references in a
  command-line `/p:` value, so every project in the test's reference graph
  shared one literal `obj/$(MSBuildProjectName)/` directory. Main CI failed
  with CS0579 duplicate assembly attributes (run 39331357869); a rerun can
  instead lose the test project's xunit references when another project's
  `project.assets.json` overwrites its own. Each project now gets its own
  `.artifacts/obj/<project>/` and `.artifacts/bin/<project>/`.

## 0.1.0

- Pure F# WebAssembly execution engine.
