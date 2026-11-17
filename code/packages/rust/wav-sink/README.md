# wav-sink

Hardened WAV file sink and reader on top of the `pcm-audio` RIFF/WAVE codec (MUS01 Stage 8).

Moves `PcmAudio` buffers to and from filesystem WAV files with strict security hardening.

## Security & Verification Guarantees

- **Safety**: `#![forbid(unsafe_code)]`, zero external dependencies (pure `std`).
- **Required capabilities**: Explicitly scoped `fs:read` and `fs:write` in `required_capabilities.json`.
- **CWE-22 (Path Traversal & Symlink Defense)**:
  - Rejects empty paths and NUL bytes.
  - Rejects any `..` path components.
  - Requires `.wav` or `.wave` extension (case-insensitive).
  - Enforces `base_dir` containment when provided.
  - Prohibits symbolic links on target and ancestors via `symlink_metadata`.
  - Re-verifies regular file on open.
- **CWE-400 (Resource Exhaustion)**:
  - Reads bounded up front by `MAX_WAV_BYTES = 64 MiB`.
  - Stream bounded with `take(MAX_WAV_BYTES + 1)`.
- **Verification Warrant**: `wav-sink.warrant` and `code/specs/wav-sink.warrant`.
