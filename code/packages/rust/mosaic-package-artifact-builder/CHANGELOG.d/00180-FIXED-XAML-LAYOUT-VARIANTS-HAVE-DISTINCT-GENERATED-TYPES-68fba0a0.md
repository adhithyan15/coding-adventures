### Fixed — XAML layout variants have distinct generated types

Named layouts now suffix their generated XAML partial class, code-behind,
row-view-model helpers, and event union (for example `EngramAppTouch`). This
lets the emitted WinUI project compile every variant side by side without C#
merging two generated files into one type and rejecting hundreds of duplicate
properties and handlers (#14234).

