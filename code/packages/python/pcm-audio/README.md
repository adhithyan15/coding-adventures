# pcm-audio

`pcm-audio` owns the digital audio stage in `MUS01`.

It converts normalized floating-point samples, such as values produced by an
oscillator sampler, into signed 16-bit PCM integers:

```text
floating samples -> PCMFormat -> PCMBuffer
```

The package is intentionally small and deterministic. It does not write files,
talk to speakers, or know where samples came from.

## Example

```python
from oscillator import SampleBuffer
from pcm_audio import PCMFormat, encode_sample_buffer

floating = SampleBuffer(samples=(0.0, 1.0, 0.0, -1.0), sample_rate_hz=4.0)
pcm = encode_sample_buffer(floating, PCMFormat(sample_rate_hz=4.0))

print(pcm.samples)  # (0, 32767, 0, -32768)
```

## Development

```bash
bash BUILD
```
