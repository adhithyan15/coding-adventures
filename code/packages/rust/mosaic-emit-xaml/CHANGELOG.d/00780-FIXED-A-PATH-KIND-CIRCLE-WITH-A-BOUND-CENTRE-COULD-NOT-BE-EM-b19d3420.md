### Fixed — a `Path (kind: circle)` with a bound centre could not be emitted, and `main` was red (#14899 follow-up)

#14899 added a schematic terminal to `mosaic-pkg-spice-workbench`:

```
Path [ schematic-terminal ] ( kind: circle , cx: ( point[0] ) , cy: ( point[1] ) , r: 5 )
```

`cx`/`cy` are bound, and the XAML emitter refused any bound circle
coordinate, so `spice_workbench_compiles_to_xaml` failed on both runners.

Lines gained bindings in #14682; circles were deliberately left out because a
circle is positioned by `Margin="cx-r,cy-r,0,0"`. XAML cannot do arithmetic in
an attribute, and a `Thickness` is one composite string rather than four
bindable doubles, so there was nowhere for `cx - r` to go.

**Split the sum instead of computing it.** `TranslateTransform` exposes `X`
and `Y` as plain bindable doubles, so the bound centre goes there, and the
`- r` half becomes a literal negative margin:

```xml
<Ellipse Width="10" Height="10" Margin="-5,-5,0,0" HorizontalAlignment="Left" VerticalAlignment="Top">
  <Ellipse.RenderTransform>
    <TranslateTransform X="{x:Bind Expr_..Number, Mode=OneWay}" Y="{x:Bind ..}"/>
  </Ellipse.RenderTransform>
</Ellipse>
```

The two compose to `(cx - r, cy - r)` with no arithmetic performed on a bound
value. `r` stays literal — it drives `Width`/`Height` and that margin, none of
which a markup binding can compute.

**A literal circle is byte-identical to before**, pinned by a test that fails
if a `TranslateTransform` ever appears on one.

`path_slot_bound_coordinate_is_a_clear_error_not_a_silent_drop` was retargeted
rather than deleted. The rule it protects — a bound coordinate must never be
silently dropped — is unchanged; it was enforced by refusing only because
support did not exist. It now covers a bound `r`, which genuinely cannot bind,
and a new test covers the centre binding.

**Not verified at runtime.** WinUI 3 does not build on macOS, so unlike the
`Margin` positioning in #12028 this shape was not confirmed with a real
`dotnet build`. `TranslateTransform.X`/`Y` are standard bindable
`DependencyProperty` doubles, but the render is unconfirmed on this host.

