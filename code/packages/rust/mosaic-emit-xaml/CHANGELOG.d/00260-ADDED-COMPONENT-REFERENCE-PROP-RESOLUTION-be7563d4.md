### Added — Component-reference prop resolution

The emitter walks the component-reference's `props` and produces
XAML attribute fragments:

- `slot ref` → `Attribute="{x:Bind Path}"` (PascalCased)
- `string literal` → `Attribute="literal"` (XAML-escaped)
- `number` → `Attribute="N"`
- `keyword (for-bound name)` → `Attribute="{x:Bind Name}"` (treated
  as a bound name when in scope)
- `keyword (other)` → `Attribute="literal"` (passes through)
- `expr` → routed through the PR-2 ExprLowerer (bindable path or
  helper call)
- `emit ref` → DEFERRED — surfaced as a XAML comment listing the
  skipped props so the gap is visible in diffs. Host-side handler-stub
  generation is PR-5+ work and lands in a follow-up.

