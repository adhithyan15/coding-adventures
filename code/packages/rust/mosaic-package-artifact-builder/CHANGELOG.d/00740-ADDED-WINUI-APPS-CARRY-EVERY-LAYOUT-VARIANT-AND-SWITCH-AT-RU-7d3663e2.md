### Added — WinUI apps carry every layout variant and switch at run time (UI48 ENV2/ENV3, §7.11)

- XAML variants compile through `mosaic_emit_xaml::pipeline::from_pipeline_variant`,
  so `<C>.<variant>.xaml.cs` declares `partial class <C><Variant>` and raises
  the default's `<C>Event`. No `<C>.<variant>.Event.cs` is written any more;
  the builder's own `xaml_variant_type_suffix` is gone (the emitter names the
  type, refusing variants such as `a--b` that every other backend refuses).
- Every XAML compile passes the package's exports
  (`EmitOptions::package_exports`), so a variant type another export owns is
  refused; `xaml_check_variant_types` refuses two exports' variants that spell
  one type (`Card` + `touch-bar` and `CardTouch` + `bar`), or one inside
  another's `…Mosaic…` support names (`Card.touch` and
  `Card.touch-mosaic-slider`), naming both files, and refuses a component
  with variants but no default `<C>.mll` (its variants would raise a
  `<C>Event` nothing declares).
- `xaml_layout_choices` uses the same `effective_layout_rules` as SwiftUI,
  Compose and Flutter, refuses a rule for a missing variant or one that cannot
  name a C# type, keys conditions by `EnvironmentAxis::wire_name`, refuses a
  selectable variant rooted in a `HostDialog` under a control-rooted default,
  and gives a dialog-rooted window no choices. The emitter writes them into
  `MainWindow.xaml.cs`; no control is re-emitted per profile, because the
  strict policy lives in the window.
- `MosaicPackage.props` lists each variant's `Page` and code-behind; it
  already listed their row view models, which named a control it never
  compiled.
- Tests mirror the other backends' (convention, declared rules under wire
  names, missing variant refused, no variants → no selector) plus the strict
  mount, every rule axis being a key of the XAML binding's
  `EnvironmentReport`, every public type of the XAML templates being
  reserved, the cross-export refusals and dialog roots.
