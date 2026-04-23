# MUS04: Portable Music Machine and Multi-Track Score Format

## 1. Overview

`MUS03` proved the first end-to-end path:

```text
text score -> note/rest events -> PCM -> audio device
```

That format is intentionally beginner-sized: one melody line, one global sound,
and one global tempo. `MUS04` defines the next layer:

```text
portable score text -> multi-track score model -> render/playback backends
```

The goals are:

- make scores easy for humans to read
- make scores easy for programs to generate
- support multiple instruments playing different parts at the same time
- give every language package the same parse/render/play contract
- use native bridges or C FFI where sharing a Rust core is the safest path
- use browser Web Audio for browser playback instead of pretending browsers can
  load the same native audio sink as Node.js

## 2. Beginner Mental Model

A `MUS03` score is like one person humming a melody.

A `MUS04` score is like a small band:

- the score says the song title, tempo map, and meter
- instruments describe what kind of sound a track should use
- tracks describe who plays what
- events describe when notes start, how long they last, and how loud they are

For example:

```text
format: music-machine-score/v2
title: Tiny Duet
ppq: 480
tempo 0 120
meter 0 4/4

instrument lead kind=sine gain=0.08
instrument bass kind=sine gain=0.05

track melody instrument=lead
event melody 0 480 note C5 velocity=0.8
event melody 480 480 note D5 velocity=0.8

track bassline instrument=bass
event bassline 0 960 note C3 velocity=0.7
```

The melody and bassline are separate tracks. Their events can overlap because
each event has an explicit start time.

## 3. Why a New Text Format?

`MUS03` has a friendly shorthand:

```text
C4/q D4/q E4/q
```

That is wonderful to type by hand, but not ideal for future tools that detect
notes from audio samples. A sample-to-sheet pipeline will naturally produce
records such as:

```text
track = melody
start_tick = 1920
duration_tick = 240
pitch = E5
confidence = 0.91
```

`MUS04` therefore defines a canonical event form that is easy to emit from
programs, plus optional human shorthand that packages may add later. The
canonical form is the required interchange format.

## 4. Canonical Text Format

Files are UTF-8 text. Blank lines are ignored. Lines starting with `#` are
comments.

The first non-comment directive must identify the format:

```text
format: music-machine-score/v2
```

Global directives:

```text
title: Fur Elise Sketch
ppq: 480
sample_rate: 44100
```

`ppq` means pulses per quarter note. It is the integer time grid used by the
canonical event format. At `ppq: 480`, a quarter note is `480` ticks, an eighth
note is `240` ticks, and a half note is `960` ticks.

Tempo changes:

```text
tempo <start_tick> <bpm>
```

Meter changes:

```text
meter <start_tick> <numerator>/<denominator>
```

Instrument declarations:

```text
instrument <instrument_id> kind=<kind> gain=<0.0..1.0>
```

Required V2 instrument kinds:

| Kind | Meaning |
|------|---------|
| `sine` | pure sine oscillator, same musical model as the current stack |
| `silence` | useful for parser and scheduler tests |
| `external` | placeholder for host-specific instruments not modeled yet |

Track declarations:

```text
track <track_id> instrument=<instrument_id>
```

Events:

```text
event <track_id> <start_tick> <duration_tick> note <pitch-list> [velocity=<0.0..1.0>]
event <track_id> <start_tick> <duration_tick> rest
```

Examples:

```text
event melody 0 240 note E5 velocity=0.65
event melody 240 240 note D#5 velocity=0.65
event piano_left 0 960 note A2,C3,E3 velocity=0.50
event drums 480 120 rest
```

Rules:

- identifiers must match `[A-Za-z_][A-Za-z0-9_-]*`
- tick values must be non-negative integers
- durations must be positive integers
- `pitch-list` is one or more `MUS00` note names separated by commas
- multiple pitches in one event form a chord
- events may overlap
- event order in the file does not define timing; `start_tick` does
- renderers should preserve source order only as a stable tie-breaker when two
  events have the same start tick

## 5. Score Model

Every language should expose equivalent data shapes:

```text
PortableScore(
    format_version,
    title,
    ppq,
    sample_rate_hz,
    tempo_events,
    meter_events,
    instruments,
    tracks,
    events,
)
```

Tempo event:

```text
TempoEvent(start_tick, bpm)
```

Meter event:

```text
MeterEvent(start_tick, numerator, denominator)
```

Instrument:

```text
Instrument(id, kind, gain)
```

Track:

```text
Track(id, instrument_id)
```

Score event:

```text
ScoreEvent(
    track_id,
    start_tick,
    duration_tick,
    kind = note | rest,
    pitches,
    velocity,
)
```

The model should be serializable to deterministic JSON so golden fixtures can
be shared across languages.

## 6. Rendering Semantics

The renderer lowers score events to PCM in three phases:

1. Convert ticks to seconds using the tempo map.
2. Render each note event with the referenced instrument.
3. Mix all tracks into one PCM buffer.

V2 requirements:

- renderers must support `sine` instruments
- renderers must support chords by rendering each pitch and summing samples
- renderers must support overlapping events across tracks
- renderers must clamp or otherwise safely bound mixed samples to signed
  16-bit PCM
- renderers must enforce a maximum event count and maximum rendered sample count
- renderers must reject unknown instruments, tracks, notes, meters, and tempo
  events with useful errors

Future instrument work can add harmonic tables, envelopes, samples, and physical
models. V2 deliberately keeps the sound model simple so the cross-language
contract lands first.

## 7. Playback Semantics

Playback is separate from parsing and rendering.

Required API shape:

```text
parse_score_text(text) -> PortableScore
render_score_to_pcm(score, options) -> PCMBuffer
play_score_text(text, options) -> PlaybackReport
```

