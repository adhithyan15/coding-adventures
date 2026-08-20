# smart-home-homekit-discovery-integration

This package discovers HomeKit Accessory Protocol IP accessories through the
official `_hap._tcp.local` mDNS service. It reuses the production Smart Home
mDNS scanner, validates the device identifier, configuration number, pairing
features, model, protocol version, fixed IP state number, status flags,
accessory category, optional setup hash, and endpoint, then records verified
D23 discovery candidates after authorization.

Discovery does not open the HAP TCP endpoint. It performs no setup-code input,
SRP pairing, pair verification, encrypted HTTP session, accessory read,
subscription, or control. Those operations require separate supervised pairing,
secret-custody, session-lifetime, and operation-policy owners.

Official references:

- <https://github.com/apple/HomeKitADK/blob/master/HAP/HAPIPServiceDiscovery.c>
- <https://developer.apple.com/apple-home/>

```sh
cargo run -p smart-home-homekit-discovery-integration -- discover
```
