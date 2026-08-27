# mosaic-emit-xaml — WinUI 3 / XAML backend for Mosaic

**Status:** Specification (draft, UI29-aligned rewrite)
**Layer:** UI / backend emitter
**Depends on:** UI23 (mosaic-pipeline), UI24 (mosaic-emit-dispatch), the three
IR compilers (mosmodel, moslayout, mosstyle), **UI29 (primitive kernel + userland
component packages)**
**Supersedes:** the pre-UI29 draft of this spec, which mapped coarse-grained
`Box / Row / Column / Grid / Input / Text / Image / Spacer / Scroll / Divider /
Stack / Icon` directly to XAML controls. That table is gone; this revision maps
the **fifteen UI29 kernel primitives** instead.
**Sibling backends:** UI20 (React/TSX), `mosaic-emit-win32.md` (pure Win32 +
Paint VM, Rust output), `mosaic-emit-swiftui`, `mosaic-emit-qt`,
`mosaic-emit-webcomponent`, `mosaic-emit-html`.
**Produces:** WinUI 3 C# source files (`.xaml` + `.xaml.cs` + project glue)

---

## 1. Purpose

Lower a three-file pipeline triple (`.mil` + `.mll` + `.msl`) targeting the
UI29 kernel to a WinUI 3 UserControl:

```
MyComponent.mil       \
MyComponent.desktop.mll → mosaic-compile --backend xaml → MyComponent.xaml + .xaml.cs
MyComponent.dark.msl   /
```

WinUI 3 remains the entry point for the XAML family (over WPF, UWP, Avalonia,
Uno) for the same three reasons as the original draft:

1. WinUI 3 is the actively developed Microsoft Windows UI framework
   (Windows App SDK 1.x line). WPF is in maintenance; UWP is being steered
   toward WinUI 3.
2. Its XAML dialect is the most modern; downward forks to WPF / Avalonia / Uno
   are easier than upward forks.
3. The UI29 kernel maps cleanly to WinUI 3's primitive set with one
   exception (`HostTable` — see §5).

What changed vs. the pre-UI29 draft:

- **No more `Grid` or `Input` in the mapping table.** Both are now userland
  components (`mosaic-pkg-grid::Grid`, `mosaic-pkg-input::Input`) that compose
  out of the kernel; the backend does not know about them at all.
- **New primitives** in the mapping table: `HostInput`, `HostButton`,
  `HostTable` (+ its four section sub-tags), `HostScroll`, `Stack`, `If`,
  `For`, `Else`.
- **No more bespoke 200-line Grid lowering.** The Grid that used to be in
  §4 is now `mosaic-pkg-grid`'s problem, compiled the same way as any other
  userland component.
- **Component references resolved at compile time.** When a `.mll` writes
  `Grid (rows: slot: data)`, the resolver (UI29 §4.4) looks `Grid` up in the
  manifest's dependencies and emits a XAML reference to the package's compiled
  `Grid.xaml` artifact — never a bespoke lowering.

What did *not* change:

- Output file shape (`.xaml` + `.xaml.cs` + `.Event.cs`, optionally a project
  triple).
- Slot → `DependencyProperty` translation (§8).
- Mosstyle → `ResourceDictionary` translation (§9).
- UI24 dispatch contract: one `Dispatch` event per `UserControl` (§7).
- `mosaic-compile --backend xaml` CLI flags (§10).
- Public `from_pipeline` API shape (§11).
- "Why WinUI 3 first" rationale (§14).

## 2. Output shape

Unchanged from the pre-UI29 draft. For a component named `MyComponent` the
backend writes:

```
MyComponent.xaml         — XAML markup; root is the lowered moslayout tree
MyComponent.xaml.cs      — code-behind: dependency properties for slots, the
                           UI24 Dispatch event, code-side handlers for any
                           `For` / `If` expression evaluation, InitializeComponent
                           boilerplate
MyComponent.Event.cs     — the discriminated event union (matches UI24 §3.1)
```

When invoked with `--emit-project` (default off), the backend also writes a
minimal WinUI 3 `.csproj`, `App.xaml(.cs)`, `MainWindow.xaml(.cs)`, and
`Package.appxmanifest`. With `--emit-project off` (default), only the
per-component triple is written and the caller integrates them into an
existing project.

When invoked on a userland component package (`mosaic-compile pkg <path>
--backend xaml` per UI29 §4.3), the backend emits one triple per exported
component plus a `<PackageName>.csproj` that bundles them as a referenceable
library.

### 2.1 Native runtime library bundling (issue #12026)

The generated `.csproj` carries a `CopyMosaicNativeHostLibraries` MSBuild
target that copies a bundled native runtime library next to the built
executable so `DllImport`/`NativeLibrary` can resolve it at launch. The
target's `Include` is the single, exact, known filename
`$(MSBuildProjectDirectory)\mosaic_app.dll` — never a glob — since
`mosaic-package-artifact-builder`'s `install_xaml_runtime_library` is the
only code path that ever legitimately places a file there, and it
validates that exact name before writing. A broader glob would copy any
`.dll` planted in the project directory next to the executable, where
the app's own load-from-own-directory convention would pick it up
directly. Bundling a runtime library is optional even with
`--emit-project` on, so the target carries
`Condition="Exists('$(MSBuildProjectDirectory)\mosaic_app.dll')"` to skip
cleanly rather than let `<Copy>` fail on a missing source file.

## 3. Mapping table — UI29 kernel primitives → XAML

The fifteen kernel primitives (UI29 §2.1). Every Mosaic backend must handle all
of them; this column is the WinUI 3 contract.