Packages may omit `play_score_text` when their runtime cannot access audio
devices. They should still provide parse and render behavior when possible.

## 8. Rust Core and FFI Strategy

The preferred shared implementation path is a Rust core crate:

```text
music-machine-core
```

It should own:

- canonical text parsing
- validation
- deterministic JSON serialization of the score model
- sine/chord/multi-track rendering to PCM
- resource-limit enforcement

A C ABI facade should be added only after the Rust core is stable:

```text
music-machine-ffi
```

The ABI must avoid Rust enum values in caller-controlled structs. Use primitive
integer status codes and opaque handles instead.

Suggested ABI shape:

```text
music_machine_parse(
    text_ptr,
    text_len,
    options_json_ptr,
    options_json_len,
    out_score_handle,
) -> status_code

music_machine_score_to_json(score_handle, out_bytes) -> status_code
music_machine_render_pcm16(score_handle, options_json, out_pcm_buffer) -> status_code
music_machine_free_score(score_handle)
music_machine_free_bytes(bytes_handle)
music_machine_last_error_json(out_bytes) -> status_code
```

All byte buffers returned across the ABI must have one obvious free function.
All input sizes must be validated before allocation.

## 9. Language Porting Plan

Porting should happen in small PRs, grouped by risk and toolchain behavior.

Tier 0: shared fixtures

- add canonical `.mmscore` fixtures
- add expected JSON model fixtures
- add expected PCM metadata fixtures

Tier 1: Rust foundation

- implement `music-machine-core`
- implement `music-machine-ffi`
- implement Rust playback by composing with `audio-device-sink`

Tier 2: native bridge languages already present in the repo

- Python can either keep the existing pure package or switch to the Rust core
- Ruby should use `ruby-bridge`
- Lua should use `lua-bridge`
- Perl should use `perl-bridge`
- Elixir should use `erl-nif-bridge`
- Node.js should use `node-bridge`

Tier 3: managed and compiled language ports

- C#, F#, Java, Kotlin, Swift, Go, Haskell, Dart, and C-style consumers should
  prefer the C ABI where practical
- pure educational implementations are acceptable when the language has no
  stable native bridge yet, but they must match shared fixtures

Tier 4: browser and WebAssembly

- TypeScript browser playback should use Web Audio
- WebAssembly can share parser/rendering logic with Rust later
- browser audio output must remain Web Audio, because browsers do not load the
  native OS audio sink used by Node.js

Starlark may support parser-only behavior if the repository keeps it sandboxed
and intentionally unable to play audio.

## 10. Node.js Package

Node.js should get a native package:

```text
@coding-adventures/music-machine-node
```

It should:

- load a Rust N-API addon through `node-bridge`
- parse scores through the Rust core
- render PCM through the Rust core
- play PCM through the Rust audio-device sink
- expose explicit parse/render/play functions

Node.js should not use browser Web Audio as its primary backend. Node does not
provide a built-in browser `AudioContext`; the stable path in this repo is the
Rust native audio sink.

## 11. Browser Package

Browsers should get a separate package:

```text
@coding-adventures/music-machine-web
```

It should:

- parse the canonical V2 text format in TypeScript or WebAssembly
- create or receive an `AudioContext`
- schedule `OscillatorNode`s for `sine` instruments
- control volume through `GainNode`
- require callers to start playback from a user gesture when browsers require
  it
- expose offline rendering later through `OfflineAudioContext`
- consider `AudioWorklet` later for custom sample-by-sample rendering

The Web Audio API is widely available in modern browsers. `AudioContext` owns
the audio graph, `OscillatorNode` can synthesize periodic tones, and
`OfflineAudioContext` can render audio into a buffer without sending it to the
speakers. Browser implementations must avoid deprecated `ScriptProcessorNode`
for new sample-processing work; `AudioWorklet` is the future path for custom
processing.

References:

- W3C Web Audio API: `https://www.w3.org/TR/webaudio/`
- MDN Web Audio API: `https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API`
- MDN AudioContext: `https://developer.mozilla.org/en-US/docs/Web/API/AudioContext`
- MDN OscillatorNode: `https://developer.mozilla.org/en-US/docs/Web/API/OscillatorNode`

## 12. Safety and Resource Limits

All implementations must reject:

- score text above the package maximum
- individual lines above the package maximum
- event counts above the package maximum
- track counts above the package maximum
- instrument counts above the package maximum
- render requests above the maximum sample count
- tempo values that are non-finite or not positive
- gain and velocity values outside `[0.0, 1.0]`
- unknown instrument references
- unknown track references
- malformed note spellings

Playback APIs must be explicit. Importing or parsing a score must never open an
audio device.

## 13. Testing Requirements

Every implementation must test:

- parsing global directives
- parsing instruments
- parsing tracks
- parsing note, rest, and chord events
- rejecting malformed identifiers and unknown references
- converting ticks to seconds at known tempos
- rendering overlapping tracks
- clipping or bounding mixed samples
- enforcing parser and renderer resource limits
- serializing the score model to deterministic JSON

Packages with playback must test delegation without requiring real audio output
in unit tests.

Browser packages must test scheduling against fake or mocked Web Audio objects
rather than making CI produce sound.

## 14. PR Slicing

Do not port all languages in one PR.

Recommended sequence:

1. `MUS04` spec and shared fixtures.
2. Rust `music-machine-core`.
3. Rust `music-machine-ffi`.
4. Node native package.
5. Browser Web Audio package.
6. Python alignment with V2.
7. Ruby/Lua/Perl/Elixir bridge packages.
8. Go/Swift/.NET/JVM/Haskell/Dart packages in small groups.

Each PR should build only the packages it owns plus direct prerequisites.
