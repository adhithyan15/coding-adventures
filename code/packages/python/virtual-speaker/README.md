# virtual-speaker

`virtual-speaker` owns the voltage-to-pressure-proxy stage in `MUS01`.

The first speaker model is deliberately tiny:

```text
speaker_pressure_proxy(t) = speaker_gain * input_signal(t)
```

This is not a physics model. It is an educational boundary where a virtual DAC
voltage can become a sound-pressure-like signal without requiring calculus,
speaker cones, room acoustics, or hardware.

## Example

```python
from dataclasses import dataclass

from virtual_speaker import LinearSpeakerSignal

@dataclass(frozen=True)
class ConstantSignal:
    value: float

    def value_at(self, time_seconds: float) -> float:
        return self.value

speaker = LinearSpeakerSignal(ConstantSignal(0.5), speaker_gain=2.0)
print(speaker.value_at(0.0))  # 1.0
```

## Development

```bash
bash BUILD
```
