# flash-player-host

Integrated Flash player application shell uniting container parsing, vector shape lowering, display list timeline execution, AVM1/AVM2 bytecode execution, audio synchronization, `UI11` windowing, `PaintVM` scene diffing, and deterministic headless SHA-256 verification.

Part of the **Coding Adventures** Flash Player suite (`SWF07`, see `code/specs/SWF00-flash-runtime-roadmap.md`).

---

## Architecture

```text
  .swf bytes (FWS / CWS / ZWS)
         │
         ▼
  swf-container (SWF01) ──► SwfHeader + bounded SwfTagIter
         │
         ├── DefineShape1..4      ──► swf-shape-to-paint (SWF02)
         ├── DefineSound / Stream ──► flash-audio-bridge (SWF06)
         ├── DoAction / DoABC     ──► avm-bytecode-lifter (SWF04) -> vm-core
         └── PlaceObject / Frame  ──► flash-display-list (SWF05)
                                             │
         UI11 (window-core) ─────────────────┤
         GAME10 (game-clock) ────────────────┤
                                             ▼
                                   FlashPlayer::player_step_with_clock
                                             │
                       ┌─────────────────────┴─────────────────────┐
                       ▼                                           ▼
         paint-palette-blend::diff_paint_scene_into     Headless SHA-256 Digest
         + paint-vm-runtime (P2D01 / P2D10)             (CI Regression Gate)
```

---

## Security & Hardening (`SWF00` §7)

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **CWE-78 / CWE-94 (Command & Script Injection):**
  - `fscommand("exec", ...)` is permanently disabled as a no-op and emits a `SecurityAuditEvent`.
  - `getURL` / `navigateToURL` URIs are strictly sanitized via `sanitize_navigation_uri`: whitespace/control-trimmed, embedded control characters rejected, allowing only `http:`, `https:`, or relative URIs while blocking `javascript:`, `file:`, `data:`, `vbscript:`, and all other schemes case-insensitively.
- **CWE-400 / CWE-125 / CWE-190 / CWE-369 / CWE-787:**
  - Frame script execution enforces `max_steps <= 2_000_000` per frame tick.
  - Display list hierarchy depth bounded to `64`, dictionary characters capped at `4096`, URI length capped at `2048`.
- **Zero-Allocation Steady-State Helpers (`VER07`):** Verified via `CountingAllocator` TAP microbenchmarks on `sanitize_navigation_uri`, `evaluate_fscommand`, `player_step_clock_and_audio_zero_alloc`, and `diff_paint_scene_into`.
