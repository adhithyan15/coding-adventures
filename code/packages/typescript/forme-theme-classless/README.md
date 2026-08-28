# @coding-adventures/forme-theme-classless

A reusable classless `StyleDocument` for Forme web output. It provides a
readable system-font prose layout, token-backed light colors, and
preference-driven dark, narrow-viewport, and high-contrast rules.

```ts
import classlessTheme from "@coding-adventures/forme-theme-classless";

const rendererConfig = {
  style: classlessTheme,
  activeStyleContexts: ["dark", "narrow", "high-contrast"],
};
```

The value is already resolved (`theme: null`). Callers may replace it or use
`forme-style-theme` to compose an overlay before passing it to a renderer.
