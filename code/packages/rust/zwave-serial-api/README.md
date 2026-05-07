# zwave-serial-api

Z-Wave Serial API request, response, callback, and controller capability
primitives.

`zwave-core` owns raw Serial API frame bytes. This crate starts the host-side
control-plane layer:

- function id constants
- request/response/callback classification
- request callback ids
- controller capability flag decoding
- Serial API Get Init Data node inventory decoding
- Application Command Handler source/command envelopes
- Memory Get ID payload decoding
- request tracker with callback correlation and timeout expiry

It does not yet open a serial port, interview nodes, handle inclusion, or decode
command-class payload semantics.

## Dependencies

- `zwave-core`

## Development

```bash
bash BUILD
```
