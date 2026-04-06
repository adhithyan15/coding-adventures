# @coding-adventures/image-codec-qoi

IC03: QOI (Quite OK Image) encoder and decoder.

## Usage

```typescript
import { encodeQoi, decodeQoi } from "@coding-adventures/image-codec-qoi";
import { createPixelContainer, fillPixels } from "@coding-adventures/pixel-container";

const c = createPixelContainer(100, 100);
fillPixels(c, 128, 0, 200, 255);
const qoi = encodeQoi(c);    // compact — solid images compress heavily via OP_RUN

const back = decodeQoi(qoi);
```
