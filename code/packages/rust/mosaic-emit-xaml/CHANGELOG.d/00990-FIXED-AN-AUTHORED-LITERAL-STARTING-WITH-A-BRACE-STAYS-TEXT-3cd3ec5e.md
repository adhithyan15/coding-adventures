### Fixed — an authored literal starting with `{` stays text (#15487)

- XAML reads an attribute value that starts with `{` as a markup extension.
  `escape_xaml_attr` escaped only `& " < >`, so an authored label
  `{x:Null}` was emitted as `Content="{x:Null}"`, which XAML reads as the
  `x:Null` extension rather than the text. The same applied to every
  attribute fed from authored text: `Content`, `Text`, `PlaceholderText`,
  `Title`, `AutomationProperties.Name`, `PaneTitle` and the others.
- `escape_xaml_attr` now prefixes XAML's own escape, `{}`, whenever the
  escaped value starts with `{`, so `{x:Null}` becomes
  `Content="{}{x:Null}"`. A brace anywhere else is already text in XAML
  and is left alone.
- Values that are markup on purpose use the new
  `escape_xaml_markup_attr`, which leaves a leading `{` as it is:
  - the `{x:Bind …}` a visual state's `StateTrigger` binds to;
  - the template trigger proxy's `Tag`;
  - the style fragment and setter values, which `translate_xaml_value`
    passes through as extensions.
- Style values that start with `{` still pass through verbatim. Whether a
  package dependency's styles may carry markup extensions is a separate
  decision, recorded in the Mosaic backlog.
