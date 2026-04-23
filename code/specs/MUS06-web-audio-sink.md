# MUS06: Browser Web Audio Sink

## 1. Overview

`MUS02` defines the native audio-device sink:

```text
PCM buffer -> native audio sink -> operating-system audio API -> speakers
```

Browsers have a different final box. They do not load the Rust native sink
directly, and they should not pretend to have Core Audio, WASAPI, or ALSA.
Instead, the browser playback path is:

```text
PCM buffer -> Web Audio sink -> AudioContext -> browser/OS audio stack
```

This spec defines the first browser-friendly sink for already-rendered PCM
samples. It does not parse notes, render instruments, or mix tracks. Those
belong below `music-machine`.

## 2. Beginner Mental Model

The music machine eventually hands us a list of signed integers:

```text
0, 1024, 2048, 1024, 0, -1024, ...
```

Those are PCM samples. A browser `AudioContext` wants floating-point samples
instead:

```text
-1.0 <= sample <= 1.0
```

The Web Audio sink's job is therefore small and very visible:

1. Validate the PCM metadata.
2. Convert signed 16-bit integers into floats.
3. Copy those floats into an `AudioBuffer`.
4. Schedule an `AudioBufferSourceNode` to play through the context destination.

## 3. V1 Contract

Input format:

```text
PcmPlaybackBuffer(
    samples = signed 16-bit PCM integers,
    format = PcmFormat(
        sample_rate_hz,
        channel_count = 1,
        bit_depth = 16,
    ),
)
```

V1 requirements:

- mono only
- signed 16-bit PCM only
- finite positive integer sample rate
- optional gain in `[0.0, 1.0]`
- no filesystem, process, or network access
- no dependency on a native audio crate

Empty buffers are valid. Playing an empty buffer is a no-op report.

## 4. API Shape

Browser package:

```text
typescript/web-audio-sink
```

Required exports:

```text
validatePcmBuffer(buffer) -> normalized format
pcmSamplesToFloat32(samples, gain) -> Float32Array
createAudioBufferFromPcm(audio_context, buffer, options) -> AudioBuffer
playPcmBuffer(buffer, options) -> Promise<PlaybackReport>
```

`playPcmBuffer` accepts an optional injected `AudioContext`-like object. Tests
must use this seam so CI does not need real speakers.

## 5. Playback Report

Playback returns:

```text
PlaybackReport(
    frames_played,
    sample_rate_hz,
    channel_count,
    duration_seconds,
    backend_name = "web-audio",
)
```

For mono V1:

```text
frames_played == samples.length
```

## 6. Relationship to Future Web Music Packages

This sink is deliberately below the browser music machine.

Future packages can choose between:

- rendering PCM in TypeScript and passing it to this sink
- receiving PCM from a WebAssembly music renderer and passing it to this sink
- streaming chunks later through an `AudioWorklet`

V1 only schedules complete buffers. Streaming, low-latency keyboard input, and
`AudioWorklet` clocks should arrive after the buffer sink is easy to inspect.
