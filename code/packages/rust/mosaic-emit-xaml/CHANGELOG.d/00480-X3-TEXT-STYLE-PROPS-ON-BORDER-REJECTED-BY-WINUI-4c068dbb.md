### X3 — text-style props on `<Border>` rejected by WinUI

`<Border>` doesn't have `Foreground` / `FontSize` / `FontWeight` /
`FontFamily` — those belong on the text content inside. The emitter
was placing every part-style property on the wrapping `<Border>`
unconditionally, so styled toolkit components like Alert and Badge
emitted invalid markup that XamlCompiler silently rejected.

Fixed in `emit_container` by partitioning the part-style fragment:
container-paint props (Background, BorderBrush, BorderThickness,
CornerRadius, Padding, Margin, Width, Height, *Alignment) stay on
the opening tag; text-style props move into a scoped
`<Border.Resources>` block as a `<Style TargetType="TextBlock">`
implicit style. WinUI's implicit-style resolution then applies
them to every `TextBlock` descendant inside the container.

This change also applies to the other emit_container call sites
(`Stack` → `<Grid>`), which have the same constraint.

Regression tests:
`box_partitions_style_between_border_and_textblock_resource`,
`box_without_text_style_emits_no_resources_block`,
`parse_style_fragment_round_trips_build_style_fragment`.

## [Unreleased] — UI31-K-xaml — `HostTable` RTL contract

The WinUI `HostTable` lowering (which produces a structural `<Grid>`
with `<Grid.RowDefinitions>` per section) now honours the UI31 §3.2
RTL contract via WinUI's `FrameworkElement.FlowDirection`:

- `dir: rtl` → `FlowDirection="RightToLeft"` on the `<Grid>`; flips
  column ordering of all descendant rows automatically.
- `dir: ltr` → `FlowDirection="LeftToRight"` — explicit-LTR for
  tables that should stay LTR inside an ambient-RTL `Page` (e.g.
  number-heavy spreadsheets).
- `dir: auto` → no attribute (spec semantic "let the host decide" =
  WinUI default of inheriting from the `Page`'s `FlowDirection`,
  typically set from `CultureInfo`).
- `dir: slot: layout-direction` → `FlowDirection="{x:Bind LayoutDirection}"`.
  The slot must evaluate to a `FlowDirection`; the slot name passes
  through `kebab_to_pascal_case` + `is_safe_identifier` so it can't
  smuggle malicious XAML through the binding path.
- Unknown keywords drop silently — the allow-list is the security
  gate. Test #6 feeds the literal payload `"RightToLeft\" Tag=\"pwn\""`
  (specifically shaped to break out of the attribute-value quoting)
  and asserts `Tag="pwn"` never reaches the output.

7 new tests cover the a11y gate (structural `<Grid>` with
`<Grid.RowDefinitions>` preserved — not a flat `<StackPanel>` mess),
the three allow-listed keywords (incl. the no-emit `auto` case),
the slot-ref binding through `{x:Bind PascalCase}`, the silent-drop
with attribute-injection payload, and a no-`dir` regression guard.
Total tests: 141 (was 134).

## [Unreleased] — UI29-4 `HostLink` + `HostTooltip` + `HostNumberInput` (U29-4-K-xaml)

Three new UI29-4 kernel primitives lower to native WinUI 3 widgets:

- **`HostLink` → `<HyperlinkButton NavigateUri="..." Content="..."/>`**.
  WinUI 3 ships `HyperlinkButton` specifically for clickable
  hyperlinks (vs `<Hyperlink>` which is the inline-text-flow
  variant). When `external: false` + `onActivate` are both bound,
  the lowering swaps to a `<Button Click="X_Click"/>` with a
  code-behind handler that dispatches the named emit (`href` flows
  into the dispatch payload as a string literal or `this.<Pascal>`
  property reference) — host's in-app router takes over.
- **`HostTooltip` → `<Border ToolTipService.ToolTip="text">child</Border>`**.
  The attached property hooks the tooltip directly to the wrapped
  element with native a11y wiring. `Border` is a layout pass-
  through (no padding/margin/background by default).
