# @ca/code39

Dependency-free Code 39 encoder that emits backend-neutral draw instructions.

This package does not know about SVG directly. It produces generic scenes that
other renderer packages can consume.

## Usage

```typescript
import { drawCode39 } from "@ca/code39";
import { renderSvg } from "@ca/draw-instructions-svg";

const scene = drawCode39("HELLO-123");
const svg = renderSvg(scene);
```
