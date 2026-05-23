---
title: Getting started with Acme
sidebar_label: Getting Started
sidebar_position: 2
---

Acme is a fictional library used only to give this demo
documentation site something to talk about.  It does not exist.
The point of this page is to exercise paragraph rendering,
ordered lists, inline code, and a code block — all driven by
the DOC00 pipeline.

## Install

```bash
npm install @acme/widget
```

That's it.  No build step.

## Hello, world

A minimal program:

```typescript
import { Widget } from "@acme/widget";

const w = new Widget({ label: "hello" });
console.log(w.render());
// → "Widget(hello)"
```

The widget exposes one method, `render()`, which returns a
string description.  See the [API reference](/api/reference) for
the full surface.

## Next steps

1. Read the [installation guide](/guide/installation) for the
   long-form version.
2. Skim the [configuration guide](/guide/configuration) for
   what's tweakable.
3. Try the search box in the header — it loads search shards
   on demand from `/search/`.