| UI29 kernel | XAML lowering | Notes |
|---|---|---|
| `Box`       | `<Border>` (or `<ContentPresenter>` when no padding/background) | Holds part-styled background, border, padding. |
| `Row`       | `<Grid>` + one `ColumnDefinition` per child | Children flow left→right; see §3.1 for sizing. |
| `Column`    | `<Grid>` + one `RowDefinition` per child | Children flow top→bottom; see §3.1 for sizing. |
| `Stack`     | `<Grid>` (single cell, all children) | Z-axis stacking; `Grid.ZIndex` on each child. |
| `Text`      | `<TextBlock>` | `Text` set from slot ref via `{x:Bind}` or literal. |
| `Image`     | `<Image Source="..."/>` | Slot ref → `{x:Bind}` on `Source`. |
| `Spacer`    | `<Rectangle/>` with `Width`/`Height` or `Grid.ColumnSpan` glue | Used inside a `Row`/`Column`. |
| `Divider`   | `<Border BorderThickness=".." />` thin band | Direction from props. |
| `Icon`      | `<FontIcon Glyph="..."/>` against Segoe Fluent Icons | |
| `If`        | `<ContentControl Visibility="{x:Bind ToVisibility(when)}">` wrapping the then-branch; an `Else` sibling wraps the else-branch with inverted visibility. See §6.2. | |
| `For`       | `<ItemsRepeater ItemsSource="{x:Bind each}">` + `<DataTemplate>` for the body. The `as:` / `index:` bindings surface as `DataContext` properties via a generated `RowVm` POCO. See §6.1. | |
| `HostInput` | `<TextBox>` | §4. |
| `HostButton`| `<Button>` | §4. |
| `HostTable` | Component-scoped `<local:*MosaicTable>` (`Grid` subclass) with native UIA Table/Grid peers for the canonical indexed dynamic shape; structural `<Grid>` fallback otherwise. See §5. |
| `HostScroll`| `<ScrollViewer>` | Vertical scroll by default; `direction: horizontal` prop swaps. |

Every emitted control gets an `x:Name` derived from its kebab-case
`part_name` PascalCased (`cell-grid` → `CellGrid`), so code-behind can refer
to it directly.

A node whose tag is *not* in the kernel and *not* a recognised sub-tag is a
**component reference** (UI29 §4.4); the resolver walks the package manifest
and emits a `<local:ComponentName .../>` or `<pkg:ComponentName .../>`
element, treating it as a XAML UserControl reference. See §11.

### 3.1 `Row`/`Column` flex lowering

`StackPanel` sizes to content and has no concept of distributing free
space, so a `Row`/`Column` that used to lower to it silently dropped
`flex-grow`, `justify-content`, `align-items`, and any percentage
`width`/`height` — those become expressible once the container is a
`<Grid>` with one `ColumnDefinition` (`Row`) or `RowDefinition`
(`Column`) per child.

**Sizing.** Each child gets an `Auto`-sized definition by default. A
child's definition becomes `"*"` (star) when either is true:

- the child's own mosstyle carries `flex-grow` (any non-zero value —
  today's authored sources only ever use `1`, so weighted star ratios
  are not yet implemented; add them if a real weighted case appears),
- the child carries a **main-axis** percentage size — `width: 100%` on
  a `Row`'s direct child, or `height: 100%` on a `Column`'s direct
  child. This is flexbox's own "let this child claim the remaining
  space" behavior in the main-axis direction, so it's treated
  identically to `flex-grow: 1` rather than as a separate mechanism.

A **cross-axis** percentage (`width: 100%` inside a `Column`,
`height: 100%` inside a `Row`) needs no special handling: a single-row
`Grid` cell already spans the container's full cross-axis extent, and
a child's default `HorizontalAlignment`/`VerticalAlignment` is
`Stretch` — matching flexbox's own default. The property is simply
dropped as before (percentage lengths have no WinUI `Double`
equivalent); the Grid's own sizing already produces the same visual
result.

**Positioning.** Every child (however many top-level XAML elements it
expands to — see below) gets a matching `Grid.Column="i"` (`Row`) or
`Grid.Row="i"` (`Column`) attached property, `i` counting up from `0`
in source order.

An `If`/`Else` pair is the one case where a single moslayout child
expands to **two** sibling top-level elements (§6.2 — both
`ContentControl`s always exist in the tree, toggled by `Visibility`).
Both get the *same* `Grid.Column`/`Grid.Row` index: they represent one
logical grid cell, mutually exclusive at runtime. A bare `If` (no
`Else`) and a `For` (always exactly one `<ItemsRepeater>` root) each
get one index, same as any other primitive.

