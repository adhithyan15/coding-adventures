# @coding-adventures/css-lexer

A CSS lexer that follows the shared `css.tokens` priority order.

```ts
import { tokenizeCss } from "@coding-adventures/css-lexer";

const tokens = tokenizeCss("h1 { color: #333; }");
```
