### Fixed — an emit-less component's sample shell compiles again (CS1061 on `ev.MosaicName`)

- The emit-project sample `MainWindow.xaml.cs` reported a handled event as
  `$"Status: Mosaic host handled {ev.MosaicName}"`. Only a component that
  declares events gets `MosaicName` on its `{Component}Event` union; an
  emit-less component's union is the bare `public abstract record FooEvent;`,
  so its sample shell failed the WinUI build with CS1061. The shell now reads
  `ev.GetType().Name` there when the component declares no events (such a
  component never dispatches, so the line only has to compile); a component
  with events is unchanged.
- `project_main_window_names_events_only_through_members_the_union_has` pins
  both forms against the event union the same project emits.
