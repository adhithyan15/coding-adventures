# smart-home-esphome-discovery-integration

This package discovers ESPHome nodes through their official
`_esphomelib._tcp.local` mDNS service. It reuses the production Smart Home mDNS
scanner, validates the stable MAC identity and bounded device-information TXT
records, and records verified native-API endpoints after D23 authorization.

Discovery reports whether a node advertises Noise PSK use, Noise support, or
no security metadata. It never opens the protobuf native API, provisions a
key, reads entities, subscribes to state, or sends actions. Those operations
require a separate supervised TCP session owner and ephemeral Vault-backed
32-byte key custody.

Official references:

- <https://esphome.io/components/mdns/>
- <https://esphome.io/components/api/>
- <https://developers.esphome.io/architecture/api/protocol_details/>

```sh
cargo run -p smart-home-esphome-discovery-integration -- discover
```
