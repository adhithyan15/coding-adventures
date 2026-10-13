### Changed — no style value is XAML markup, not even a resource lookup (X-4)

- #16948 let a brace-led style value through when it was exactly
  `{ThemeResource Key}` or `{StaticResource Key}`.
- Decision (2026-10-08): mosstyle is platform-neutral (UI15 §8 rule 8). A
  XAML resource lookup is a platform value, so `STYLE_MARKUP_EXTENSIONS` is
  now empty. Every brace-led style value drops its property and is
  reported with the updated `REFUSED_STYLE_MARKUP_REASON`, as other
  refused markup already was. The matcher, the composite checks and the
  `escape_style_attr` check at each emission point are unchanged, so the
  rule holds everywhere.
- `mosstyle-compiler` now refuses such a value at compile time
  (`ErrorKind::PlatformValue`), so this is a second line of defence.
- No `.msl` or manifest token in the repo used a lookup.
- The seven tests that pinned lookups passing through now pin them being
  dropped.
