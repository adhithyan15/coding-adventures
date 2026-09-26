### Security — an authored value is validated before it becomes Dart

Two injection sinks of the same class, both reachable from any stylesheet.

#### `opacity` passed authored text through verbatim

`dart_opacity_literal`'s fallback arm was `trimmed.to_string()`, interpolated
straight into `Opacity(opacity: {expr}, ..)`. So `opacity: "1 ), child:
evil(/*"` emitted

```dart
Opacity(
  opacity: 1 ), child: evil(/*,
  child: Container(
```

escaping the argument and commenting out what followed. A non-numeric opacity
now falls back to fully opaque — the same choice the colour path makes, and an
authored value the emitter cannot understand should not become code.

This one predates UI79 entirely and is untouched by the border work; it was
found by re-reviewing after the colour fix and asking whether the same shape
existed elsewhere in the file. It did.

#### A hex colour is validated by its digits, not its length

`css_color_to_dart` checked only that 6 or 8 characters followed the `#`, then
interpolated them straight into a Dart expression. The `.msl` grammar's
`HASH_COLOR` really is hex-only, but `style_value` also admits a quoted
`STRING`, which passes through verbatim and is unquoted before it reaches here
— and quoted colours are idiomatic in this repo.

So an authored `border-top-color: "#00)+E(/*"` emitted

```dart
border: Border(top: BorderSide(color: const Color(0x00)+E(/*), width: 1), ...
```

escaping the `Color(..)` argument and opening a comment that swallowed the next
widget property. That is code injection into generated Dart, reachable from any
stylesheet, and it predates UI79 — `background`, `background-color`, `color`
and `border-color` all funnel through the same function. UI79 added four more
entry points to it, which is how it was found.

Fixed at that one function so every sink is covered; a rejected colour lands on
each caller's existing fallback. Verified by re-emitting the hostile stylesheet
and seeing `Colors.transparent`.

Checked rather than assumed: **SwiftUI, Compose and Qt already validate the
digits** — the review that surfaced this claimed SwiftUI shared the flaw and it
does not. **html escapes** the value (`&quot;`) and **react** preserves the
backslash escaping, so both keep a hostile value inert inside its literal.
Flutter was the only backend affected.

A negative per-edge width is also skipped: `BorderSide` asserts `width >= 0` at
*runtime*, so it would type-check and then throw in the app.

Also hardened `parse_pixel_value`: `f64::parse` accepts `inf`, `NaN` and
overflowing literals like `1e400`, and `{f}` printed them as bare Dart
identifiers that do not compile. Not injection — no punctuation survives the
parse — but it broke the "generated source still type-checks" contract the `0`
fallback exists to keep.

