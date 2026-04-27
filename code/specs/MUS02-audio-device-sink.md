# MUS02: Audio Device Sink

## 1. Overview

This spec defines the first real playback sink in the note-to-sound stack.

`MUS01` deliberately kept every stage virtual:

```text
note -> frequency -> oscillator -> sampler -> PCM -> virtual DAC -> virtual speaker
```

That is perfect for learning because every intermediate value is inspectable.
This spec adds the next box:

```text
PCM buffer -> audio device sink -> operating-system audio API -> speakers
```

The goal is still educational. We are not hiding the lower layers. We are
creating a clean public abstraction for "play these already-rendered samples"
while keeping the OS-specific backend visible for readers who want to peek
behind the curtain.

## 2. Beginner Mental Model

An audio device sink is like the final delivery truck in the audio pipeline.

Earlier packages decide what should be played:

- the note package turns `A4` into `440.0 Hz`
- the oscillator package defines a smooth sine wave
- the sampler snapshots that wave
- the PCM package turns snapshots into signed integers

The audio sink does not know what a note is. It only receives a buffer of PCM
numbers and asks the operating system to deliver those numbers to the default
speaker device.

In other words:

```text
Python: "Here are 44,100 signed 16-bit samples."
Rust:   "These samples are valid and safe to hand to an OS backend."
macOS:  "Core Audio will schedule them for the selected output device."
```

## 3. Package Boundaries

The first implementation is split into three reusable packages.

| Package | Language | Responsibility |
|---------|----------|----------------|
| `audio-device-sink` | Rust | Backend-neutral sink contract, PCM validation, shared error types |
| `audio-device-coreaudio` | Rust | macOS Core Audio implementation of the sink contract |
| `audio-device-sink` | Python | Python bridge that adapts Python PCM buffers into the Rust sink |

The Python and Rust packages may share a distribution name family, but their
source trees remain separate:

```text
code/packages/rust/audio-device-sink/
code/packages/rust/audio-device-coreaudio/
code/packages/python/audio-device-sink/
```

The Rust core crate must not depend on Python or Core Audio. It should be useful
for future Linux, Windows, browser, and microcontroller sinks.

The Core Audio crate must depend on the Rust core crate and may depend on
macOS-specific APIs.

The Python package must depend on `python-bridge` for the extension boundary and
must expose a normal Python module for callers.

## 4. Relationship to `MUS01`

`MUS01` owns the educational signal chain up to PCM.

`MUS02` starts at PCM.

This means the audio device sink must not parse notes, build oscillators, sample
continuous functions, or encode floats into PCM. Those are already separate
layers.

Required input:

```text
PcmPlaybackBuffer(
    samples = signed 16-bit PCM integers,
    sample_rate_hz = integer sample rate,
    channel_count = channel count,
)
```

For V1, the sink should align with the existing Python PCM stage:

- signed 16-bit PCM
- mono
- integer sample rate, defaulting to `44_100 Hz`
- blocking playback only
- default output device only

## 5. Rust Core Contract

The Rust `audio-device-sink` crate defines the stable contract.

### 5.1 `PcmFormat`

`PcmFormat` describes how the integer samples should be interpreted:

```text
PcmFormat {
    sample_rate_hz: u32,
    channel_count: u16,
    bit_depth: u16,
}
```

V1 required validation:

- `sample_rate_hz` must be greater than zero
- `channel_count` must be exactly `1`
- `bit_depth` must be exactly `16`

The mono-only rule is intentional. It keeps the first device sink aligned with
the existing virtual audio pipeline before we introduce interleaved stereo
buffers.

### 5.2 `PcmPlaybackBuffer`

`PcmPlaybackBuffer` owns the samples and their format:

```text
PcmPlaybackBuffer {
    samples: Vec<i16>,
    format: PcmFormat,
}
```

The buffer should expose:

- `sample_count()`
- `duration_seconds()`
- `is_empty()`

An empty buffer is valid to construct, but playing it is a no-op that returns a
successful report with zero frames played.

### 5.3 `AudioSink`

An `AudioSink` is any object that can play a PCM buffer:

```text
trait AudioSink {
    fn play_blocking(&self, buffer: &PcmPlaybackBuffer) -> Result<PlaybackReport, AudioSinkError>;
}
```

`play_blocking` means the call does not return until the sink has either:

- handed the whole buffer to a backend that completed playback
- decided there is nothing to play
- failed with a typed error

V1 does not define a streaming callback API. Streaming requires a more careful
clocking story and should arrive after this blocking sink is easy to understand.

### 5.4 `PlaybackReport`

`PlaybackReport` records what the sink accepted:

```text
PlaybackReport {
    frames_played: usize,
    sample_rate_hz: u32,
    channel_count: u16,
    duration_seconds: f64,
    backend_name: &'static str,
}
```

