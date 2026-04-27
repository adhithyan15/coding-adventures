# typescript/pcm-audio

`pcm-audio` is the reusable digital encoding stage in `MUS01`.

It converts normalized floating-point samples into signed 16-bit PCM integers for the
next audio layer:

```text
floating samples -> PCMFormat -> PCMBuffer
```

The package is intentionally small and deterministic. It does not write files or
talk to audio devices.

## Usage

```ts
import { floatToPcm16, samplesToPcmBuffer } from "@coding-adventures/pcm-audio";

const pcm = samplesToPcmBuffer([0.0, 1.0, -1.0, 2.0], { sampleRateHz: 4.0 });
console.log(pcm.samples); // [0, 32767, -32768, 32767]
```

## Development

```bash
bash BUILD
```
