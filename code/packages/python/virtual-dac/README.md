# virtual-dac

`virtual-dac` owns the PCM-to-voltage stage in `MUS01`.

It does not talk to real hardware. It answers one educational question:

```text
If this PCM sample reached an ideal DAC, what voltage would be held now?
```

## Example

```python
from pcm_audio import PCMBuffer, PCMFormat
from virtual_dac import ZeroOrderHoldDACSignal

pcm = PCMBuffer((0, 32767, 0, -32768), PCMFormat(sample_rate_hz=4.0))
dac = ZeroOrderHoldDACSignal(pcm)

print(dac.value_at(0.25))  # 1.0
```

## Development

```bash
bash BUILD
```
