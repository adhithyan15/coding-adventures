---
title: FAQ
sidebar_position: 3
---

## Why is install failing?

The most common cause is a Node.js version below 20.  Run
`node --version`; if it prints `v18.x.x` or lower, upgrade.

## Does this work in the browser?

Yes.  The package ships an ES module that bundlers (Vite,
Rollup, esbuild, webpack) accept directly.  No polyfills
are needed for any modern browser.

## How do I report a bug?

Open an issue on the project's GitHub repository.  Please
include the version (`Widget.VERSION`), the runtime
(`node --version` or browser + version), and a minimal
reproduction.

## Is it tree-shakeable?

The package is published as ES modules and side-effect-free
(`"sideEffects": false` in `package.json`).  Both Vite and
Rollup are confirmed to drop unused exports.

## How big is it?

About 8KB minified, 3KB gzipped.  The biggest single chunk
is the CSS variables; if you don't use theming, your bundler
should drop them as dead code.

## Where's the source?

See the [API reference](/api/reference) for the surface and
the [getting started](/getting-started) page for an example.
The full source lives on GitHub.
