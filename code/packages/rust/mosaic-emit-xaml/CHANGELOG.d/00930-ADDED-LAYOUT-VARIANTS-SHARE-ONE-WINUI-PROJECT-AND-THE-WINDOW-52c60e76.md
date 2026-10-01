### Added — layout variants share one WinUI project, and the window switches between them (UI48 ENV2/ENV3, §7.11)

- `from_pipeline_variant` emits a layout variant as a control of its own,
  `<Component><Variant>` (`EngramApp.touch.mll` → `partial class
  EngramAppTouch : UserControl`), named by the new `variant_type_name` (`-`
  and `_` separate words, the rule every backend uses). Every type its layout
  needs is named after it, as the default's are after the component.
- A variant declares no event union: its `Dispatch` is the default's
  `EventHandler<<Component>Event>` and its handlers construct the default's
  cases (`EmitContext::event_union`), so one window handler serves every root
  and nothing is declared twice in the project's namespace. Its `events` is
  empty. The control is the same under either shell policy.
- A variant type that would take a name already in the namespace is refused
  (`PipelineEmitError::InvalidLayoutVariant`): the component, its `…Event`
  union or `…Mosaic…` support types, the same for every export in the new
  `EmitOptions::package_exports`, another layout choice's type or its
  `…Mosaic…` support types (`in_support_namespace`), and
  `SHELL_RESERVED_NAMES` -- the shell's
  `App`, `MainWindow`, WinUI's `Program`, the binding's and platform
  library's public types, a package's `MosaicHost` and the three converters.
- `EmitOptions::layout_variants` (`LayoutChoice { variant, conditions }`,
  rule order, wire-name keys) makes a control-rooted project shell switch
  roots: `MainWindow.xaml` holds an empty `LayoutHost` grid instead of the
  component; `MainWindow.xaml.cs` carries `MosaicLayoutRules`, a public
  `MosaicLayoutVariant(environment)` with `select_variant`'s semantics, and
  `CreateLayoutRoot` / `MountLayout` / `SwitchLayout`. The native-complete
  window mounts every root through `ApplyRequiredProps(next, RequiredProps)`
  before showing it; the sample window carries each slot's value across.
  The switch is queued on the `DispatcherQueue` from the ENV4 handlers (one at
  a time), re-checks `MosaicRuntimeHost.IsSettling` when it runs (retrying in
  100 ms), and a failed mount keeps the layout showing and throws nothing.
  After the window's `Closed` nothing is queued or switched, the retry timer
  stops and its tick throws nothing; a root leaving the tree is unsubscribed
  from the window's handler.
  Choices are validated before any C# is written. A dialog-root window does
  not select (`layout_root_is_dialog`).
- Without `layout_variants` every generated file is byte-for-byte what it
  was (TaskApp and RatingControls emitted before and after), and a test pins
  the switching window, minus its edits and the switch, as the plain one.
- `fixtures/layout-variants`: a default and a `compact` layout over two of
  the conformance runtime's props, for the Windows resize gate.
