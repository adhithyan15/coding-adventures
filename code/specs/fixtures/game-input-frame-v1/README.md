# GAME11 input action frame v1 vectors

`cases.json` is the shared, explicit wire corpus for the pure GAME11 per-player
reducer. Tick and button-mask values are canonical decimal strings, preserving
all `u64` values in every language. Player IDs are exact JSON `u32` integers.

Run `python3 code/specs/fixtures/game-input-frame-v1/validate.py` from the
repository root to pin the original bytes, validate the exact case set and
shape, and check every hand-authored expected frame/error against the
independent Python reference. Language consumers must still compare the full
public result and exact error fields; the Python validator is not a Rust
implementation test.

The 255, 256, and 257 event vectors use explicit repeated no-op events so the
actual ordered slice length is visible to every consumer. The over-limit case
also contains invalid prior state and an invalid event to pin count-first
precedence.
