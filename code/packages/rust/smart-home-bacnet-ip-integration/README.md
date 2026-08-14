# smart-home-bacnet-ip-integration

`smart-home-bacnet-ip-integration` performs bounded BACnet/IP device discovery.
It sends one Who-Is request to an explicit IPv4 destination, strictly parses a
limited number of I-Am replies, and projects verified devices into the shared
Smart Home discovery catalog after the caller is authorized for discovery.

The integration has no property-read or control surface. It does not manage a
BBMD, register as a foreign device, or implement BACnet/SC. The included CLI is
an explicit operator diagnostic; production composition uses the authorized
runtime entry point.

```sh
cargo run -p smart-home-bacnet-ip-integration -- discover 192.168.1.255
```
