# audio-mixer-realtime

Deterministic, fixed-point real-time audio mixer and voice allocator (`MUS08`).

Part of the **Coding Adventures** polyglot monorepo real-time audio & game engine substrate.

---

## Architectural Role

`audio-mixer-realtime` serves as the additive substrate layer bridging raw audio generators (`arcade-sfx`, `pcm-audio`, `oscillator`, `percussion-kit`) to backend playback interfaces (`audio-device-sink`):

```text
[ arcade-sfx ] ----\
[ percussion ] -----> [ audio-mixer-realtime ] ----> [ audio-device-sink ]
[ oscillator ] ----/     (Zero-Alloc Mixer)           (Device Backend)
```

1. **Substrate Decoupling:** Standalone crate layering directly on `pcm-audio` (`MUS01`) and `audio-device-sink` (`MUS02`), preserving modularity and isolating CI diff blasts.
2. **Fixed-Point Predictability (Q16.16):** Eliminates floating-point non-determinism and cross-platform rounding discrepancies by using pure integer and Q16.16 fixed-point arithmetic for gains, panning, and pitch-stepping phase accumulators.
3. **Hard Zero-Allocation Hot Path (`VER07`):** The primary mixing routine `mix_channels_into` operates strictly on caller-provided buffers with **zero heap allocations** (`max_alloc_bytes: Some(0)`), verified by `tests/microbenchmarks.rs` with `test-bootstrap`'s `CountingAllocator`.
4. **Ring Buffer Staging:** Provides bounded single-producer/single-consumer `PcmRingBuffer<const N: usize>` staging without reallocation (`CWE-400`).
5. **CWE-190 Saturation Defense:** Channel accumulation utilizes 64-bit signed integers (`i64`), saturating cleanly to $[-32768, 32767]$ without integer overflow wrap-around even when 16 simultaneous channels output full-scale waveforms.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08b`, `VER15`, `VER27`, `VER29`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **`CAP002` (Zero Unsafe / Safe Rust):** Compiled with `#![forbid(unsafe_code)]`.
- **Zero Allocation Budget (`VER07`):** Hard zero-allocation budget on `mix_channels_into`, `PcmRingBuffer::push`, and `PcmRingBuffer::pop`.
- **26-CWE Weakness Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes documented in `assurance_report.json`.

---

## Core Types & API

### 1. `VoiceSlot`

A `Copy` struct describing the playback state of an individual voice:

```rust
pub struct VoiceSlot {
    pub active: bool,
    pub voice_id: u32,
    pub priority: u8,
    pub cursor: usize,
    pub gain_q16: u32,
    pub pan_q16: i32,
    pub pitch_step_q16: u32,
    pub phase_q16: u64,
    pub looping: bool,
}
```

### 2. `RealtimeMixerState`

State holding up to 16 simultaneous mixing slots:

```rust
pub struct RealtimeMixerState {
    pub slots: [VoiceSlot; MAX_MIXER_CHANNELS],
    pub master_gain_q16: u32,
    pub sample_rate_hz: u32,
    pub clipped_samples: u64,
    pub mixed_frames: u64,
}
```

Key operations:
- `RealtimeMixerState::new(sample_rate_hz: u32) -> Result<Self, MixerError>`
- `allocate_voice(voice_id, priority, gain_q16, looping) -> Result<usize, MixerError>`
- `set_voice_pitch(slot_idx, pitch_step_q16) -> Result<(), MixerError>`
- `set_voice_gain(slot_idx, gain_q16) -> Result<(), MixerError>`
- `stop_voice(voice_id)`
- `reset()`

### 3. `PcmRingBuffer<const N: usize>`

Fixed-capacity single-producer / single-consumer FIFO buffer:

```rust
pub struct PcmRingBuffer<const N: usize> {
    pub head: usize,
    pub tail: usize,
    pub len: usize,
    pub overruns: u64,
    pub underruns: u64,
    pub samples: [i16; N],
}
```

### 4. Zero-Allocation Hot Paths & Bridges

- `mix_channels_into(state: &mut RealtimeMixerState, sources: &[&[i16]], out_mono: &mut [i16]) -> Result<usize, MixerError>`
- `export_to_sink_buffer(samples: &[i16], sample_rate_hz: u32) -> Result<audio_device_sink::PcmPlaybackBuffer, MixerError>`
- `export_to_pcm_audio(samples: &[i16], sample_rate_hz: u32) -> Result<pcm_audio::PcmAudio, MixerError>`

---

## Verification & Conformance

Run test suite:
```bash
cargo test -p audio-mixer-realtime
```

Run capability analyzer check:
```bash
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
```
