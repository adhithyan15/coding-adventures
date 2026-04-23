# @coding-adventures/diagram-to-paint

Compiles layouted diagrams into `PaintScene`.

This package is the seam between graph/sequence/waveform layout packages and
the generic Paint VM stack. The first implementation slice focuses on layouted
graph diagrams and lowers them into:

- node shapes
- edge paths
- arrowheads
- text labels

## Running tests

```bash
npx vitest run --coverage
```
