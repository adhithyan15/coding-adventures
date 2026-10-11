### Fixed — a stylesheet may name a theme resource, not run markup (#15487 follow-up)

#15487 split author literal text (`escape_xaml_attr`, which adds XAML's `{}`
escape) from emitter-built markup (`escape_xml_attr`, XML layer only). One gap
remained: `translate_xaml_value` passed **any** `.msl` value that started with
`{` through as a XAML markup extension, and the base `part` attributes and
visual-state `<Setter Value=…>` then escaped it for XML only. A third-party
package's stylesheet -- compiled into the consumer's XAML once packages resolve
as dependencies -- could therefore emit `{Binding …}` (read the consumer's view
model), `{x:Bind Method()}` (call a method on the consumer's page),
`{x:Null}`, or any `{StaticResource …}` with converter arguments.

- Brace-led style values now pass an allow-list: the whole trimmed value must be
  exactly `{ThemeResource Key}` or `{StaticResource Key}`, one space between,
  with `Key` an identifier (ASCII letter or `_`, then letters, digits, `_`,
  `.`). Those are pure resource lookups, the one thing a stylesheet needs `{…}`
  for (following the system theme).
- Every other brace-led value -- `{Binding X}`, `{x:Bind M()}`, `{x:Null}`,
  `{StaticResource a b}`, `{ThemeResource Foo}extra`,
  `{ThemeResource {x:Bind}}`, `{}…`, whitespace-led variants -- **drops the
  property**: no attribute, no `<Setter>`. A base-`part` drop is reported by
  `dropped_style_properties` (so it reaches `mosaic-degradations.json`) with a
  reason naming the allow-list. Dropping rather than `{}`-escaping is
  deliberate: a style value goes to a `Brush`/`Double` parser, so escaped text
  would throw at page load and let a package stop its consumer's window from
  opening.
- Composites drop whole: a brace-led `top`/`left` under `position: absolute`
  drops `Margin` (the top-left pinning alignments stay), and a brace-led
  per-edge or shorthand border width drops `BorderThickness` -- even for an
  allow-listed lookup, which cannot be one number of four. A refused border
  colour is reported instead of silently skipped.
- A new `escape_style_attr` re-checks the policy at the two emission choke
  points (base attribute fragment, visual-state setter) and drops/reports or
  omits the setter, so a future producer that forgets the allow-list still
  cannot emit markup.
- No output changes for any real stylesheet: none of the repo's 186 `.msl`
  files has a brace in any of its 3,162 quoted values or 8,890 declarations.
  Regenerating every `.msl` that compiles standalone with `mosaic-compile
  --backend xaml` (159 components, 786 generated files) before and after this
  change gives byte-identical output.
- Tests: the allow-list accepts `{ThemeResource …}`/`{StaticResource …}` across
  setter kinds; 25 refused values (bindings, `x:Bind`, `x:Null`, extra
  arguments, trailing text, nested extensions, bad keys, wrong case,
  unbalanced braces, `{}`, leading whitespace) are refused by both
  `translate_xaml_value` and `escape_style_attr`; non-brace values are
  unchanged; end to end, each refused value is absent from the base attribute
  and present in the drop report, the visual-state `<Setter>` is omitted, and
  absolute-position `Margin` and edge `BorderThickness`/brush composites drop
  and report. The X5 tests that asserted `{x:Bind …}` / `{Binding …}` style
  values pass through now assert they are dropped.
