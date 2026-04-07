# @coding-adventures/image-codec-bmp

IC01: BMP image encoder and decoder.

Encodes `PixelContainer` to 32-bit BGRA BMP and decodes BMP back to `PixelContainer`.

## Usage

```typescript
import { encodeBmp, decodeBmp, BmpCodec } from "@coding-adventures/image-codec-bmp";
import { createPixelContainer, setPixel } from "@coding-adventures/pixel-container";

const c = createPixelContainer(2, 1);
setPixel(c, 0, 0, 255, 0, 0, 255); // red
setPixel(c, 1, 0, 0, 0, 255, 255); // blue
const bmp = encodeBmp(c);

const back = decodeBmp(bmp);
// back.width === 2, back.height === 1
```
