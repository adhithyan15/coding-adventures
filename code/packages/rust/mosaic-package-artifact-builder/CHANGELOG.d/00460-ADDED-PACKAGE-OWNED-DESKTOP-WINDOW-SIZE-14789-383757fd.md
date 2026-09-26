### Added — package-owned desktop window size (#14789)

Generated Compose, Qt, SwiftUI, XAML, and Electron project shells now honor
the manifest's optional `[app]` initial window dimensions. A missing declaration
keeps the existing emitter defaults, while a declared size reaches each
platform's native window API.

- Implement WebComponent table numeric typography and scoped editor inheritance;
  retain direct table input/button nodes and nested tables (#15713).

- Recognize WebComponent text/control numeric typography; retain the degradation
  diagnostic for table-wide inheritance (#15692).

- Support HTML HostTable numeric typography with scoped native-control inheritance,
  row/container overrides and nested-table boundaries (#15677).

- Recognize HTML numeric typography on text, buttons and inputs; retain
  diagnostics for HTML tables and WebComponent bindings (#15647).

- Recognize Flutter HostTable numeric typography now that header/cell/editor
  propagation is implemented and exercised in generated widgets (#15602).

- Recognize Flutter numeric typography on text, buttons and inputs; continue
  reporting unsupported HostTable font-size bindings (#15598).

- Recognize XAML HostTable numeric font-size projection now that native
  header/cell/editor propagation is implemented and exercised (#15564).


- Recognize implemented XAML numeric typography on text/input/button primitives;
  retain the unsupported font-size diagnostic for HostTable (#15556).

