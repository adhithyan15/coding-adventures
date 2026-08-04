# hue-integration

Production Philips Hue LAN workers for the D23 smart-home runtime.

The integration owns real bounded HTTP/TLS execution for the transport-neutral
`hue-client`, decrypts paired application keys only inside the worker, and
projects bridge snapshots and Server-Sent Events into normalized runtime
devices, entities, state, health, and command audit.

The worker supports:

- full CLIP v2 snapshot refreshes
- authorized light command dispatch
- bounded event-stream polling with reconnect-safe SSE decoding
- actor messages for refresh and event-poll work
- strict certificate verification with caller-supplied Hue trust roots

Runtime state, actor messages, reports, and errors carry only opaque
`VaultRef` handles and redacted credential metadata.

## Development

```bash
bash BUILD
```
