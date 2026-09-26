### Added — `HostButton` `selected` on WinUI: a Button with a SelectionItem peer (UI86, #15463)

A `HostButton` with `selected : …` is now emitted as
`<local:{Component}MosaicSelectableButton MosaicSelected="…">`, a generated
`Button` subclass appended to the code-behind.

**The subclass.** Its template, styling, visual states, `Click` and Invoke
pattern are the plain `Button`'s. Only its automation peer differs: a
`ButtonAutomationPeer` subclass that also implements **SelectionItem**.
- `IsSelected` is the bound `MosaicSelected` dependency property.
- `Select` raises `Click`, so a screen reader's selection goes through the
  host.
- `RemoveFromSelection` does nothing, because deselecting is the application's
  decision.
- A change to `MosaicSelected` raises the `IsSelected` property change, and
  `ElementSelected` when the value becomes true.

**Why not `ToggleButton`**, which UI86 first proposed:
- its Checked visual states apply accent brushes over the authored style;
- it toggles itself on click.

**Values.**
- A literal becomes `True` or `False`.
- A bool slot, loop binding or expression becomes `{x:Bind …, Mode=OneWay}`.
  `i == selectedIndex` inside `For` binds the row's live `IsSelected`: the new
  `strip_balanced_outer_parens` removes the parentheses the prop keeps, which
  the row-predicate matcher did not expect.
- A non-bool slot, an unknown bare name, or an expression `x:Bind` cannot take
  is an emit error.
- An elevated selectable button takes its depth through `ElevationZ`, because
  `Translation` plus `<X.Shadow>` breaks the XAML compiler on a custom
  subclass.
- `host_button_selected_is_native` now returns true for the slot, keyword and
  expression shapes.

**Unchanged output:** a button without `selected` is still a plain `<Button>`,
with no helper class and no new usings.

**Fully qualified framework types.** The helper class lands in the package's own
namespace, beside the generated components, so every `Microsoft.UI.Xaml.…` type
is written out in full. The toolkit exports a component named `Button`, whose
generated class is `sealed`, and an unqualified base type bound to that one:
*"cannot derive from sealed type 'Button'"*, on SegmentedControl, Tabs and
ListGroup at once. A single-component fixture compiled fine; building the whole
toolkit is what caught it, and a test now pins that no bare framework type is
emitted.

**Verified.**
- **CI fixture:** the new `fixtures/host-selected-button` covers every value
  shape plus an elevated button. It generates native-complete and builds with
  `dotnet build` on Windows App SDK / .NET 9, locally and now in CI.
- **UI Automation:** `code/scripts/xaml-selected-button-smoke.ps1 -WithRows`,
  run against a build with props set in place of an engine, found:
  - `true` reports IsSelected=True;
  - a false slot reports False;
  - a plain button has no SelectionItem pattern;
  - binding rows report `True,False`;
  - option rows report `False,True,False`;
  - invoking row 0 gives `True,False,False`;
  - `SelectionItem.Select` on row 2 gives `False,False,True`.
- **Tests:** unit tests cover every shape, the absent case, refusals, the
  parenthesis stripping (mutation-checked) and `ElevationZ`.

