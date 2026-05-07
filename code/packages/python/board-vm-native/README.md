# coding-adventures-board-vm-native

Python sugar over the Rust Board VM language core.

The Python package does not implement request IDs, framing, CRCs, or Board VM
message layouts. Those remain in `board-vm-language-core`, with this package
wrapping that Rust boundary through the repo-local `python-bridge`.

```python
from board_vm_native import Session

session = Session()
frames = session.blink(program_id=1, instruction_budget=12).frames
read_frames = session.gpio_read(pin=13, mode="pullup").frames
```

Each frame is a Rust-produced Board VM wire frame ready for a transport to
write to a board. A `Session` can also dispatch through any object that exposes
`transact(frame, timeout_ms=...)` or `write(frame)`.

The REPL-style sugar is intentionally thin as well:

```python
session.run_command("gpio-read 13 pullup 24")
session.run_command("stop")
```

Python parses the friendly command text, then delegates module construction,
request IDs, framing, and response decoding to `board-vm-language-core` through
the repo-local `python-bridge` extension.
