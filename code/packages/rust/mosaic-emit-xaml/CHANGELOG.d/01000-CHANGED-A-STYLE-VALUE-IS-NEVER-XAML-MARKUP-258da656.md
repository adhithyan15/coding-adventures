### Changed — a style value is never XAML markup (X-4)

- `translate_xaml_value` and the colour translator used to pass any value
  starting with `{` straight through, as a "markup extension". mosstyle is
  platform-neutral: such a value is now refused upstream (`PlatformValue`).
  The emitter, which does not rely on that check alone, drops the property
  and reports it.
- Style fragments and visual-state setter values now use the literal escape
  `escape_xaml_attr`, since a style value can no longer be markup. Only the
  `StateTrigger` binding the emitter builds keeps the markup escape.
- The X5 test that pinned a `{x:Bind …}` style value passing through now
  pins it being dropped. No real style used it.
