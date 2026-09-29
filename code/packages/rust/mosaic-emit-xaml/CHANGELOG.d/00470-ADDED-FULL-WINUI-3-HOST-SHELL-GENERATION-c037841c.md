### Added — full WinUI 3 host shell generation

`mosaic-compile --backend xaml --emit-project -o <BASE>` now produces a
buildable WinUI 3 project alongside the per-component triple. Output:

| File | Source |
|---|---|
| `<Component>.xaml` / `.xaml.cs` / `.Event.cs` | mosaic-emit-xaml (component triple) |
| `<Component>.csproj` | --emit-project |
| `App.xaml` / `App.xaml.cs` | --emit-project |
| `MainWindow.xaml` / `MainWindow.xaml.cs` | --emit-project |
| `app.manifest` | --emit-project |
| `build.ps1` | --emit-project |
| `README.md` | --emit-project |
| `BoolToVisibilityConverter.cs` (when `If` used) | A5 (PR-2) |
| `<Component>_<As>Vm.cs` (one per For block) | PR-2 |

The `MainWindow` shape depends on the component's `RootShape`:
- `ContentDialog`-rooted (HostDialog): host window has a "Show
  dialog" button which constructs the dialog, sets its `XamlRoot`
  from the button (Fix D1 from the demo catalog), wires the
  `Dispatch` event to a stub handler, and `ShowAsync`'s it.
- `UserControl`-rooted: host window's Grid hosts the component
  directly as its main content; component DPs are wired in the
  MainWindow constructor.

Slot DPs are pre-populated with sensible stubs (`"Sample <Slot>"`
for text, `0` for number, `false` for bool, `null!` for image/node,
empty list for `list<T>`). The user replaces them with real data.

The `Dispatch` event is wired to `OnComponentDispatch` which
pattern-matches the discriminated event union. Each arm has a
`// TODO: business logic for <EventName>` comment marking the
insertion point.

