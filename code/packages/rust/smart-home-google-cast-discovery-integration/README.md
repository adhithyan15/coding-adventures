# smart-home-google-cast-discovery-integration

This package discovers Google Cast receivers through the official
`_googlecast._tcp.local` mDNS service. It reuses the production Smart Home mDNS
scanner, validates the CastV2 receiver UUID, protocol version, capability
bitfield, status, friendly name, model, and endpoint, then records verified D23
discovery candidates after authorization.

Discovery does not open the Cast TCP endpoint. It performs no TLS handshake,
receiver authentication, application launch, session management, queue access,
or media command. Those operations require a separate supervised Cast channel
owner and operation-specific policy.

Official references:

- <https://developers.google.com/cast/docs/discovery>
- <https://github.com/chromium/openscreen/blob/876b5381036e91ca05e21b1446f453ebccfc3acf/cast/common/public/receiver_info.h>
- <https://github.com/chromium/openscreen/blob/876b5381036e91ca05e21b1446f453ebccfc3acf/cast/docs/protocol_flow.md>

```sh
cargo run -p smart-home-google-cast-discovery-integration -- discover
```
