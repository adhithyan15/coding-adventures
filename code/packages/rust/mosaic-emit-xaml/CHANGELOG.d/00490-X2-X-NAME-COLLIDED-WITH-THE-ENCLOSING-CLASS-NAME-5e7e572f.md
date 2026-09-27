### X2 — `x:Name` collided with the enclosing class name

Components where the pascal-cased part name equals the component
name (e.g. `Button.mll`'s `HostButton [ button ]` inside the
`Button` component) produced `<Button x:Name="Button">`. WinUI's
XAML compiler auto-generates a `private Button Button;` field
that triggers C# error CS0542 ("member names cannot be the same
as their enclosing type"). Affected Button, Checkbox, Input,
Radio.

Fixed by detecting the collision in `host_x_name` and suffixing
`Element` to the identifier. Event-handler stems are derived from
`x_name` so both the XAML attribute (`Click="ButtonElement_Click"`)
and the code-behind method (`private void ButtonElement_Click`)
stay consistent automatically.

Regression tests: `x_name_avoids_component_class_name_collision`,
`x_name_unchanged_when_no_collision`.

