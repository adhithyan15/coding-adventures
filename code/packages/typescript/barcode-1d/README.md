# @coding-adventures/barcode-1d

End-to-end 1D barcode pipeline for TypeScript.

This package is the high-level entry point:

1. Choose a symbology package such as Code 39 or UPC-A
2. Produce barcode runs and lay them out with `@coding-adventures/barcode-layout-1d`
3. Convert the resulting `PaintScene` to PNG bytes through a native Paint VM

## Usage

```typescript
import { renderBarcode1DToPng } from "@coding-adventures/barcode-1d";

const png = renderBarcode1DToPng({
  symbology: "code39",
  data: "ADHITHYA",
});
```

## Native backend

- macOS: `paint-metal`
- Windows: `paint-vm-direct2d` with `paint-vm-gdi` available as the fallback path inside the native layer

Linux support is not implemented yet.
