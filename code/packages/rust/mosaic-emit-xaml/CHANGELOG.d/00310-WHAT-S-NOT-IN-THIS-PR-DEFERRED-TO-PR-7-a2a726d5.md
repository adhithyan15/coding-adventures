### What's NOT in this PR (deferred to PR-7)

- **VisiCalc Windows demo** (`code/programs/typescript/visicalc/windows/xaml/`) — the
  full end-to-end app that consumes the compiled `mosaic-pkg-grid`
  package and a hand-written `FormulaBar` component. PR-7 lands
  this directory, the `windows/build.ps1` driver, and the
  hand-written C# host code (`State.cs` mirroring
  `src/app/state.ts`).
- **`dotnet build` smoke test** on Windows CI. Requires the
  Microsoft .NET SDK + Windows App SDK; will land alongside the
  demo so we have a real consumer to validate against.
- **Manifest-driven CLI** (`mosaic-compile pkg <path> --backend xaml`)
  that walks `mosaic-package.toml`, parses dependency manifests,
  and constructs the `ComponentRegistry`. The single-component
  invocation works today; the multi-component package invocation
  needs the resolver wired into `run_pkg`.

