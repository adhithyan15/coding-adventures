# mosaic-emit-xaml — WinUI 3 / XAML backend for Mosaic

**Status:** Specification (draft)
**Layer:** UI / backend emitter
**Depends on:** UI23 (mosaic-pipeline), UI24 (mosaic-emit-dispatch), the three
IR compilers (mosmodel, moslayout, mosstyle), UI25 (Input), UI27 (Grid v2)
**Sibling backends:** UI20 (React/TSX), `mosaic-emit-win32.md` (pure Win32 +
Paint VM, Rust output)
**Produces:** WinUI 3 C# source files (`.xaml` + `.xaml.cs` + project glue)

---

## 1. Purpose

This spec defines a Mosaic backend that lowers a three-file pipeline triple
(`.mil` + `.mll` + `.msl`) to a WinUI 3 component:

```
Grid.mil       \
Grid.desktop.mll → mosaic-compile --backend xaml → Grid.xaml + Grid.xaml.cs
Grid.dark.msl  /
```

WinUI 3 was chosen as the entry point for the XAML family (over WPF and
UWP) because:

1. It is the actively developed Microsoft Windows UI framework (Windows App
   SDK 1.x line) — WPF is in maintenance, UWP is steered toward WinUI 3.
2. Its XAML parser and control set are the closest to a "modern" XAML that
   future WPF / Uno / Avalonia backends can fork from.
3. The Mosaic moslayout vocabulary (Box / Row / Column / Grid / Input /
   Text / Image / Spacer / Scroll / Divider / Stack / Icon) has direct
   one-to-one XAML control mappings — no novel JSX-style trickery is
   required.

The companion `mosaic-emit-win32.md` spec covers the other extreme: pure
Win32 + Direct2D via the Paint VM, no XAML, Rust output. The two share
nothing except the UI24 event-dispatch contract.

## 2. Output shape

For a component named `Grid` declared by `Grid.mil` / `Grid.desktop.mll` /
`Grid.dark.msl`, the backend writes:

```
Grid.xaml         — XAML markup; root is the moslayout root primitive lowered to a Panel
Grid.xaml.cs      — code-behind: dependency properties for each slot, a
                    `Dispatch` event for the discriminated event union,
                    InitializeComponent boilerplate, and any inline event
                    handlers UI24 dispatch wiring requires
Grid.Event.cs     — the discriminated event union (record types + interface)
                    — equivalent of the TS `export type GridEvent = ...`
                    from UI24 §3.1
```

Additionally, when invoked with `--emit-project` (default off), the backend
writes a minimal WinUI 3 `.csproj` and `App.xaml` / `App.xaml.cs` that hosts
the generated component as the top-level window. With `--emit-project off`
the backend only writes the per-component triple above and the caller is
expected to integrate them into an existing WinUI 3 project. The VisiCalc
plumbing (§9) uses `--emit-project` on; library consumers turn it off.

## 3. Mapping table — moslayout primitives → XAML

| moslayout primitive | XAML control                          | Notes |
|---------------------|---------------------------------------|-------|
| `Box`               | `<Border>`                            | Holds part-styled background, border, padding. |
| `Row`               | `<StackPanel Orientation="Horizontal">` | Children flow left→right. |
| `Column`            | `<StackPanel Orientation="Vertical">`   | Children flow top→bottom. |
| `Stack`             | `<Grid>` (single cell, all children)   | Z-stack via children's `Grid.ZIndex`. |
| `Grid`              | `<Grid>` with `RowDefinitions` / `ColumnDefinitions` and `<DataTemplate>`-based content for `headers` + `rows`. | See §4. |
| `Text`              | `<TextBlock>`                          | `Text="{Binding ...}"` to a slot. |
| `Image`             | `<Image Source="..."/>`                | |
| `Input`             | `<TextBox>` (or `<MultilineTextBox>` analogue) | See §5; `multiline: true` → AcceptsReturn=True + TextWrapping=Wrap. |
| `Scroll`            | `<ScrollViewer>`                       | |
| `Spacer`            | `<Rectangle/>`                         | With `Width` / `Height` set from the moslayout prop. |
| `Divider`           | `<Border BorderThickness=".."/>`       | Direction picked from props. |
| `Icon`              | `<FontIcon Glyph="..."/>`              | Segoe Fluent Icons font. |

