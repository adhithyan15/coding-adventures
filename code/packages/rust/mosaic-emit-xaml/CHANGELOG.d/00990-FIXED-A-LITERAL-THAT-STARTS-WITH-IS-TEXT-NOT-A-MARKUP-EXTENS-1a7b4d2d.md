### Fixed — a literal that starts with `{` is text, not a markup extension (#15487)

`escape_xaml_attr` escaped only the XML layer (`&`, `"`, `<`, `>`). XAML reads
any attribute value whose first character is `{` as a markup extension, so an
authored literal such as `HostButton [ ok ] ( label : "{x:Null}" )` emitted
`Content="{x:Null}"` -- a null button rather than the text the author wrote --
and a literal like `"{oops"` was a XAML parse error. Once `.mll` files arrive as
third-party package dependencies, the same gap lets a string become a binding or
a resource lookup.

- `escape_xaml_attr` (authored literal text) now prepends XAML's `{}` escape
  when the escaped value starts with `{`: `{x:Null}` → `{}{x:Null}`. A value
  the author began with `{}` gets its own prefix (`{}{}…`), because the input is
  never pre-escaped XAML. Values that do not start with `{` are unchanged, so
  no existing app's output changes.
- The XML-only escaping moves to a new `escape_xml_attr`, used where the value
  is markup the emitter built and must stay markup: `StateTrigger IsActive`
  bindings (and their template proxy `Tag`), visual-state `Setter Value`s and
  style-fragment values (mosstyle `{…}` values are passed through as markup
  extensions by design), and `xmlns:` declarations. Applying the prefix inside
  the single shared helper, as first proposed, broke thirteen existing
  visual-state unit tests by turning every trigger binding into text.
- The native-table row-header `Header=` mixed both kinds through one escape;
  each arm now escapes for its own kind, so a literal header is protected and a
  bound header stays a binding.
- Tests: the helper's table (leading `{`, lone `{`, `{}`-prefixed, unchanged
  values), one case per literal attribute path (`Text`, `Content`,
  `AutomationProperties.Name`/`AutomationId`, `PlaceholderText`, `Source`,
  `Glyph`, `GroupName`, `ToolTipService.ToolTip`, `PaneTitle`, `Title`,
  `DragLabel`, `DropKey`, row-header `Header`, native-table name, and
  component-reference props), and the markup paths that must not change.
  `HostLink`'s `href` is refused by the URI-scheme allow-list before it is
  escaped; a test pins that.
