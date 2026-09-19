### Dart build-tool graph/diff core

- Add an authority-free Dart dependency-graph and diff-selection core that
  consumes all eight graph and eleven diff-selection neutral fixtures.
- Generate source-embedded Unicode 17 normalization, full-fold, and uppercase
  tables for portable path policy, with official-vector self-checks on exact
  Dart 3.12.2.
- Wire the Dart consumer and generated Unicode runtime into strict package
  validation and required CI gates without claiming a complete build-tool
  front door.
