---
category: Mosaic compiler pipeline
---

# Web component `when`/`each` blocks must emit JavaScript ternaries / `.map()`, not `<template>` HTML tags

The `<template data-when>` approach requires a client-side runtime to interpret. Since Custom Elements use a self-contained `_render()` that writes to `shadowRoot.innerHTML`, the when/each control flow must be JS expressions: `${this._show ? \`...\` : ''}` and `${this._items.map(item => \`...\`).join('')}`.
