# smart-home-bacnet-ip-integration

`smart-home-bacnet-ip-integration` performs bounded BACnet/IP device discovery
and read-only Device-object inspection. It sends one Who-Is request to an
explicit IPv4 destination, strictly parses a limited number of I-Am replies,
and projects verified devices into the shared Smart Home discovery catalog
after the caller is authorized for discovery.

After D23 read authorization, one configured private, link-local, or loopback
IPv4 endpoint can be inspected through a fixed allowlist of eight standardized
Device properties: object name, system status, vendor name and identifier,
model name, firmware revision, application software version, and protocol
version. Every UDP response is bounded and must correlate the connected peer,
invoke id, service, Device instance, property id, and expected value type before
one normalized network-diagnostic entity is installed.

The integration has no generic property-read or control surface. It does not
read point objects, send writes, accept public endpoints, use forwarded NPDUs,
manage a BBMD, register as a foreign device, or implement BACnet/SC. The
included CLI remains an explicit discovery diagnostic; production composition
uses the authorized runtime entry points.

```sh
cargo run -p smart-home-bacnet-ip-integration -- discover 192.168.1.255
```
