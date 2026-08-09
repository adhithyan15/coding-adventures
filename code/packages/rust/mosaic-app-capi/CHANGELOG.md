# Changelog

## 0.1.0

- Add fixed C layouts for input bytes, owned output buffers, opaque app handles,
  and status codes.
- Add a reusable export macro for create, dispatch, snapshot, restore, destroy,
  and buffer-free symbols.
- Contain Rust panics, poison runtimes after a panic, and return bounded UTF-8
  diagnostics through the normal output buffer.
- Test lifecycle, retry, protocol failures, application failures, panic poisoning,
  invalid pointers, snapshots, restores, and buffer ownership.
