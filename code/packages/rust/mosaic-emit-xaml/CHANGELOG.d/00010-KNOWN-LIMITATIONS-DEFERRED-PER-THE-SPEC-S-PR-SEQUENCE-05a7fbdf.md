### Known limitations (deferred per the spec's PR sequence)

- **`If` / `Else` / `For` / `Expr`** — these surface as
  `UnsupportedPrimitive` / `UnsupportedExpression` in PR-1. The
  `ExprLowerer` plus the `<ContentControl>` / `<ItemsRepeater>` lowerings
  land in PR-2.
- **`HostInput` / `HostButton` / `HostScroll`** — same: `UnsupportedPrimitive`
  in PR-1, real lowering in PR-3.
- **`HostTable` + section sub-tags** — `UnsupportedPrimitive` in PR-1,
  real lowering in PR-4. The four sub-tags (`HostTableColGroup`,
  `HostTableHead`, `HostTableBody`, `HostTableFoot`) are recognised by
  name only as a "you need PR-4" diagnostic.
- **Component references (non-kernel tags)** — `UnsupportedPrimitive` in
  PR-1; the manifest-driven resolver lands in PR-5.
- **`mosstyle::StyleDef`** — accepted in the signature so consumers can
  build against the stable interface today; only base `part` blocks
  inline as a `<UserControl.Resources>` `<Style>` per part. State
  blocks (`state hover { ... }`) get a placeholder
  `<VisualStateGroup>`; the full `<VisualState>` setter wiring is a
  follow-up.
- **`<UserControl.Resources>` theming cascade** — host overrides land
  with the component-reference resolver in PR-5.
- **`--use-community-datagrid`** flag — placeholder on `EmitOptions`,
  has no effect until PR-4.
- **`--package-mode`** flag — placeholder on `EmitOptions`, has no
  effect until PR-5.
- **`dotnet build` Windows-only smoke test** — gated by
  `#[cfg(target_os = "windows")]`; PR-1 includes the test scaffold but
  the actual `dotnet` CLI invocation lands once we have a real WinUI 3
  consumer (a follow-up after PR-6 builds the VisiCalc demo).
- **`mosaic-compile --backend xaml` CLI wiring** — the CLI driver's
  `run_pipeline` currently only routes `--backend react`; the swiftui
  and qt backends also aren't wired today. A small follow-up PR will
  add the three new arms together (xaml/swiftui/qt) once the team
  agrees on the multi-file output convention (XAML emits three files
  per component; pure react/swift/qt emit one each).