Every emitted control gets `x:Name="{kebab→PascalCase part name}"` so the
code-behind can refer to it without re-querying the visual tree.

## 4. Grid primitive (UI27 §3 + §2.1 in this spec)

XAML has no built-in spreadsheet control, so Grid lowers to a virtualised
`<ItemsRepeater>` (or `<ListView>` with `ItemContainerStyle` for header
rows) wrapped in a `<Grid>` that holds the header row, the data rows
viewport, and (optionally) a `<colgroup>`-equivalent built from
`<Grid.ColumnDefinitions>` driven by the `column-widths` slot.

Concretely:

```xml
<Grid x:Name="Sheet" Background="{x:Bind PartBackground}">
  <Grid.RowDefinitions>
    <RowDefinition Height="Auto"/>   <!-- header row -->
    <RowDefinition Height="*"/>      <!-- data rows scroll-viewer -->
  </Grid.RowDefinitions>
  <Grid.ColumnDefinitions/>          <!-- generated at runtime from column-widths -->

  <ItemsRepeater Grid.Row="0" ItemsSource="{x:Bind ColumnHeaders}">
    <ItemsRepeater.Layout>
      <StackLayout Orientation="Horizontal"/>
    </ItemsRepeater.Layout>
    <ItemsRepeater.ItemTemplate>
      <DataTemplate x:DataType="x:String">
        <Border Style="{StaticResource HeaderCellStyle}">
          <TextBlock Text="{x:Bind}"/>
        </Border>
      </DataTemplate>
    </ItemsRepeater.ItemTemplate>
  </ItemsRepeater>

  <ScrollViewer Grid.Row="1">
    <ItemsRepeater ItemsSource="{x:Bind ViewportRows}">
      <ItemsRepeater.ItemTemplate>
        <DataTemplate x:DataType="local:RowVm">
          <ItemsRepeater ItemsSource="{x:Bind Cells}">
            <ItemsRepeater.Layout>
              <StackLayout Orientation="Horizontal"/>
            </ItemsRepeater.Layout>
            <ItemsRepeater.ItemTemplate>
              <DataTemplate x:DataType="local:CellVm">
                <Border Style="{StaticResource DataCellStyle}" Tapped="Cell_Tapped">
                  <TextBlock Text="{x:Bind Value}"/>
                </Border>
              </DataTemplate>
            </ItemsRepeater.ItemTemplate>
          </ItemsRepeater>
        </DataTemplate>
      </ItemsRepeater.ItemTemplate>
    </ItemsRepeater>
  </ScrollViewer>
</Grid>
```

The code-behind generates `RowVm` and `CellVm` POCOs whose properties match
the `list<list<text>>` shape coming in via the `viewport-rows` slot.
Per-cell selection / editing highlights are realised by `CellVm` toggling
visual states (`VisualStateManager.GoToState`) rather than inline style
spreads.

### 4.1 column-widths

When the moslayout's Grid binds `column-widths: slot: column-widths`, the
code-behind appends one `<ColumnDefinition Width="{w}"/>` per element to
`Sheet.ColumnDefinitions` and walks the per-row `ItemsRepeater` setting
`Grid.Column=i, Width=widths[i]` on each cell `Border`. The slot stays
`list<number>` (interpreted as DIPs — the WinUI default unit).

### 4.2 onNavigate

The `<Border>` template's `Tapped` handler is generated as:

```csharp
private void Cell_Tapped(object sender, TappedRoutedEventArgs e)
{
    var cell = (CellVm)((FrameworkElement)sender).DataContext;
    Dispatch?.Invoke(this, new GridEvent.Navigate(cell.Row, cell.Col));
}
```

Where `Dispatch` is the routed-event delegate defined in §6.

