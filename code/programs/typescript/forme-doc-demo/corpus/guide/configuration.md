---
title: Configuration
sidebar_position: 2
---

Every knob Acme exposes is passed to the constructor — there
are no environment variables, no config files, no global
state to fight with.

## Constructor options

```typescript
new Widget({
  label: "hello",     // required
  size: "medium",     // optional: "small" | "medium" | "large"
  rounded: false,     // optional: defaults to true
  className: "x",     // optional: extra CSS class for the wrapper
});
```

### `label`

The user-visible string.  Required, non-empty.  Validated at
construction; an empty or non-string label throws a
`TypeError`.

### `size`

One of three allowlisted strings.  Maps to a CSS sizing
class.  Unknown values throw at construction.

### `rounded`

Whether to apply the rounded-corners style.  Boolean only —
truthy/falsy coercion is intentionally rejected.

### `className`

Extra class name forwarded to the root element.  HTML-escaped
internally; safe to pass user input.

## Theming

CSS variables on the `.acme-widget` root let you re-skin
without forking:

```css
.acme-widget {
  --acme-bg: #fafafa;
  --acme-fg: #111;
  --acme-radius: 6px;
}
```

See the [API reference](/api/reference) for the complete
attribute list.
