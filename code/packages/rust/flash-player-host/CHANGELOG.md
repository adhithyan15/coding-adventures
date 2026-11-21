# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- Made `player_step_with_clock` atomic on `Err` by rolling back `(clock, stage, audio, pending_initial_frame, stage_dirty, processed_action_log_len)` while preserving `self.audit_log`, and reordered `player_step_clock_and_audio_zero_alloc` to validate `advance_frame`, `diff_paint_scene_into`, and `audio.advance_frame_clock` before mutating `clock`/`stage`.
- Registered `flash_get_property`, `flash_set_property`, `flash_invoke_method`, and `flash_new_array` in `register_avm_builtins_for_env` (and routed all `execute_iir_module_bounded` budgets through it) so `player_execute_source_script` blocks and audits `__proto__`/`prototype`/`constructor` (CWE-1321).
- Canonicalized `NaN` payloads (`if v.is_nan() { f64::NAN }`) alongside `-0.0 -> +0.0` in `digest::hash_f64`.
- Added `tests/hostile_inputs.rs` (`#![forbid(unsafe_code)]`) covering hostile URIs, hostile `WindowEvent`s, atomic rollback on `PatchBufferTooSmall`/`HierarchyDepthExceeded`/`ScriptTimeout` (with `audit_log` preserved), CWE-1321 inline script blocking, canonical `NaN` digests, and a seeded 1,500-case hostile SWF/URI/event fuzz campaign.

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `flash-player-host` (`SWF07`).
- `FlashPlayer` session loader uniting `swf-container`, `swf-shape-to-paint`, `flash-display-list`, `flash-audio-bridge`, `avm-bytecode-lifter`, `actionscript-compiler`, `window-core`, `paint-vm-runtime`, `paint-palette-blend`, and `game-clock`.
- Fixed-timestep frame loop (`player_step_with_clock`) dispatching `UI11` (`window-core`) mouse/keyboard events, stepping `MovieClip` timelines and AVM1/AVM2 frame scripts (`max_steps <= 2_000_000`), syncing audio, lowering `Stage` to `PaintScene`, and diffing via `diff_paint_scene_into`.
- `FlashHostSession<B: WindowBackend>` uniting `window-core` host presentation with `paint-vm-runtime` backend selection and rendering.
- Headless CI runner (`run_headless_swf`, `verify_headless_digests`) producing canonical deterministic SHA-256 `PaintScene` and PCM digests without a display server.
- Mandatory CWE-94 / CWE-78 defenses: `"capabilities": []`, permanently disabled `fscommand("exec")` with diagnostic audit events, and strict `getURL` / `navigateToURL` URI scheme sanitization.
- Conformance, `CountingAllocator` zero-alloc microbenchmark, and runtime capability test suites.
