# zwave-core

Z-Wave identifier, region, and Serial API frame primitives for the smart-home
runtime.

Current scope:

- Home ID and classic/Long Range node id models
- region profile metadata
- network summary counts for classic/Long Range nodes and command-class coverage
- common command-class ids
- command-class payload frame parse/encode for short and extended class ids
- Serial API SOF/ACK/NAK/CAN constants
- Serial API request/response frame parse and encode
- checksum validation

Out of scope:

- controller request/callback correlation
- inclusion/exclusion
- S0/S2 security
- node interview
- command-class semantics beyond the shared byte frame
- Long Range topology behavior

## Development

```bash
bash BUILD
```
