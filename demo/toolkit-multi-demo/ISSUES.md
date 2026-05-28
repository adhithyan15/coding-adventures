# mosaic-emit-xaml gaps caught by the toolkit demo

Building `mosaic-pkg-toolkit` components through `mosaic-emit-xaml`
turned up three real code-gen bugs that the simpler hello-dialog
demo (#3925) didn't exercise. Each is one localised fix in
`code/packages/rust/mosaic-emit-xaml/src/pipeline.rs`.

The `winui/` directory in this demo applies hand-patches for each
of the three so the WinUI 3 build succeeds. The patches are tiny
(under 20 lines total) and are documented inline as
`<!-- hand-patched for WinUI 3 attribute validity -->`. When the
fixes land in the emitter, this directory will regenerate cleanly
from the `mosaic/` sources via `mosaic-compile --backend xaml
--emit-project` with no patches required.

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
