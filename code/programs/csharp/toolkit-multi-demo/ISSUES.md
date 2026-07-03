# mosaic-emit-xaml gaps caught by the toolkit demo

Building `mosaic-pkg-toolkit` components through `mosaic-emit-xaml`
turned up four real code-gen bugs that the simpler hello-dialog
demo (#3925) didn't exercise. Each was one localised fix in
`code/packages/rust/mosaic-emit-xaml/src/pipeline.rs`.

**Status (2026-06-04):** all four emitter bugs (X1, X2, X3, X4)
have landed and `Alert.xaml` now regenerates cleanly with no
hand-patches required.  The remaining widget XAMLs
(`Badge.xaml`, `Button.xaml`, `Spinner.xaml`) still carry
hand-tweaks for layout preferences that aren't representable in
mosstyle yet (asymmetric padding, `HorizontalAlignment`,
`VerticalAlignment`, `Margin`); those are tracked as
**mosstyle gaps**, not emitter gaps — see the per-issue notes
below.  See also Issue X5 (`Spinner.xaml`'s nonexistent glyph)
which is open and tracked in [PR #5005](https://github.com/adhithyan15/coding-adventures/pull/5005).

The original hand-patches were tiny (under 20 lines total) and
were documented inline as
`<!-- hand-patched for WinUI 3 attribute validity -->`.  As the
emitter fixes land, each `.xaml` regenerates cleanly from the
`mosaic/` sources via `mosaic-compile --backend xaml
--emit-project`.

---

## Issue X1 — `border-radius` lowers to `BorderRadius`, but WinUI 3 calls it `CornerRadius`

**Symptom.** Every component with `border-radius : N` in its .msl
fails the XAML markup compile with no specific error — XamlCompiler
exits 1 silently.

**Repro.** Compile any toolkit component:

```sh
mosaic-compile --backend xaml \
  --interface code/packages/mosaic-pkg-toolkit/src/Button.mil \
  --layout    code/packages/mosaic-pkg-toolkit/src/Button.mll \
  --style     code/packages/mosaic-pkg-toolkit/src/Button.light.msl \
  --output    Button.xaml
```

The emitted XAML contains `<Button … BorderRadius="4" …/>`. WinUI 3
`Button` doesn't have a `BorderRadius` property; the equivalent
property is `CornerRadius`.

**Fix.** In
`code/packages/rust/mosaic-emit-xaml/src/pipeline.rs` change the
`border-radius` mapping in the style-attribute lookup from
`BorderRadius` to `CornerRadius`. One line.

**Patch applied in this demo.**

```sh
sed -i 's/BorderRadius=/CornerRadius=/g' winui/*.xaml
```

## Issue X2 — `x:Name="<Class>"` collides with the enclosing C# class

**Symptom.** Components whose root primitive's part name pascal-
cases to the same identifier as the component get a C# compile
error:

```
error CS0542: 'Button': member names cannot be the same as their enclosing type
```

This affects **Button**, **Checkbox**, **Input**, and **Radio** in
the current toolkit (their .mll files all wrap a single primitive
named after the component itself — `HostButton [ button ]`, etc.).

**Why.** WinUI's XAML compiler auto-generates a `private … Button
Button;` field for every `x:Name="Button"` element. When the
enclosing `partial class` is also called `Button`, the field name
collides with the class name.

**Fix.** In the emitter, detect collisions between the pascal-cased
part name and the component name. When they're identical, append
`Element` (or another safe suffix) to the `x:Name`. The `_Click`
handler stem stays the same because event-handler stems already use
the pascal-cased part name plus `_Click`.

**Patch applied in this demo.**

```sh
sed -i 's/x:Name="Button"/x:Name="ButtonElement"/g' winui/Button.xaml
```

## Issue X3 — Style properties forwarded onto `<Border>` that doesn't accept them

**Symptom.** Components that use a styled top-level `Box` (Alert,
Badge, …) emit XAML like:

```xml
<Border Padding="12" CornerRadius="4" Background="#cff4fc"
        Foreground="#055160" BorderThickness="1" BorderBrush="#b6effb"
        FontSize="14" FontWeight="500">
  <TextBlock Text="{x:Bind Message}"/>
</Border>
```

`Border` in WinUI 3 has `Background`, `BorderBrush`, `BorderThickness`,
`CornerRadius`, `Padding` — but **not** `Foreground`, `FontSize`,
or `FontWeight`. Those properties belong to the inner text content.
The XAML markup compiler rejects them.

**Fix.** In the `Box` → `Border` lowering path, partition the
collected style attributes:

| Property class | Stays on `Border`             | Moves to inner content    |
|---|---|---|
| Box chrome     | Background, BorderBrush,      | —                         |
|                | BorderThickness, CornerRadius,|                           |
|                | Padding                       |                           |
| Text style     | —                             | Foreground, FontSize,     |
|                |                               | FontWeight                |

When the inner content is a `TextBlock`, forward the text-style
properties to it (creating one if needed). When the inner content
is a more complex tree, this needs more thought — but the toolkit's
Box-styled components all wrap a single TextBlock today.

**Patch applied in this demo.** Hand-rewrote Alert.xaml and
Badge.xaml to keep box chrome on `<Border>` and put text style on
the inner `<TextBlock>`. Also fixed two unrelated issues caught
along the way:

- Alert's close-button had `Background="transparent"` (invalid in
  WinUI; should be `Transparent` (capital T) or `#00000000`).
- Spinner's `FontIcon Glyph="spinner"` referenced a non-existent
  glyph; replaced with the native `<ProgressRing IsActive="True"/>`
  which gives the animated spinner WinUI users expect.

The `FontIcon Glyph` issue is really a separate "Icon part-name
lookup" emitter gap — the .msl doesn't say *which* glyph, so the
emitter shouldn't be guessing one. A follow-up could introduce a
`glyph: text` slot on the toolkit Spinner that drives Symbol or
Glyph emission across backends.

---

## Issue X4 — `background: "transparent"` emits lowercase, but WinUI 3 needs `Transparent`

**Status.** **Fixed in [PR #5002](https://github.com/adhithyan15/coding-adventures/pull/5002).** Surfaced by Alert.xaml's close-button while auditing the hand-patches in this demo. The mosstyle `.msl` writes CSS-style lowercase color names (`background : "transparent"`), and the XAML emitter passed them through verbatim. WinUI 3's XAML markup compiler rejects `Background="transparent"` — the named-color table is PascalCase only, and the silent-exit-1 failure mode (no diagnostic on stderr) makes it hard to debug downstream.

**Fix.** `normalize_xaml_color_value(s)` in `build_style_fragment`, gated on `is_color_setter(key)`:

- Hex literals (`#…`) and markup extensions (`{x:Bind …}`) pass through.
- Already-PascalCased names (first char uppercase) pass through.
- All-lowercase ASCII names get their first letter uppercased (`transparent` → `Transparent`, `red` → `Red`).
- Anything else passes through verbatim — better to surface a stale value the markup compiler can flag than to silently mangle a user identifier.

Non-color setters (`FontSize`, `FontWeight`, `Padding`, …) are unaffected; the gate is by XAML setter name, not raw `.msl` property. `"normal"` for font-weight passes through (XAML accepts the lowercase form for non-color setters).

**Why this got past X1–X3.** X1–X3 were *property-name* issues (`BorderRadius` → `CornerRadius`, `Foreground` on `Border`). X4 is a *value-shape* issue. The emitter has always had a property-name translation table (`css_property_to_xaml_setter`); the X4 fix introduces the parallel value-normalization step.

## Issue X5 — `Icon[part]` lowers to `<FontIcon Glyph="…"/>` even when the named glyph doesn't exist in Segoe Fluent Icons

**Status.** **Fixed via Path A** in [PR #5005](https://github.com/adhithyan15/coding-adventures/pull/5005) (XAML) and [PR #5007](https://github.com/adhithyan15/coding-adventures/pull/5007) (Flutter). Both emitters now recognise the literal glyph name `"spinner"` and lower it to the backend-native progress indicator (`<ProgressRing IsActive="True"/>` on XAML, `CircularProgressIndicator()` on Flutter) before falling through to the standard `<FontIcon Glyph="…"/>` / `Icon(Icons.<name>)` path. Surfaced by Spinner.xaml during the X4 audit.

**Symptom.** The toolkit's Spinner.mll declares:

```
layout Spinner {
  Stack [ spinner ] {
    Icon [ spinner-glyph ] ( glyph : "spinner" )
  }
}
```

The XAML emitter lowers this to `<FontIcon Glyph="spinner" Foreground="#0d6efd" FontSize="24"/>`. WinUI 3's default `FontIcon` uses Segoe Fluent Icons, which has no glyph literally named `"spinner"` — the `<FontIcon/>` renders as an empty square, not a spinner. The hand-patch in this demo replaced it with `<ProgressRing IsActive="True" Width="24" Height="24" Foreground="#0d6efd"/>`, the WinUI-native animated spinner.

**Root cause.** Two layered issues:

1. **Mosaic kernel has no "spinner" concept.** The toolkit author wrote `Icon (glyph: "spinner")` as a semantic name, expecting backends to translate. The emitter has no translation table — it just emits the literal string as a font glyph.
2. **Even with the right glyph name, `FontIcon` would only render a static character, not the animated spinning ring that `ProgressRing` provides.** A glyph-name lookup table isn't enough; the toolkit's Spinner specifically wants an *animated* progress indicator.

**Two fix paths.**

**Path A (smaller, less general): `Icon (glyph: "spinner")` lowers to backend-native progress indicator.** The emitter recognizes a small set of semantic glyph names (`"spinner"`, `"loader"`, …) and lowers them to backend-native widgets instead of `<FontIcon/>`. On XAML: `<ProgressRing IsActive="True"/>` (with size from `width`/`height` style). On SwiftUI: `ProgressView()`. On Compose: `CircularProgressIndicator()`. On Flutter: `CircularProgressIndicator()`. On Qt: `BusyIndicator { running: true }`. On WebComponent/HTML: an animated SVG or `<progress>` element. **Drawback**: the semantic-name list grows over time as toolkit authors invent more names. Risks coupling the kernel emitter to userland naming conventions.

**Path B (cleaner, larger): `Spinner` becomes a kernel primitive (or a userland Mosaic package).** Either UI29 grows a `Spinner` primitive that backends know how to lower natively — analogous to `HostInput` / `HostTable` — or the toolkit ships a `Spinner.mil/.mll` whose Mosaic source decomposes into a `Box [ ring ] {}` shape with `style` keyword `is-spinner: true`, and a per-backend lowering rule recognizes that keyword and emits the native widget. **Drawback**: bigger design, touches the kernel primitive vocabulary. Right place to make the decision is during UI34 or UI35 (whichever cycle revisits primitive-vocabulary growth).

**Recommendation.** Ship Path A as a one-shot fix for `"spinner"` so the toolkit demo regenerates cleanly. Open a UI-spec issue to formalize Path B at the next vocabulary-revisit cycle.

**Patch applied in this demo.** Originally hand-rewrote `Spinner.xaml`'s `<Grid Width="24" Height="24"><FontIcon Glyph="spinner".../></Grid>` to `<ProgressRing IsActive="True" Width="24" Height="24" Foreground="#0d6efd"/>`. With #5005 merged, the regenerated `Spinner.xaml` emits `<Grid Width="24" Height="24"><ProgressRing IsActive="True" Foreground="#0d6efd" FontSize="24"/></Grid>` (sizing on the outer `Stack`-as-`Grid`, brush + size on the inner `ProgressRing` — `FontSize` is inherited from `Control` and harmless on `ProgressRing` since it draws no text). Hand-patch removed; the regen is now the committed file.
