# web-audio-sink

`web-audio-sink` is the browser playback seam for the music stack.

It starts where the virtual audio pipeline ends:

```text
PCM samples -> Web Audio AudioBuffer -> browser speakers
```

The package does not parse notes or synthesize instruments. It accepts already
rendered mono signed 16-bit PCM samples, converts them to Web Audio's
floating-point format, and schedules them on an `AudioContext`.

## Usage

```typescript
import { playPcmBuffer } from "@coding-adventures/web-audio-sink";

await playPcmBuffer({
  samples: new Int16Array([0, 4096, 8192, 4096, 0, -4096]),
  format: {
    sampleRateHz: 44100,
    channelCount: 1,
    bitDepth: 16,
  },
});
```

Most browsers require playback to start from a user gesture such as a button
click. Pass an existing `AudioContext` if your app owns the gesture handling:

```typescript
button.addEventListener("click", async () => {
  const audioContext = new AudioContext();
  await playPcmBuffer(buffer, { audioContext, gain: 0.2 });
});
```

## API

| Export | Description |
|--------|-------------|
| `validatePcmBuffer` | Checks mono signed 16-bit PCM metadata and samples |
| `pcmSamplesToFloat32` | Converts integer PCM samples into Web Audio floats |
| `createAudioBufferFromPcm` | Copies PCM into an `AudioBuffer` |
| `playPcmBuffer` | Schedules a complete buffer and resolves on `ended` |

## Development

```bash
bash BUILD
```