For mono V1, `frames_played == samples.len()`.

### 5.5 `AudioSinkError`

Errors should be explicit and beginner-readable:

| Error | Meaning |
|-------|---------|
| `InvalidFormat` | The PCM metadata is not supported |
| `InvalidSamples` | The sample buffer is too large or malformed |
| `UnsupportedPlatform` | No backend exists for this OS yet |
| `BackendUnavailable` | The OS audio service or default output device could not be opened |
| `BackendFailure` | The OS accepted the request but playback failed |

The error type should implement `std::error::Error`, `Display`, `Debug`, and
`PartialEq` where possible.

## 6. Core Audio Backend

The Rust `audio-device-coreaudio` crate is the first concrete backend.

It must implement the `AudioSink` trait using macOS Core Audio.

Required V1 behavior:

- compile on macOS
- expose a backend named `coreaudio`
- use the default output device
- accept mono signed 16-bit PCM
- play blocking until the buffer is finished
- return `UnsupportedPlatform` on non-macOS builds

The backend may convert samples internally if Core Audio asks for a different
device format. That conversion belongs inside the backend because it is an
operating-system concern, not a music concern.

The public API should make this layering visible:

```text
CoreAudioSink implements AudioSink
```

Callers above the abstraction should not need to know about Core Audio, but
readers should be able to open the backend crate and see exactly where the OS
boundary begins.

## 7. Python Bridge

The Python package is the ergonomic caller-facing layer.

It should expose:

```python
play_pcm_buffer(buffer: PCMBuffer) -> PlaybackReport
play_samples(samples: Iterable[int], *, sample_rate_hz: int, channel_count: int = 1) -> PlaybackReport
```

`play_pcm_buffer` adapts the existing Python `pcm_audio.PCMBuffer` type.

`play_samples` is the lower-level escape hatch for tests, examples, and callers
that already have PCM integers.

The Python wrapper must validate obvious Python-side mistakes before crossing
into Rust:

- sample rate must be integer-valued and greater than zero
- channel count must be `1`
- samples must fit signed 16-bit PCM
- booleans are not samples
- huge buffers must be rejected before allocating unbounded memory

The Rust extension receives only primitive data:

```text
_play_samples(samples, sample_rate_hz, channel_count)
```

It must copy samples into Rust-owned memory before calling the backend. It must
not retain Python object references across OS callbacks or blocking playback.

## 8. Safety Limits

Audio buffers are user-controlled input. The bridge and Rust core should use
small, explicit limits.

Initial V1 limits:

- at most `10` minutes of audio per blocking call
- at most `44_100 * 60 * 10` mono samples at the default rate
- sample rates above `384_000 Hz` are rejected

These limits are not audio theory. They are practical guardrails so accidental
calls do not allocate gigabytes or block for hours.

## 9. Testing Requirements

The Rust core crate must test:

- valid format construction
- invalid sample rates
- invalid channel counts
- invalid bit depths
- buffer duration math
- empty-buffer no-op report behavior using a fake sink
- typed error display strings

The Core Audio crate must test:

- backend name
- non-macOS unsupported behavior
- macOS smoke playback with a very short hidden/quiet buffer when CI supports it

The Python package must test:

- wrapper validation without needing a real audio device
- adaptation from `pcm_audio.PCMBuffer`
- delegation into the native function with primitive values
- unsupported-platform errors remain readable
- module exports stay stable

Unit tests should avoid playing audible sound by default. Any real-device smoke
test should use a tiny quiet buffer and should be skipped when the platform or
CI environment cannot safely open an output device.

## 10. Out of Scope for V1

V1 intentionally does not include:

- streaming playback
- stereo or multichannel buffers
- float PCM input
- device selection
- volume control
- pause/resume
- latency tuning
- mixing multiple notes
- scheduling melodies
- callbacks from the audio device thread
- Arduino, I2S, PWM, or other embedded output

Those are future packages. This spec only proves the first clean bridge from
our virtual audio world into an operating-system audio device.

## 11. End-to-End Example

Conceptually, a caller should be able to write:

```python
from note_audio import render_note_to_sound_chain
from audio_device_sink import play_pcm_buffer

rendered = render_note_to_sound_chain("A4", duration_seconds=0.25, amplitude=0.2)
report = play_pcm_buffer(rendered.pcm_buffer)

assert report.frames_played == rendered.pcm_buffer.sample_count()
```

This example still passes through every abstraction:

```text
note -> frequency -> oscillator -> sampler -> PCM -> Rust sink -> Core Audio -> speakers
```

The important boundary is that `audio_device_sink` starts at PCM. It is the
speaker-facing box, not the music-facing box.
