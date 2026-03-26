# @coding-adventures/code128

Dependency-free Code 128 encoder that currently implements Code Set B and emits
backend-neutral draw instructions.

## V1 Scope

This first version focuses on Code Set B because it already demonstrates the
important structure of Code 128:

- printable ASCII input
- a start code
- per-symbol numeric values
- modulo-103 checksum calculation
- a dedicated stop pattern

Future versions can add Code Sets A and C on top of the same internal shape.

## Pipeline

```text
printable ASCII
  -> normalize for Code Set B
  -> map characters to Code 128 values
  -> prepend Start B
  -> append modulo-103 checksum
  -> append stop
  -> expand to shared 1D runs
  -> translate to draw instructions
  -> hand the scene to any renderer backend
```

## Usage

```typescript
import { drawCode128 } from "@coding-adventures/code128";
import { renderSvg } from "@coding-adventures/draw-instructions-svg";

const scene = drawCode128("Code 128");
const svg = renderSvg(scene);
```

Intermediate helpers include:

- `normalizeCode128B()`
- `computeCode128Checksum()`
- `encodeCode128B()`
- `expandCode128Runs()`

## Why This Package Is Useful

Code 128 is the densest 1D format in the current barcode track. It is a good
bridge between the simpler wide/narrow symbologies and future formats that feel
more like compact data languages than simple retail labels.

## Development

```bash
bash BUILD
```