## 5. Input primitive (UI25)

`Input` lowers to `<TextBox>` with two-way binding to the bound `value`
slot. UI25 specifies four emits — `onChange`, `onCommit`, `onCancel`,
`onFocus` — which the backend maps as:

| Emit         | XAML hookup                                          |
|--------------|------------------------------------------------------|
| `onChange`   | `TextChanged` handler dispatching a `Change(value)`. |
| `onCommit`   | `KeyDown` handler with `e.Key == Enter`.             |
| `onCancel`   | `KeyDown` handler with `e.Key == Escape`.            |
| `onFocus`    | `GotFocus`.                                          |

`multiline: true` props produce `AcceptsReturn=True`, `TextWrapping=Wrap`,
and an explicit `Height="*"` row in the parent Grid.

When the moslayout grammar adds string-literal prop values (today's known
limitation #3 in `demo/visicalc/README.md`), `placeholder: "Enter formula"`
will compile to `PlaceholderText="Enter formula"`. Until then, the
emitter ignores the prop (matching the React backend's behaviour).

## 6. Event dispatch contract (UI24)

For a component with `n ≥ 0` emits, the emitter writes
`{Component}.Event.cs`:

```csharp
namespace Mosaic.Generated;

public abstract record GridEvent
{
    public sealed record Navigate(int Row, int Col) : GridEvent;
    public sealed record EditCommit(string Value)   : GridEvent;
    // ...
}
```

And in `Grid.xaml.cs`:

```csharp
public sealed partial class Grid : UserControl
{
    /// <summary>Fires once for every emit declared in the .mil interface.</summary>
    public event EventHandler<GridEvent>? Dispatch;

    // One DependencyProperty per slot — see §7.
}
```

Hosts wire it up identically to the React `dispatch` prop:

```csharp
grid.Dispatch += (sender, e) => state.HandleEvent(e);
```

The empty-emit case still emits the abstract record with zero concrete
variants (matching the `export type Foo = never` shape from UI24 §3.1) so
host code can pattern-match exhaustively without special-casing.

## 7. Slot lowering — mosmodel → DependencyProperty

Every slot in `.mil` becomes a public WinUI `DependencyProperty` with the
following type translation:

| mosmodel slot type   | C# property type                  |
|----------------------|------------------------------------|
| `text`               | `string`                          |
| `number`             | `double`                          |
| `bool`               | `bool`                            |
| `color`              | `Windows.UI.Color`                |
| `image`              | `Microsoft.UI.Xaml.Media.Imaging.ImageSource` |
| `node`               | `Microsoft.UI.Xaml.UIElement`     |
| `list<T>`            | `IReadOnlyList<T-mapped>`         |
| `list<list<T>>`      | `IReadOnlyList<IReadOnlyList<T-mapped>>` (UI27 §2.1) |

DependencyProperty registration is hand-rolled per slot to keep changes
observable from XAML bindings; the property name is kebab→PascalCase
(`column-widths` → `ColumnWidths`). The `Required: true` flag in the .mil
becomes a `[Required]` attribute on the property; bindings still
compile, but a runtime warning is logged on first render when the
property is its default.

## 8. Style application (UI23 §5)

The mosstyle `.msl` source's `part` blocks are lowered into a
`<ResourceDictionary>` emitted into `Grid.xaml`'s top-level `<UserControl.Resources>`:

```xml
<UserControl.Resources>
  <Style x:Key="SheetStyle" TargetType="Border">
    <Setter Property="Background" Value="#1e1e1e"/>
    <Setter Property="FontFamily" Value="Consolas"/>
    <Setter Property="FontSize"   Value="12"/>
  </Style>
  <!-- sub-parts (UI27 §3) get their own keys: SheetCellStyle, SheetHeaderCellStyle, ... -->
</UserControl.Resources>
```

CSS property names are translated to XAML setter names by a fixed table —
no design-time inference. The table covers the UI26 demo property surface:

| .msl property              | XAML setter                                   |
|----------------------------|-----------------------------------------------|
| `background`               | `Background`                                  |
| `color`                    | `Foreground`                                  |
| `font-family`              | `FontFamily`                                  |
| `font-size`                | `FontSize`                                    |
| `font-weight`              | `FontWeight`                                  |
| `padding`                  | `Padding`                                     |
| `border-width`             | `BorderThickness`                             |
| `border-color`             | `BorderBrush`                                 |
| `border` (shorthand UI27)  | `BorderThickness` + `BorderBrush` (split)     |
| `border-collapse`          | (ignored — XAML grids do not have collapse)   |
| `width` / `height`         | `Width` / `Height`                            |

State blocks (`state hover { … }`, `state pressed { … }`, etc.) compile to
`<VisualState>` entries inside a `<VisualStateGroup>` named after the
state's parent part:

```xml
<VisualStateManager.VisualStateGroups>
  <VisualStateGroup x:Name="SheetCellStates">
    <VisualState x:Name="Hover">
      <VisualState.Setters>
        <Setter Target="Cell.Background" Value="#2a2d2e"/>
      </VisualState.Setters>
    </VisualState>
  </VisualStateGroup>
</VisualStateManager.VisualStateGroups>
```

(See "state semantics" in UI27 §5.)

## 9. VisiCalc plumbing — proof that this all hangs together

Once both this backend and `mosaic-emit-win32` land, the VisiCalc demo
re-uses its `.mil` / `.mll` / `.msl` sources unchanged and produces:

```
demo/visicalc/
  windows/xaml/
    VisiCalc.csproj           — generated WinUI 3 project (when --emit-project)
    App.xaml, App.xaml.cs     — generated host shell
    Grid.xaml, Grid.xaml.cs, Grid.Event.cs
    FormulaBar.xaml, FormulaBar.xaml.cs, FormulaBar.Event.cs
    State.cs                  — hand-written host equivalent of src/app/state.ts
                                 (reducer over the union of GridEvent +
                                 FormulaBarEvent + host events, mirrors
                                 the React reducer shape per UI26 §7.3)
    MainWindow.xaml(.cs)      — generated; instantiates Grid + FormulaBar and
                                 wires Dispatch to State.HandleEvent
  windows/build.ps1           — calls `mosaic-compile --backend xaml` for each
                                 component, then `dotnet build VisiCalc.csproj`
```

`demo/visicalc/windows/xaml/README.md` documents how to install Windows App
SDK and run the resulting `.exe`. The C# host code is small (~150 lines —
mirroring `src/app/state.ts`) and lives next to the React host code so the
two stay in lockstep.

## 10. CLI integration

`mosaic-compile --backend xaml` accepts the same `--interface`,
`--layout`, `--style`, and `-o` flags as the React backend, plus:

| Flag                    | Default | Effect                                                |
|-------------------------|---------|-------------------------------------------------------|
| `--emit-project`        | `false` | Also write `.csproj` + `App.xaml(.cs)` + `MainWindow.xaml(.cs)`. |
| `--namespace <ns>`      | `Mosaic.Generated` | Top-level C# namespace for emitted types. |
| `--windows-app-sdk <v>` | `1.5`   | Pins the `<PackageReference Include="Microsoft.WindowsAppSDK" Version="..."/>` for `--emit-project`. |

Inside `code/packages/rust/mosaic-compile/src/main.rs`, the `--backend`
match grows a new `"xaml"` arm that constructs a
`mosaic_emit_xaml::XamlRenderer { … }`, validates the moslayout / mosstyle
pair through the shared `mosaic-analyzer`, and writes the file set.

## 11. Public API

```rust
// code/packages/rust/mosaic-emit-xaml/src/lib.rs

pub struct XamlRenderer { /* options */ }

pub struct XamlEmitResult {
    pub xaml:         String,       // {Component}.xaml
    pub code_behind:  String,       // {Component}.xaml.cs
    pub events:       String,       // {Component}.Event.cs
    pub project:      Option<ProjectFiles>, // Some(...) when emit_project=true
    pub component_name: String,
}

pub struct ProjectFiles {
    pub csproj:           String,
    pub app_xaml:         String,
    pub app_xaml_cs:      String,
    pub main_window_xaml: String,
    pub main_window_cs:   String,
    pub package_manifest: String,
}

#[derive(Debug, thiserror::Error)]
pub enum XamlEmitError {
    #[error("component name mismatch: mil/mll/msl disagree ({0:?})")]
    ComponentNameMismatch(Vec<String>),
    #[error("unsupported primitive: {0}")]
    UnsupportedPrimitive(String),
    #[error("slot type {0:?} has no XAML mapping")]
    UnmappableSlotType(String),
    #[error("style property {0:?} has no XAML setter mapping")]
    UnmappableStyleProperty(String),
}

pub fn from_pipeline(
    model:  &mosmodel_compiler::MosmodelComponent,
    layout: &moslayout_compiler::LayoutDef,
    style:  &mosstyle_compiler::StyleDef,
    options: &EmitOptions,
) -> Result<XamlEmitResult, XamlEmitError>;
```

Mirrors `mosaic_emit_react::pipeline::from_pipeline` so callers can swap
backends with one type change.

## 12. Test plan

The crate is tested at three levels:

1. **Unit tests** (don't require Windows): for each row of §3, build a
   tiny `LayoutDef` and assert the XAML output contains the expected
   control name and attribute set. Empty-emit, one-emit, three-emit
   cases for §6. Each row of §8's setter table.
2. **Integration tests** (don't require Windows): compile each of the
   `demo/visicalc/mosaic/*.mil/.mll/.msl` triples through `from_pipeline`
   and assert the XAML round-trips through a reference XAML parser
   (the lightweight one in `code/packages/rust/xaml-lexer/` if it exists,
   else just check well-formedness via `quick_xml`).
3. **Windows-only smoke tests** (`#[cfg(target_os = "windows")]`): the CI
   matrix's windows-latest job runs `dotnet build` on the
   `--emit-project` output for each VisiCalc component, asserting a
   successful build.

Target coverage: ≥90% for the pure emitter, smoke tests for the
Windows-only build.

## 13. Out of scope (tracked as follow-ups)

- **WPF backend.** WPF can fork most of this spec — same `Grid`,
  `StackPanel`, `Border`, dependency-property model — but uses
  `System.Windows.Controls` not `Microsoft.UI.Xaml.Controls` and runs on
  .NET Framework 4.x or .NET 6+. Future spec: `mosaic-emit-wpf.md`.
- **Avalonia / Uno backend.** Same XAML-flavoured surface; a thin
  fork of this spec switches the namespace and a few control names.
- **Animation / transition lowering.** The UI24+ spec does not yet model
  motion; the XAML backend will pick up `<Storyboard>` mapping when the
  IR catches up.
- **Localization.** `x:Uid` resource binding is not generated; emitted
  strings come from slot bindings only.
- **Themes beyond dark.** The UI26 demo only ships `.dark.msl`; multi-theme
  switching via `ResourceDictionary.ThemeDictionaries` is a follow-up
  once the demo gains a light theme.

## 14. Why WinUI 3 first, not WPF

WPF has the larger installed base but is in maintenance. The Mosaic
backend table is going to grow — WPF, Avalonia, Uno, MAUI — and they all
share 80%+ of the XAML vocabulary. Picking WinUI 3 as the first cut
locks in the *most modern* XAML dialect; downward forks to WPF (smaller
control set, different DependencyProperty registration syntax) are
easier than upward forks from WPF (which doesn't have
`ItemsRepeater`, `VisualStateManager` setters of the same shape, or the
`x:Bind` markup extension this spec leans on heavily).

The trade-off: WinUI 3 requires the Windows App SDK runtime on the user's
machine. For the VisiCalc plumbing, this is acceptable — the same demo
also has a no-runtime path via `mosaic-emit-win32` + Direct2D for users
who want a single self-contained `.exe`.
