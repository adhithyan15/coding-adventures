# zwave-serial-api

Z-Wave Serial API request, response, callback, and controller capability
primitives.

`zwave-core` owns raw Serial API frame bytes. This crate starts the host-side
control-plane layer:

- function id constants
- request/response/callback classification
- request callback ids
- controller capability flag decoding
- Memory Get ID payload decoding
- request tracker with callback correlation and timeout expiry

It does not yet open a serial port, interview nodes, handle inclusion, or map
command classes.

## Dependencies

- `zwave-core`

## Development

```bash
bash BUILD
```
