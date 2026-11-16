# arcade-sfx

Deterministic, fixed-point arcade sound effect generator and preset synthesis engine (`GAME12`).

Part of the **Coding Adventures** polyglot monorepo real-time audio & game engine substrate.

---

## Architectural Role

`arcade-sfx` provides the sound effect synthesis substrate for vintage coin-op arcade titles and retro consoles (Pong `GAME13`, Breakout `GAME14`, etc.):

```text
[ SfxPreset ] ----> [ SfxPatch ] ----> [ SfxVoice / SfxBank<N> ]
(Pong, Breakout,     (DDS Waveform,     (Zero-Alloc Render & Mix)
 Coin, Laser)         ADSR, Sweeps)            |
                                               v
                                    [ audio-mixer-realtime ]
                                     (Ring Buffer / Staging)
                                               |
                                               v
                                    [ audio-device-sink ]
```

1. **Substrate Decoupling:** Standalone crate layering directly on `pcm-audio` (`MUS01`) and `audio-mixer-realtime` (`MUS08`), preserving modularity and isolating CI diff blasts.
2. **Deterministic Fixed-Point Synthesis:** Eliminates IEEE 754 floating-point drift across platforms, architectures, and compiler optimizers through 32-bit Direct Digital Synthesis (DDS) phase accumulators and Q16.16 fixed-point arithmetic.
3. **Hard Zero-Allocation Hot Path (`VER07`):** Sound synthesis routines (`render_sfx_into`, `mix_bank_into`, `trigger_preset`) operate strictly on caller-provided buffers with **zero heap allocations** (`max_alloc_bytes: Some(0)`), verified by `tests/microbenchmarks.rs` with `test-bootstrap`'s `CountingAllocator`.
4. **Retro Waveform Suite:** Full support for variable duty-cycle square waves (`Waveform::Square { duty_q16 }`), symmetric triangle waves (`Waveform::Triangle`), sawtooth ramps (`Waveform::Sawtooth`), and deterministic LFSR pseudo-random noise (`Waveform::LfsrNoise { short_mode }`).
5. **Sample-Counted ADSR Envelopes:** Integer sample-counted Attack, Decay, Sustain (Q16.16), and Release stages, supporting both automated one-shot duration completion and interactive dynamic premature note-off releases.
6. **Classic Arcade Presets:** Out-of-the-box presets for Pong (`PongPaddleHit`, `PongWallBounce`, `PongScore`), Breakout (`BreakoutBrickHit { row }`, `BreakoutLoseLife`), `CoinInsert` dual-tone chime, and `LaserSweep`.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08b`, `VER15`, `VER27`, `VER29`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **`CAP002` (Zero Unsafe / Safe Rust):** Compiled with `#![forbid(unsafe_code)]`.
- **Zero Allocation Budget (`VER07`):** Hard zero-allocation budget on `render_sfx_into`, `mix_bank_into`, and `trigger_preset`.
- **26-CWE Weakness Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes documented in `assurance_report.json`.

---

## Core Types & API

### 1. `Waveform`

```rust
pub enum Waveform {
    Square { duty_q16: u16 },
    Triangle,
    Sawtooth,
    LfsrNoise { short_mode: bool },
}
```

### 2. `AdsrEnvelope`

Sample-counted envelope with Q16.16 sustain level:

```rust
pub struct AdsrEnvelope {
    pub attack_samples: u32,
    pub decay_samples: u32,
    pub sustain_level_q16: u32,
    pub release_samples: u32,
}
```

Construct via `from_ms(attack_ms, decay_ms, sustain_level_q16, release_ms, sample_rate_hz)` or `from_samples(...)`.

### 3. `SfxPreset`

```rust
pub enum SfxPreset {
    PongPaddleHit,
    PongWallBounce,
    PongScore,
    BreakoutBrickHit { row: u8 },
    BreakoutLoseLife,
    CoinInsert,
    LaserSweep,
    Custom(SfxPatch),
}
```

### 4. `SfxVoice` & `SfxBank<const N: usize>`

- `SfxVoice::new(sample_rate_hz: u32) -> Result<Self, SfxError>`
- `voice.trigger_preset(preset: SfxPreset) -> Result<(), SfxError>`
- `render_sfx_into(&mut SfxVoice, &mut [i16]) -> Result<usize, SfxError>`
- `SfxBank::empty(sample_rate_hz: u32) -> Result<Self, SfxError>`
- `mix_bank_into(&mut SfxBank<N>, &mut [i16]) -> Result<usize, SfxError>`
- `voice.route_voice_to_ring_buffer(&mut ring, &mut scratch)`
- `bank.route_bank_to_ring_buffer(&mut ring, &mut scratch)`

---

## Verification & Testing

```bash
# Unit, differential, and security conformance tests
cargo test -p arcade-sfx -- --nocapture

# Release mode microbenchmarks and zero-allocation checks
cargo test -p arcade-sfx --release -- --nocapture

# Capability analyzer verification
ca-capability-analyzer-rs --dir . --check-module
ca-capability-analyzer-rs --dir . --verify-cert effect_certificate.json
```
