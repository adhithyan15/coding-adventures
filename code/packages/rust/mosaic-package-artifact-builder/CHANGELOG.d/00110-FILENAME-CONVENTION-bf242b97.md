### Filename convention

- **Default variant** (bare `<Component>.mll` exists): output is the
  unsuffixed `<Component>.<ext>` — same name as pre-UI30 builds.
- **Named variants** (`<Component>.touch.mll` etc.): output is
  `<Component>.<variant>.<ext>`. The variant infix lands between the
  component name and the file extension so multiple variants coexist
  in one output directory without collision.

For XAML this means a single component can emit:
```
Grid.xaml          Grid.touch.xaml          (default + variant XAML)
Grid.xaml.cs       Grid.touch.xaml.cs       (matching code-behinds)
Grid.Event.cs      Grid.touch.Event.cs      (matching event unions)
```

