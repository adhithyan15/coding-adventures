# wav-sink

`wav-sink` owns the optional file/container sink in `MUS01`.

WAV is not the sound itself. It is a simple container around PCM samples and
metadata, useful because normal audio tools can open it:

```text
PCMBuffer -> RIFF/WAVE bytes -> optional file path
```

## Example

```python
from pcm_audio import PCMBuffer, PCMFormat
from wav_sink import to_wav_bytes

pcm = PCMBuffer((0, 32767, -32768), PCMFormat(sample_rate_hz=8.0))
wav_bytes = to_wav_bytes(pcm)

print(wav_bytes[:4])  # b"RIFF"
```

## Development

```bash
bash BUILD
```