**Cross-axis alignment (`align-items`).** Read off the *container's*
own mosstyle (not the child's). Only `center` is authored today, so
that's what's implemented: it becomes `VerticalAlignment="Center"` on
every child of a `Row` (cross-axis = vertical) or
`HorizontalAlignment="Center"` on every child of a `Column`
(cross-axis = horizontal), injected the same way as the
`Grid.Column`/`Grid.Row` attribute. `flex-start` / `stretch` / unset
need no attribute — `Stretch` is both WinUI's and flexbox's default.
`flex-end` and `baseline` are not yet mapped; add them when an
authored source needs them rather than guessing at the XAML shape
unverified.

`align: "center-vertical"` / `align: "center"` (not `align-items`) are
also recognized here, mapped to the identical `center` treatment
(issue #13164). `align` is not a real mosstyle property — per
`UI15-mosstyle.md` §1/§11 alignment belongs in `.mll`, and mosstyle's
grammar has no property whitelist to reject it — but it reached 30+
real `.msl` declarations anyway (TaskApp, VentureChrome, Calendar,
ProjectNav), always on a `Row`, always meaning exactly what
`align-items: center` means. Recognizing the value that's actually
authored fixes the real dropped-centering bug; the underlying
mosstyle-grammar gap stays open, tracked in #13164.

**Main-axis distribution (`justify-content`).** Only `space-between`
is authored today. It inserts a `"*"`-sized spacer definition
*between* each pair of children — `N` children get `N − 1` spacers, no
leading or trailing one, which is exactly `space-between`'s contract.
Each spacer is an empty element (the same shape `Spacer` already
emits: `<Rectangle Width="Auto" Height="Auto"/>`). Any other value (or
none) gets no distribution — children pack from the start, unchanged
from today's visual behavior. `space-around`, `space-evenly`, and
`center` distribution are explicitly deferred, tracked in
`code/programs/mosaic/task-app/BACKLOG.md`.

**`gap`.** Unchanged mapping (`gap` → the `Spacing`-shaped setter, §3
row unaffected), but the *attribute name* changes: `Grid` has no
`Spacing` property, so the Grid emitter renames it to
`ColumnSpacing` (`Row`) / `RowSpacing` (`Column`) — both added to
`Grid` in Windows App SDK 1.3+, safely inside this spec's pinned 1.5
floor (§10).

**Out of scope**, tracked separately: `flex-wrap` (no WinUI 3
`WrapPanel` — 8 uses repo-wide, its own follow-up), weighted
`flex-grow` values other than `1`, `align-items: flex-end`/`baseline`,
and `justify-content` values other than `space-between`.

### 3.2 Reporting dropped style properties (issue #12022)

Style lowering (`build_style_fragment`) drops a property with no record
at two points: the property name has no XAML setter at all
(`css_property_to_xaml_setter` returns `None`), or the value can't be
translated into a form the markup compiler accepts
(`translate_xaml_value` returns `None` — a non-100% percentage length,
an unsupported CSS unit, `currentColor`, …). Neither point used to
leave a trace; `pub fn dropped_style_properties(style: &StyleDef) ->
Vec<DroppedStyleProperty>` (`mosaic-emit-xaml::pipeline`) makes both
visible, one entry per dropped `(part, property, value, reason)`.

This is a *reporting* function, not a lowering change — nothing about
emitted XAML output is affected. `mosaic-package-artifact-builder`'s
degradation analyzer calls it per component/variant (the same shape it
already uses for `host_table_has_native_semantics` and friends) and
surfaces the results as `styleDegradations` in `mosaic-degradations.json`
— see that crate's docs for why this list is deliberately kept separate
from `degradations` (non-gating) rather than folded in.

**Exclusions**, so the report doesn't false-positive on properties §3.1
already handles through a side channel: `align-items`/`align`/
`justify-content` are excluded only when their value is one `FlexHints`
actually consumes (`"center"` for `align-items`/`align`, or
`"center-vertical"` for `align`; `"space-between"` for
`justify-content` — any other value IS reported, since nothing consumes
it); `flex-grow` is excluded unconditionally (fully boolean-handled
today, nothing recognisable as "lost"); `position`/`top`/`left` are
excluded only when `position` is literally `"absolute"` (see §3.3) —
`top`/`left` authored without it are meaningless in CSS too and stay
reported as before.

### 3.3 `position: "absolute"` → pinned `Margin` (issue #12028 item 4)

WinUI's `Canvas.Left`/`Canvas.Top` attached properties only take effect
when the immediate parent panel is a literal `<Canvas>` — but `Stack`
already lowers to `<Grid>` (§3, a z-axis stacking panel: every child
occupies the same cell unless given an explicit offset). Restructuring
every enclosing container into `Canvas` just to place a handful of
absolutely-positioned children would be a much larger change than the
problem calls for, so `absolute_position_style_attrs` reproduces the
same visual offset within the *existing* `Grid` instead:

```
position: "absolute"; top: 11; left: 4;
    →  Margin="4,11,0,0" HorizontalAlignment="Left" VerticalAlignment="Top"
```

A missing `top`/`left` defaults to `0` (CSS's own default for an
offset-less absolutely-positioned element). The three attrs are applied
*after* the normal per-property loop, overriding whatever `Margin`/
alignment another authored property might already have produced —
`position: absolute` takes full control of placement in CSS too, so the
two must never both apply. `position`/`top`/`left` are skipped in the
main loop when this fires, so they never reach
`dropped_style_properties` as a false-positive drop (§3.2).

Real usage: `mosaic/programs/task-app`'s bridge-arc brand mark
(`brand-post-left`/`brand-post-right`/`brand-arc`, three `Box`es
absolutely positioned inside one `Stack`) and its inline status-dot
icon compositions.

## 4. Host* primitives

### 4.1 `HostInput`

```xml
<TextBox x:Name="FormulaField"
         Text="{x:Bind Formula, Mode=TwoWay}"
         IsReadOnly="{x:Bind ReadOnly}"
         MaxLength="{x:Bind MaxLength}"
         PlaceholderText="Enter formula"
         TextChanged="HostInput_TextChanged"
         KeyDown="HostInput_KeyDown" />
```

| moslayout prop      | XAML attribute                | Notes |
|---------------------|-------------------------------|-------|
| `value: slot: v`    | `Text="{x:Bind V, Mode=TwoWay}"` | Two-way so `TextChanged` works without manual sync. |
| `read-only: slot: r`| `IsReadOnly="{x:Bind R}"`    | `read-only: true` keyword → literal `IsReadOnly="True"`. |
| `placeholder: "..."`| `PlaceholderText="..."`     | UI29-G3 string-literal prop. |
| `max-length: N`     | `MaxLength="N"`             | Numeric prop. |
| `multiline: true`   | adds `AcceptsReturn="True"` and `TextWrapping="Wrap"` to a `TextBox` (or swap to a multi-line `TextBox` if a future spec adds one). | |
| `onChange: emit: X` | `TextChanged` code-behind dispatches `X(e.NewText)`. | |
| `onCommit: emit: X` | `KeyDown` code-behind: on Enter, dispatch `X`. | |
| `onCancel: emit: X` | `KeyDown` code-behind: on Escape, dispatch `X`. | |
| `onFocus: emit: X`  | `GotFocus` dispatches `X`. | |

### 4.2 `HostButton`

```xml
<Button x:Name="SubmitButton"
        Content="{x:Bind Label}"
        IsEnabled="{x:Bind IsEnabled}"
        Click="HostButton_Click" />
```

| moslayout prop      | XAML attribute                 |
|---------------------|-------------------------------|
| `label: slot: l`    | `Content="{x:Bind L}"` (string slot → Content text). |
| `label: "..."`      | `Content="..."` literal. |
| `disabled: slot: d` | `IsEnabled="{x:Bind Not(D)}"` — the polarity flip lives in a tiny generated `Not(bool)` helper in the code-behind. |
| `disabled: true`    | literal `IsEnabled="False"`. |
| `onClick: emit: X`  | `Click` code-behind dispatches `X`. |

### 4.3 `HostScroll`

```xml
<ScrollViewer x:Name="Viewport"
              VerticalScrollBarVisibility="Auto"
              HorizontalScrollBarVisibility="Disabled">
  <!-- children -->
</ScrollViewer>
```

| moslayout prop          | XAML attribute |
|-------------------------|---------------|
| `direction: horizontal` | swap visibility flags (H=Auto, V=Disabled). |
| `direction: both`       | both Auto. |
| `direction: vertical`   | default (V=Auto, H=Disabled). |

### 4.4 `HostLink` (issue #12038 — URI scheme validation)

`href` lowers to `HyperlinkButton.NavigateUri`, which WinUI hands to the
OS shell launcher on click. Layout/style source is a trust boundary (a
third-party Mosaic package), so the scheme is validated rather than only
XML-escaped — escaping does nothing to make an unsafe scheme (`file:`, a
UNC path, a registered custom protocol) safe. Allowlist: `http`,
`https`, `mailto` (case-insensitive scheme match).

| `href` form | Behavior |
|---|---|
| literal string, allowed scheme | `NavigateUri="..."` (XML-attr-escaped), unchanged from before |
| literal string, disallowed/no scheme | compile-time `PipelineEmitError::UnsafeUriScheme` — rejected, no XAML emitted |
| `slot: u` | `NavigateUri="{x:Bind SafeNavigateUri(U), Mode=OneWay}"` — a generated shared helper validates the runtime value; a disallowed/unparseable scheme yields `null`, and `HyperlinkButton` simply doesn't navigate on click |

`external: false` (the in-app-routing path, `<Button>` + `Click` handler)
never emits `NavigateUri` at all and is unaffected.

## 5. `HostTable` and its section sub-tags

WinUI 3 has no core DataGrid. The emitter therefore preserves Mosaic's explicit
row/column composition as a `Grid`/`ItemsRepeater` visual tree. For the canonical
indexed dynamic UI31 shape, the root, headers, and cells are component-scoped
controls whose automation peers implement native UIA Table/Grid and
TableItem/GridItem provider patterns. This avoids a deprecated third-party
control while retaining arbitrary interactive content inside each cell.

The native semantic path requires exactly one header row containing an indexed
`For` over a slot, and one indexed body `For` over a rows slot whose `Row`
contains an indexed cell `For` over that row alias. Ambiguous structures,
footers, or missing stable indices keep the structural visual fallback and are
reported as `accessibility.table-semantics-missing` by native-completeness
analysis.

### 5.1 Section sub-tags

UI29 freezes four section sub-tags. The emitter recognises each by tag name
and lowers it as part of the parent `HostTable`'s layout:

| sub-tag             | role                                              |
|---------------------|---------------------------------------------------|
| `HostTableColGroup` | one or more `Col` children carrying widths/styles |
| `HostTableHead`     | one or more `Row` children — header cells (`<th>` semantics) |
| `HostTableBody`     | one or more `Row` children — data cells |
| `HostTableFoot`     | one or more `Row` children — footer cells |

Each section appears at most once per `HostTable` (debug-assertion in the
emitter; release builds keep the first occurrence). An empty `HostTable`
with no sections lowers to an empty `<Grid/>`.

### 5.2 Layout shape

For the canonical indexed dynamic shape, the emitter produces this abridged
structure:

```xml
<local:SheetMosaicTable RowCount="{x:Bind BodyRows.Count}"
                        ColumnCount="{x:Bind HeadCells.Count}"
                        AutomationProperties.Name="sheet table">
  <Grid.RowDefinitions>
    <RowDefinition Height="Auto"/>
    <RowDefinition Height="*"/>
  </Grid.RowDefinitions>

  <ItemsRepeater Grid.Row="0" ItemsSource="{x:Bind HeadCells}">
    <ItemsRepeater.ItemTemplate>
      <DataTemplate x:DataType="local:HeadCellVm">
        <local:SheetMosaicTableHeaderCell
            Column="{x:Bind Index}"
            Header="{x:Bind Value}"
            AutomationProperties.Name="{x:Bind Value}">
          <!-- authored header-cell subtree -->
        </local:SheetMosaicTableHeaderCell>
      </DataTemplate>
    </ItemsRepeater.ItemTemplate>
  </ItemsRepeater>

  <ScrollViewer Grid.Row="1">
    <ItemsRepeater ItemsSource="{x:Bind BodyRows}">
      <ItemsRepeater.ItemTemplate>
        <DataTemplate x:DataType="local:BodyRowVm">
          <ItemsRepeater ItemsSource="{x:Bind Cells}">
            <ItemsRepeater.ItemTemplate>
              <DataTemplate x:DataType="local:CellVm">
                <local:SheetMosaicTableCell
                    Row="{x:Bind RowIndex}"
                    Column="{x:Bind Index}"
                    Header="{x:Bind MosaicTableHeader}"
                    Value="{x:Bind Value}"
                    AutomationProperties.Name="{x:Bind MosaicTableName}">
                  <!-- authored interactive cell subtree -->
                </local:SheetMosaicTableCell>
              </DataTemplate>
            </ItemsRepeater.ItemTemplate>
          </ItemsRepeater>
        </DataTemplate>
      </ItemsRepeater.ItemTemplate>
    </ItemsRepeater>
  </ScrollViewer>
</local:SheetMosaicTable>
```

The emitter generates matching row view-model projections in the code-behind.
They carry stable indices plus computed header and accessible-name properties;
the existing projection machinery materialises them from bound slot values.

### 5.3 Accessibility contract

The generated table peer publishes `AutomationControlType.Table`, row and
column counts, `IGridProvider.GetItem`, and column-header providers. Realized
cell peers publish `AutomationControlType.DataItem`, their row/column/span,
containing-grid provider, column-header association, and an accessible name
derived from header, row, and value. Header peers publish
`AutomationControlType.HeaderItem`. Cell controls are keyboard focusable and
arrow keys move to adjacent realized cells.

The fallback `<Grid>` does not claim these patterns. Capability analysis calls
the emitter's same conservative shape predicate, so a permissive artifact gets
an explicit degradation and a strict artifact fails rather than overstating UIA
fidelity.

## 6. New grammar — `If`, `For`, and `Expr`

UI29 §3 adds three grammar features to moslayout. Each gets a deterministic
XAML lowering.

### 6.1 `For (each: <expr>, as: <name>, index: <name>?) { <children> }`

Lowers to an `<ItemsRepeater>` whose `ItemsSource` is the `each:` expression
and whose `ItemTemplate` is a `<DataTemplate>` over a generated `RowVm`
type that mirrors the body's bound names.

```moslayout
For (each: slot: viewport-rows, as: row, index: r) {
  Row [data-row] {
    Text (content: row[c])
  }
}
```

…lowers to:

```xml
<ItemsRepeater ItemsSource="{x:Bind ViewportRows}">
  <ItemsRepeater.ItemTemplate>
    <DataTemplate x:DataType="local:ViewportRowVm">
      <!-- "row" is the DataTemplate's DataContext;
           "r" is exposed via a generated Index property on RowVm -->
      <Grid Style="{StaticResource DataRowStyle}">
        <Grid.ColumnDefinitions>
          <ColumnDefinition Width="Auto"/>
        </Grid.ColumnDefinitions>
        <TextBlock Grid.Column="0" Text="{x:Bind GetCell(R, C)}"/>
      </Grid>
    </DataTemplate>
  </ItemsRepeater.ItemTemplate>
</ItemsRepeater>
```

The code-behind generates one `RowVm` per `For` block, naming it from the
`as:` binding (`row` → `RowVm`), with properties for the bound name itself
(`Row`) and the optional index (`R`). Expressions inside the body that use
`row[c]` lower through a code-behind helper `GetCell(int r, int c)` because
XAML `{x:Bind}` does not support indexer syntax. See §6.3 for the helper
shape.

### 6.2 `If (when: <expr>) { <then> } [Else { <else> }]`

Lowers to two `<ContentControl>`s whose `Visibility` is bound to the
`when:` expression and its negation. Both branches always exist in the
visual tree (just one is collapsed); this is cheap and matches WinUI 3's
idiomatic conditional-rendering pattern.

```moslayout
If (when: slot: editable && slot: is-editing) {
  HostInput (value: slot: value, onCommit: emit: onCommit)
}
Else {
  Text (content: slot: value)
}
```

…lowers to:

```xml
<ContentControl Visibility="{x:Bind IsEditableAndEditing, Converter={StaticResource BoolToVis}}">
  <TextBox Text="{x:Bind Value, Mode=TwoWay}" KeyDown="HostInput_KeyDown"/>
</ContentControl>
<ContentControl Visibility="{x:Bind IsNotEditableOrEditing, Converter={StaticResource BoolToVis}}">
  <TextBlock Text="{x:Bind Value}"/>
</ContentControl>
```

The `IsEditableAndEditing` / `IsNotEditableOrEditing` properties are
generated in the code-behind from the expression (see §6.3). A shared
`BoolToVisibilityConverter` is added once per emitted project — the
emitter writes it into `Converters/BoolToVisibilityConverter.cs` the first
time an `If` appears.

### 6.3 Expression evaluation

UI29 §3.3 stores expressions as raw source-text strings inside
`LayoutPropValue::Expr`. WinUI 3's `{x:Bind}` markup extension supports a
restricted Path syntax — bare names, dotted access (`row.value`), method
calls — but it does *not* support indexer brackets (`row[c]`),
short-circuiting logical operators, or comparison expressions.

The pragmatic split:

1. **`{x:Bind}`-able**: bare slot / for-bound name, dotted member access
   (`row.value`), boolean literals. These compile to a direct
   `{x:Bind expr}` in the XAML.
2. **Not `{x:Bind}`-able**: indexer (`row[c]`), comparisons, logical
   operators, anything else from UI29 §3.3's grammar. These lower to a
   *named code-behind helper* — the emitter generates a method on the
   `UserControl` (or on the `RowVm`, depending on the binding scope) and
   the XAML references it: `{x:Bind GetCell(Row, C)}` for `row[c]`,
   `{x:Bind IsEditableAndEditing}` for `slot: editable && slot: is-editing`.

The expression-to-helper translation lives in a small `ExprLowerer` module
of the backend. For v1 it supports exactly the grammar UI29 §3.3 freezes:
`||`, `&&`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `!`, member access `.NAME`,
indexer `[expr]`. Each unsupported construct surfaces as a clear emit error
("expression form X not yet supported on the WinUI 3 backend") rather than
silently producing wrong XAML.

The helper method names use a deterministic hash of the expression source so
that two identical expressions in the same component dedupe to one helper.

### 6.3.1 Sites that consume `Expr` (issue #12126)

Every `find_prop_value(node, "...")` match that lowers a prop to an XAML
attribute must route the `LayoutPropValue::Expr` arm through this
lowering and must be **exhaustive** over `LayoutPropValue`'s variants (no
`_` catch-all) — a bare `_ => {}` silently drops the attribute for any
value not already given an explicit arm, which renders a control with
that attribute simply absent rather than raising a diagnostic. As of
#12126 the following sites are fixed and exhaustive: `Text.content`
(pre-existing, the template for the others), `Text.a11y-label`,
`HostSlider.a11y-label`, `HostTooltip.text`, `HostInput.value`,
`HostInput.placeholder`, `HostNumberInput.value`, `HostDialog.title`,
`Image.src`, and the four content-control `label` matches
(`HostButton`/`HostCheckbox`/`HostRadio`/`HostLink`, fixed earlier in
#12045/#12121). `Mode` follows what the *source* can support, not what
the target attribute allows: every `Expr` arm uses `Mode=OneWay`, even at
otherwise-`TwoWay` targets (`HostInput.value`, `HostNumberInput.value`),
because an indexer expression lowers to a `Helper` — a C# method call,
not an assignable path, so `Mode=TwoWay` would not compile. A helper's
C# return type is always `string`; WinUI's compiled-binding implicit
conversions (verified against a real `dotnet build`, not assumed) make
this bind correctly even at non-`string` targets like
`HostNumberInput.value`'s `double` and `Image.src`'s `ImageSource`.
Issue #13040 closed the remaining ~15 sites with the same bare-catch-all
shape, in two treatments:

- **Full `Expr` support, same pattern as above** (13 sites): `disabled:`
  → `IsEnabled` (negated) on `HostButton`/`HostCheckbox`/`HostRadio`/
  `HostNumberInput`/`HostSlider`; `checked:` → `IsChecked` on
  `HostCheckbox`/`HostRadio`; `read-only:` → `IsReadOnly` on
  `HostInput`; `indeterminate:` → `IsThreeState` on `HostCheckbox`;
  `HostSurface.content` → `Content`; `Icon.glyph`/`.name` → `Glyph`;
  `HostRadio.group` → `GroupName`; `HostTable.dir` → `FlowDirection`
  (both the native-shape and non-native fallback lowering paths — the
  existing `rtl`/`ltr` keyword allow-list there is a gate against an
  *unrecognized static keyword*, not a reason to reject `Expr`
  wholesale, since `lower_expr_for_xbind` never splices raw text). The
  five negated `disabled:` sites route through a new
  `disabled_expr_xbind_path`, composing the lowered expression with the
  same shared `Not(bool)` helper `disabled_slot_xbind_path` already
  registers for `SlotRef`; inside a `For` template scope it returns
  `Unsupported` rather than guessing, since composing that negation
  with whatever `register_template_helper_binding` already projected
  onto the row VM isn't implemented or verified.
- **Exhaustive but `Expr` deliberately deferred** (2 sites):
  `host_link_href_payload_expr`, used both for `HostLink`'s in-app
  `Click` payload and `HostRadio`'s `onSelect` `value:` payload. These
  build a bare C# expression spliced into a code-behind
  `Dispatch?.Invoke(...)` call at click time — no for-scope awareness,
  `SlotRef` resolves via `this.<Pascal>`. Real per-row `Expr` support
  would need the same sender-`DataContext`-cast-to-row-VM-type codegen
  `host_button_click_payload_expr` already does for its own params, a
  materially more novel shape with no existing precedent — a real
  feature addition, not an exhaustiveness fix. Both functions are now
  exhaustive over `LayoutPropValue` (no `_`); `Expr` returns
  `PipelineEmitError::UnsupportedExpression` instead of silently
  falling back to an empty-string payload.

## 7. Event dispatch contract (UI24)

Unchanged from the pre-UI29 draft. For a component with `n ≥ 0` emits the
emitter writes `{Component}.Event.cs`:

```csharp
namespace Mosaic.Generated;

public abstract record GridEvent
{
    public sealed record Navigate(int Row, int Col) : GridEvent;
    public sealed record EditCommit(string Value)   : GridEvent;
    // ...
}
```

And in `{Component}.xaml.cs`:

```csharp
public sealed partial class Grid : UserControl
{
    /// <summary>Fires once for every emit declared in the .mil interface.</summary>
    public event EventHandler<GridEvent>? Dispatch;

    // DependencyProperties for each slot — see §8.
}
```

The empty-emit case still emits the abstract record with zero concrete
variants (matches `export type FooEvent = never` from UI24 §3.1).

When a `For` body contains a node with an `onClick: emit: X` (or any other
emit binding), the dispatched event carries the bound names as named
parameters: `Click` handler dispatches `X(Row, C)` where `Row`/`C` are the
`For`'s bindings in scope at the call site. This is how the React backend
already wires it; XAML follows suit.

## 8. Slot → `DependencyProperty`

Unchanged from the pre-UI29 draft. Every slot in the `.mil` becomes a public
`DependencyProperty`:

| mosmodel slot type   | C# property type                              |
|----------------------|------------------------------------------------|
| `text`               | `string`                                       |
| `number`             | `double`                                       |
| `bool`               | `bool`                                         |
| `color`              | `Windows.UI.Color`                             |
| `image`              | `Microsoft.UI.Xaml.Media.Imaging.ImageSource`  |
| `node`               | `Microsoft.UI.Xaml.UIElement`                  |
| `list<T>`            | `IReadOnlyList<T-mapped>`                      |
| `list<list<T>>`      | `IReadOnlyList<IReadOnlyList<T-mapped>>`       |

DependencyProperty registration is hand-rolled per slot so bindings are
observable. The property name is kebab→PascalCase. `Required: true` becomes
a `[Required]` attribute; bindings still compile and a runtime warning logs
on first render when the property is its default.

## 9. Style application (mosstyle)

The `.msl` source's base `part` properties lower to native attributes and
scoped text styles. Values are XML-escaped (`escape_xaml_attr` — `&`, `"`,
`<`, `>`) at the point the base attribute fragment is built, the same
guarantee the `<Setter Value="...">` path always had (#12025). This is the
single production write path for every base-`part`-derived attribute
across every primitive, so nothing downstream needs its own escaping.

On native Host controls, a `state-when-*` layout
predicate plus its matching MSL state block lowers to a WinUI
`StateTrigger` and `VisualState` setter.

Transitions use one `VisualStateGroup` per property. This preserves MSL's
property-scoped durations and curves even though WinUI's generated transition
duration applies to every property changed by one group. A part-level
transition supplies the default entry/exit curve; a state-local transition
with the same property overrides entry into that state. Named easing curves
lower to native quadratic easing functions. Arbitrary `cubic-bezier(...)`
currently uses WinUI `CubicEase`; exact control points require a future
Composition-API lowering. The groups attach to a transparent `Grid` that is
the generated root's first visual child, which WinUI requires for automatic
declarative trigger evaluation.

UI29 §8 open question 6 (theming across packages) is resolved here as:
when a host's `.msl` overlays a package's `.msl`, the host's
`ResourceDictionary` is merged with `Order="After"`, so host setters win on
key collision. Setting this up requires the host project to include the
package's `Themes/Generic.xaml` *first* and the host's overrides second.

## 10. CLI integration

`mosaic-compile --backend xaml` accepts the standard pipeline flags
(`--interface`, `--layout`, `--style`, `-o`) plus:

| Flag                        | Default | Effect |
|-----------------------------|---------|--------|
| `--emit-project`            | `false` | Also write `.csproj`, `App.xaml(.cs)`, `MainWindow.xaml(.cs)`, `Package.appxmanifest`. |
| `--namespace <ns>`          | `Mosaic.Generated` | Top-level C# namespace. |
| `--windows-app-sdk <v>`     | `1.5`   | Pins the `<PackageReference>` version. |
| `--package-mode` *(new)*    | `false` | Treat the input as a UI29 userland package (`mosaic-package.toml` next to the source). Emits one XAML triple per exported component plus a library `.csproj`. |

Inside `code/packages/rust/mosaic-compile/src/main.rs`, the `--backend`
match adds a `"xaml"` arm that constructs a `mosaic_emit_xaml::XamlRenderer`
and writes the file set.

## 11. Component references (UI29 §4.4)

When the layout walker encounters a node whose tag is *not* in the kernel
and *not* a recognised sub-tag, it is a component reference. The XAML
backend resolves it as follows:

1. Look up the tag name in the active manifest's `[dependencies]` section.
   If absent: emit `UnknownComponent` error.
2. For each matched package, verify the tag is in its `[components].exports`.
3. Read the package's compiled artifact from `mosaic-pkg-{name}/dist/xaml/`.
4. Emit a XAML reference using the package's emitted namespace:
   ```xml
   xmlns:grid="using:Mosaic.Package.Grid"
   ...
   <grid:Grid Rows="{x:Bind State.ViewportRows}"
              Headers="{x:Bind State.ColumnHeaders}"
              ColumnWidths="{x:Bind State.ColumnWidths}"
              Dispatch="GridControl_Dispatch"/>
   ```
5. The host's `.xaml.cs` adds a `using` for the package's namespace and
   wires the `Dispatch` event to its own `HandleEvent` reducer.

The package's compiled artifact is just a normal WinUI 3 library: a
`UserControl` per exported component with its own slots /
`DependencyProperty`s / `Dispatch` event. There is nothing
Mosaic-specific about how it integrates — it's a `PackageReference` from
the host's `.csproj`.

For props whose values are component references' typed parameters, the host
binds via standard `{x:Bind}`. There's no special "Mosaic ABI" — the
package is a regular .NET library that happens to have been generated from
moslayout source.

## 12. VisiCalc plumbing — proof under the new paradigm

Once `mosaic-pkg-grid` (UI29 §5.2) lands, the VisiCalc demo's Windows
plumbing simplifies dramatically. The host project becomes:

```
code/programs/typescript/visicalc/
  windows/xaml/
    VisiCalc.csproj           — generated (when --emit-project)
                                 includes a <ProjectReference> to
                                 mosaic-pkg-grid's compiled XAML library
                                 and mosaic-pkg-input's
    App.xaml, App.xaml.cs     — generated host shell
    FormulaBar.xaml(.cs)(.Event.cs)  — generated; FormulaBar is the demo's
                                       one custom component, composed of
                                       Row + Text + HostInput
    State.cs                  — hand-written reducer; same shape as
                                 src/app/state.ts in the React demo
    MainWindow.xaml(.cs)      — instantiates the package's Grid + the
                                 local FormulaBar, wires Dispatch events
                                 into State.HandleEvent
  windows/build.ps1           — calls `mosaic-compile pkg
                                 .../mosaic-pkg-grid/ --backend xaml`,
                                 then the same for FormulaBar, then
                                 `dotnet build VisiCalc.csproj`
```

The demo doesn't have its own `Grid.mil` / `Grid.mll` / `Grid.dark.msl`
anymore — those moved to `mosaic-pkg-grid`. The C# host code is ~150
lines, mirroring `src/app/state.ts`.

## 13. Public API

```rust
// code/packages/rust/mosaic-emit-xaml/src/lib.rs

pub struct XamlRenderer { /* options */ }

pub struct XamlEmitResult {
    pub xaml:        String,                 // {Component}.xaml
    pub code_behind: String,                 // {Component}.xaml.cs
    pub events:      String,                 // {Component}.Event.cs
    pub project:     Option<ProjectFiles>,   // when --emit-project
    pub component_name: String,
    /// One entry per `For` block — the generated RowVm C# source. The
    /// project's .csproj includes these as compile items. Empty when the
    /// component contains no `For` blocks.
    pub for_view_models: Vec<EmittedFile>,
    /// One per `If` whose expression isn't `{x:Bind}`-able — the
    /// computed-property helper C# source. Same lifecycle as RowVms.
    pub if_helpers: Vec<EmittedFile>,
}

pub struct EmittedFile { pub filename: String, pub source: String }
pub struct ProjectFiles { /* csproj + App.xaml(.cs) + MainWindow + manifest */ }

#[derive(Debug, thiserror::Error)]
pub enum XamlEmitError {
    #[error("component name mismatch: mil/mll/msl disagree ({0:?})")]
    ComponentNameMismatch(Vec<String>),

    #[error("unsupported primitive: {0} (not in UI29 kernel; is it missing from a package manifest?)")]
    UnsupportedPrimitive(String),

    #[error("unknown component reference: {0} (not in manifest dependencies)")]
    UnknownComponent(String),

    #[error("expression form not yet supported on WinUI 3 backend: {0}")]
    UnsupportedExpression(String),

    #[error("slot type {0:?} has no XAML mapping")]
    UnmappableSlotType(String),

    #[error("style property {0:?} has no XAML setter mapping")]
    UnmappableStyleProperty(String),

    #[error("HostTable has duplicate {0} section sub-tag")]
    DuplicateTableSection(String),
}

pub fn from_pipeline(
    model:    &mosmodel_compiler::MosmodelComponent,
    layout:   &moslayout_compiler::LayoutDef,
    style:    &mosstyle_compiler::StyleDef,
    manifest: Option<&mosaic_compile::PackageManifest>,
    options:  &EmitOptions,
) -> Result<XamlEmitResult, XamlEmitError>;
```

The signature mirrors the other UI29-aware backends (`mosaic-emit-react`,
`mosaic-emit-swiftui`). The new `manifest` argument carries the resolved
package dependencies for §11; it is `None` when compiling a single component
in isolation, in which case any non-kernel tag becomes `UnknownComponent`.

## 14. Test plan

1. **Unit tests** (cross-platform, pure code-gen):
   - Each row of §3 → assert the XAML output contains the expected control.
   - Empty / one-emit / three-emit cases for UI24 (§7).
   - Each row of §9's setter table.
   - `HostTable` section sub-tag combinations: empty, head-only, body-only,
     full quad, duplicate section (error).
   - `For` over a slot, over a `For`-bound name, over an expression with
     index access. RowVm naming and dedup.
   - `If` with `{x:Bind}`-able vs. helper-requiring expressions.
   - Component-reference lowering with one and multiple manifest dependencies.
   - `UnknownComponent` and `UnsupportedExpression` errors.

2. **Snapshot tests**: for each kernel primitive, assert the emitted XAML
   matches a frozen `.xaml.snap` file. UI29's "kernel is frozen" promise is
   the same promise we rely on for snapshot stability.

3. **Cross-backend parity tests**: compile a small but kernel-comprehensive
   `.mll` fixture through every backend and assert all emitted code passes
   that backend's compiler. The same fixture verifies that the XAML output
   matches semantic behaviour the React/SwiftUI outputs already pass.

4. **Windows-only smoke tests** (`#[cfg(target_os = "windows")]`): on
   `windows-latest` CI, `dotnet build` each `--emit-project` output for
   `mosaic-pkg-grid`, `mosaic-pkg-input`, and the VisiCalc demo. Exit code 0
   means the WinUI 3 SDK accepted the emitted XAML + C#.

Target coverage: ≥90% for the pure emitter; smoke tests for the
Windows-only build.

## 15. Out of scope (tracked as follow-ups)

- **WPF backend.** Forks of this spec with a different namespace
  (`System.Windows.Controls` vs `Microsoft.UI.Xaml.Controls`) and
  DependencyProperty registration differences. New spec: `mosaic-emit-wpf.md`.
- **Avalonia / Uno backends.** Same XAML surface, different namespace,
  Linux/macOS reach.
- **Exact cubic-bezier easing.** WinUI XAML does not expose arbitrary cubic
  control points through `EasingFunctionBase`; exact curves require a
  Composition-API lowering.
- **Template-local visual states.** Host controls inside `For` live in a
  DataTemplate namescope and need per-template VisualState placement rather
  than the root-level groups used for top-level controls.
- **Localization.** `x:Uid` resource binding is not generated.
- **Multi-theme switching.** UI29 §8 open question 6 — the
  `ResourceDictionary.ThemeDictionaries` mechanism is supported, but the
  demo only ships `.dark.msl` today.
- **Expanded expression grammar.** Arithmetic (`+`/`-`/`*`/`/`), ternary,
  string concat, function calls — added when a userland package needs them.
- **Hot-reload integration.** WinUI 3 supports XAML hot reload via Visual
  Studio; wiring the emitter to participate is a follow-up.

## 16. Why WinUI 3 first, not WPF

Unchanged from the pre-UI29 draft. WPF has the larger installed base but is
in maintenance. The Mosaic XAML-family backend table will grow — WPF,
Avalonia, Uno, MAUI — and they all share 80%+ of the XAML vocabulary.
WinUI 3 locks in the most modern dialect; downward forks are easier than
upward forks (WPF doesn't have `ItemsRepeater`, modern
`VisualStateManager` setter shapes, or `{x:Bind}` with the same
typed-DataTemplate fidelity).

The trade-off is unchanged: WinUI 3 requires the Windows App SDK runtime.
For the VisiCalc demo this is acceptable — the demo also has a
no-runtime path via `mosaic-emit-win32` + Direct2D for users who want a
single self-contained `.exe`.

## 17. Relationship to UI29

This spec is the WinUI 3 instance of UI29-K (kernel-implementing emitters).
The other instances are tracked in UI29 §6:

| ID            | Backend                  | Status |
|---------------|--------------------------|--------|
| U29-K-react   | mosaic-emit-react        | merged |
| U29-K-swiftui | mosaic-emit-swiftui      | merged |
| U29-K-qt      | mosaic-emit-qt           | merged |
| U29-K-webcomp | mosaic-emit-webcomponent | merged |
| U29-K-html    | mosaic-emit-html         | merged |
| **U29-K-xaml**| **mosaic-emit-xaml**     | **this spec — not yet implemented** |

The implementation PR sequence mirrors the React backend's UI29-K rollout:
one PR per kernel primitive group (Stack/If/For first, then Host* family),
each landing on `mosaic-emit-xaml`'s skeleton (which itself is a single
"scaffold" PR landing the crate, the kernel primitive shapes, and stubbing
the Host* primitives).

The implementation PR sequence:

```
PR-1  mosaic-emit-xaml: scaffold crate, Box/Row/Column/Stack/Text/Image
                       /Spacer/Divider/Icon lowering, Event union, slot DPs
PR-2  mosaic-emit-xaml: If / Else / For lowering + ExprLowerer
PR-3  mosaic-emit-xaml: HostInput, HostButton, HostScroll
PR-4  mosaic-emit-xaml: HostTable with section sub-tags
PR-5  mosaic-emit-xaml: component-reference resolver, --package-mode
PR-6  mosaic-pkg-grid → compile through xaml backend; VisiCalc Windows demo
```

PR-1 through PR-5 produce no end-user-visible output beyond the WinUI 3
sample; PR-6 is the first thing a user can run.
