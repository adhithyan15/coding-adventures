# @coding-adventures/image-codec-ppm

IC02: PPM P6 image encoder and decoder.

## Usage

```typescript
import { encodePpm, decodePpm } from "@coding-adventures/image-codec-ppm";
import { createPixelContainer, fillPixels } from "@coding-adventures/pixel-container";

const c = createPixelContainer(4, 4);
fillPixels(c, 200, 100, 50, 255);
const ppm = encodePpm(c);     // P6\n4 4\n255\n + raw RGB bytes

const back = decodePpm(ppm);  // A always 255 on decode
```
