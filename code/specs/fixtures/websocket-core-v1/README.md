# WebSocket Core v1 Fixtures

This directory is the language-neutral executable oracle for the portable
portion of RFC 6455 described by `code/specs/websocket-core.md`.

`schema.json` closes the fixture shape. `cases.json` contains bounded success
and adversarial vectors for:

- accept-key derivation;
- client request construction;
- client response validation;
- server request acceptance;
- outbound frame encoding;
- incremental inbound frame decoding; and
- fragmented message and control-event assembly.

Byte strings use lowercase, byte-aligned hexadecimal. HTTP heads use JSON
strings with explicit CRLF sequences so fixture consumers do not inherit a
host newline convention. Error expectations carry a canonical typed code and a
payload-free diagnostic; they never reproduce header values, nonces, mask
keys, close reasons, or buffered payload bytes.

The Rust `websocket-core` integration test is the first behavior consumer.
Later ports must consume these same records without rewriting expected values.
Transport adapters are not fixture operations: sockets, DNS, TLS, timeouts,
random mask-key generation, event loops, retries, and application dispatch
remain outside this contract.

Validate the fixture schema with:

```sh
python -m unittest discover -s code/scripts/tests -p "test_websocket_core_fixtures.py"
```