- **`HostNumberInput` → `<NumberBox Value="{x:Bind V, Mode=TwoWay}"
  Minimum Maximum SmallChange PlaceholderText IsEnabled
  ValueChanged>`**. WinUI 3's NumberBox is the native numeric
  input with built-in ± stepper, min/max validation, and locale-
  aware decimal parsing. `onChange` registers a `ValueChanged`
  handler that dispatches `XEvent.X(args.NewValue)` — the standard
  WinUI NumberBox event-arg shape (`args.NewValue` is the
  validated `double`).

6 new tests cover: HyperlinkButton with NavigateUri+Content, the
external-false + onActivate Button swap with Click handler +
href-in-payload dispatch, HostTooltip's Border + ToolTipService
wrap, bare NumberBox emission, min/max/step → Minimum/Maximum/
SmallChange mapping, and the ValueChanged code-behind handler
emission.

## [Unreleased] — UI29-2 `HostCheckbox` + `HostRadio` (U29-2-K-xaml)

Both new UI29-2 primitives lower to native WinUI / WPF widgets:

- `HostCheckbox` → `<CheckBox>` with `IsChecked` / `IsEnabled` / `Content`
  / `IsThreeState` / `Checked` + `Unchecked` events.
- `HostRadio`    → `<RadioButton>` with `IsChecked` / `IsEnabled` /
  `Content` / `GroupName` / `Checked` event (only — `Unchecked` is
  silent per UI29-2 §2.2's "onSelect = this radio was chosen").

Detailed prop handling:

- `checked: slot: c` → `IsChecked="{x:Bind C, Mode=OneWay}"`.
- `checked: true|false` → `IsChecked="True"` / `IsChecked="False"`.
- `disabled: slot: d` → `IsEnabled="{x:Bind Not(D)}"` (reuses
  HostButton's shared `Not(bool)` helper).
- `disabled: true|false` → `IsEnabled="False"` / `IsEnabled="True"`.
- `label: str|slot` → `Content="..."` / `Content="{x:Bind Label}"`.
- `HostCheckbox.indeterminate: slot|true` → `IsThreeState="True"`.
  The actual `IsChecked = null` transition is the host's job (WinUI
  doesn't have a "show as indeterminate" attribute, only the
  three-state-enabled flag).
- `HostCheckbox.onToggle: emit: onX` → registers TWO code-behind
  handlers — `<x>_Checked` dispatches `XEvent.X(true)` and
  `<x>_Unchecked` dispatches `XEvent.X(false)`. WinUI has no
  combined "toggled" event; the pair satisfies the kernel-canonical
  `onToggle(checked: bool)` signature exactly.
- `HostRadio.group: str|slot` → `GroupName="..."` / `GroupName="{x:Bind G}"`.
  WinUI auto-deselects siblings sharing `GroupName` when one
  `IsChecked` goes true — true radio-group behavior at the XAML
  level, no userland RadioGroup needed for v1.
- `HostRadio.value: str|slot` → flows into the C# dispatch payload
  as a string literal (escaped) or `this.<Pascal>` property ref.
- `HostRadio.onSelect: emit: onX` → registers ONLY a `<x>_Checked`
  handler that dispatches `XEvent.X(<value>)`. The `Unchecked` event
  is intentionally not wired so sibling-caused deselects don't
  trigger `onSelect`.

10 new tests cover: bare CheckBox / RadioButton blocks, checked-slot
binding, string label → Content, disabled → Not(bool) helper,
onToggle's Checked + Unchecked pair with matching bool payloads,
indeterminate → IsThreeState, bare RadioButton, group → GroupName,
onSelect with string-literal value, onSelect with slot-typed value.

Internal: added `escape_csharp_string` helper for embedding string
literals inside C# code-behind handler bodies (separate from
`escape_xaml_attr`, which is for XML-attribute contexts).

## [Unreleased] — `--emit-project` (B1, B2, B3 from demo catalog)

