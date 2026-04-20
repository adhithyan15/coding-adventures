# note-audio

`note-audio` is the teaching/orchestration package for
`MUS01: Note-to-Sound Signal Chain`.

It deliberately does not jump from `"A4"` straight to a speaker. Instead, it
keeps the whole chain visible:

```text
note -> frequency -> oscillator -> sampler -> PCM -> DAC -> speaker model
```

The package is virtual and deterministic. It does not open a real audio device,
sleep in real time, or talk to hardware. That makes it useful for learning,
debugging, and tests.

The reusable stages now live in smaller packages:

| Stage | Package |
|-------|---------|
| note parsing and frequency | `note-frequency` |
| oscillator and sampler | `oscillator` |
| PCM encoding | `pcm-audio` |
| virtual DAC | `virtual-dac` |
| virtual speaker | `virtual-speaker` |
| WAV file/container sink | `wav-sink` |

`note-audio` wires those packages together and returns an inspectable
`RenderedNote`.

## Example

```python
from note_audio import render_note_to_sound_chain, to_wav_bytes

chain = render_note_to_sound_chain("A4", duration_seconds=1.0)

print(chain.frequency_hz)              # 440.0
print(chain.floating_samples.samples[:5])
print(chain.pcm_buffer.samples[:5])
print(chain.dac_signal.value_at(0.001))
print(chain.speaker_signal.value_at(0.001))

wav_bytes = to_wav_bytes(chain.pcm_buffer)
```

## Layers

`NoteEvent` stores the human-level note, duration, amplitude, and start time.

`render_note_to_sound_chain()` parses the note with `note-frequency`, creates a
sine oscillator with `oscillator`, samples it, delegates signed 16-bit PCM
encoding to `pcm-audio`, builds a zero-order-hold virtual DAC with
`virtual-dac`, and wraps that in a linear virtual speaker model from
`virtual-speaker`.

`PCMBuffer` stores machine-friendly signed 16-bit samples and timing metadata.

`ZeroOrderHoldDACSignal` turns PCM integers back into voltage-like values over
time.

`LinearSpeakerSignal` is a deliberately simple speaker placeholder:

```text
speaker_pressure_proxy(t) = speaker_gain * dac_voltage(t)
```

## WAV

WAV support is an optional file/container sink:

```python
from note_audio import render_note_to_sound_chain, write_wav

chain = render_note_to_sound_chain("C4", duration_seconds=0.5)
write_wav("c4.wav", chain.pcm_buffer)
```

The WAV writer is re-exported from `wav-sink`. It supports mono signed 16-bit
PCM so the digital stage can be handed to normal audio software, but it does
not replace the DAC and speaker abstractions.

## Development

```bash
bash BUILD
```
