# flash-audio-bridge

Deterministic, zero-allocation SWF audio tag parser, decoder, mixer, and timeline playback synchronization bridge for Rust.

Part of the **Coding Adventures** Flash Player suite (`SWF06`, see `code/specs/SWF00-flash-runtime-roadmap.md`).

---

## Overview

`flash-audio-bridge` bridges Shockwave Flash (`.swf`) audio tags from `swf-container` (`SWF01`) to the repository's deterministic audio and timing substrates (`pcm-audio`, `audio-mixer-realtime` `MUS08`, `audio-device-sink` `MUS02`, and `game-clock` `GAME10`):

1. **SWF Audio Tag Parsing (`tags`)**:
   - `DefineSound` (Tag 14): Validates sound ID (`1..=65535`), format (`UncompressedNe`, `Adpcm`, `Mp3`, `UncompressedLe`), sample rate (`5512`, `11025`, `22050`, `44100` Hz), 8-bit/16-bit sample width, mono/stereo layout, bounded sample count (`<= MAX_SOUND_SAMPLES`), and payload bounds.
   - `StartSound` (Tag 15) / `StartSound2` (Tag 89): Parses `sync_stop`, `sync_no_multiple`, `in_point`, `out_point`, `loop_count` (`1..=MAX_LOOP_COUNT`), up to `MAX_ENVELOPE_POINTS` stereo `SOUNDENVELOPE` points, and AS3 class names.
   - `SoundStreamHead` (Tag 18) / `SoundStreamHead2` (Tag 45) / `SoundStreamBlock` (Tag 19): Parses stream format, average sample count, MP3 latency seek, and frame-aligned audio payloads.
2. **Zero-Allocation Audio Decoders (`decoder`)**:
   - **Uncompressed PCM**: 8-bit unsigned (`0..=255` centered at `128`) and 16-bit signed little-endian / native-endian (`decode_pcm8_into`, `decode_pcm16_le_into`, `decode_pcm16_ne_into`), plus stereo-to-mono downmixing (`mix_stereo_to_mono_into`).
   - **Flash ADPCM**: Full 2-bit, 3-bit, 4-bit, and 5-bit mono/stereo decoding in 4,096-sample packets with IMA step-size and bit-width-specific index tables (`decode_adpcm_into`).
   - **MP3 Frame Validation & Extraction**: Validates MPEG-1/2/2.5 Layer III frame headers (`parse_mp3_frame_header`) and extracts bounded frame slices (`Mp3FrameIter`, `extract_define_sound_mp3_info`).
3. **Event Mixer, Streaming FIFO & Clock Sync (`mixer`)**:
   - `FlashAudioState`: Caches up to `MAX_EVENT_SOUNDS = 32` decoded event sounds, mixes up to `MAX_ACTIVE_VOICES = 16` concurrent voices with Q16.16 fixed-point resampling, looping, and 44.1 kHz envelope interpolation, and reads frame-synced streaming audio from a `PcmRingBuffer<STREAM_FIFO_CAPACITY>`.
   - ActionScript APIs: `as_sound_start` (`Sound.start()`), `as_stop_all_sounds` (`stopAllSounds()`), `advance_frame_clock` (`game-clock` `GAME10`).
   - Zero-allocation hot path: `render_flash_audio_into(&mut state, &mut out_i16)` and `bridge_render_to_ring_buffer`.
   - Offline bridges: `export_flash_audio_to_pcm_audio` (`pcm-audio`) and `export_flash_audio_to_sink_buffer` (`audio-device-sink`).

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero Allocation Hot Paths (`VER07`):** PCM/ADPCM decoding, MP3 frame iteration, voice triggering, streaming block queuing, `render_flash_audio_into`, and `bridge_render_to_ring_buffer` perform zero heap allocations, verified with `CountingAllocator` in `tests/microbenchmarks.rs`.
- **CWE Weakness Closures (`VER15`):** Complete 26-CWE matrix in `assurance_report.json` with defenses against CWE-20, CWE-125, CWE-190, CWE-369, CWE-400, and CWE-787.

---

## Usage

```rust
use flash_audio_bridge::{
    parse_define_sound, parse_start_sound, render_flash_audio_into, FlashAudioState,
};

let mut engine = FlashAudioState::create_engine(44_100, 24)?;
let define_tag = parse_define_sound(&define_sound_bytes)?;
engine.register_define_sound(&define_tag)?;

let start_tag = parse_start_sound(&start_sound_bytes)?;
engine.apply_start_sound(&start_tag)?;

let mut pcm_out = [0i16; 1024];
render_flash_audio_into(&mut engine, &mut pcm_out)?;
# Ok::<(), flash_audio_bridge::AudioBridgeError>(())
```
