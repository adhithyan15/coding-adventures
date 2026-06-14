---
title: API Reference
sidebar_position: 1
---

The complete public surface of `@acme/widget`.

## `Widget`

### Constructor

```typescript
new Widget(options: WidgetOptions): Widget
```

Throws `TypeError` on invalid options.  See the
[configuration guide](/guide/configuration) for option
semantics.

### `widget.render(): string`

Returns the string representation:

```typescript
const w = new Widget({ label: "hi" });
w.render();  // → "Widget(hi)"
```

Pure function.  Idempotent.  Safe to call many times.

### `widget.label: string`

Read-only accessor for the constructor's `label` option.

### `widget.size: "small" | "medium" | "large"`

Read-only accessor for the size.

## Types

```typescript
export interface WidgetOptions {
  label: string;
  size?: "small" | "medium" | "large";
  rounded?: boolean;
  className?: string;
}
```

## Constants

### `VERSION`

A string of the form `"x.y.z"` matching the package version.

## Errors

The constructor throws `TypeError` with descriptive messages
for every invalid input.  No silent coercion; no `undefined`
defaults that hide bugs.
