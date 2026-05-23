---
title: Installation
sidebar_position: 1
---

Acme runs on any modern JavaScript runtime.  Pick whichever
you already have.

## Node.js

```bash
npm install @acme/widget
# or
yarn add @acme/widget
# or
pnpm add @acme/widget
```

Node.js 20 or later is required.

## Deno

```typescript
import { Widget } from "npm:@acme/widget";
```

Deno's npm-compat layer handles the rest.

## Bun

```bash
bun add @acme/widget
```

## Verifying the install

After install, run a one-liner to verify:

```javascript
console.log(require("@acme/widget").VERSION);
```

You should see the current version number printed.  If you
see an error instead, check the [FAQ](/faq) for common
installation problems.

## What's installed

The package installs a single `.js` bundle (around 8KB
minified) plus TypeScript type declarations.  No native
dependencies, no install scripts, no postinstall hooks.
