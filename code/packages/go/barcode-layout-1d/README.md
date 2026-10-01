# barcode-layout-1d

Pure 1D barcode layout that converts barcode runs into `PaintScene`.

## Portable v1 API

The additive `V1` entry points implement the bounded, stable-error
`barcode-layout-1d-v1` contract while the original functions remain available
for downstream compatibility.

```go
runs, err := barcodelayout1d.ExpandBinaryV1("101", barcodelayout1d.Barcode1DV1BinaryOptions{
    SourceLabel: "guard",
    SourceIndex: 0,
    Role:        "guard",
})
if err != nil {
    return err
}

scene, err := barcodelayout1d.ProjectSceneV1(runs, nil)
```

Portable v1 projection is integer-only and rectangle-only. Human-readable text
requests return `human-readable-text-unsupported` before any native resolver or
backend can run.

## Development

```bash
bash BUILD
```
