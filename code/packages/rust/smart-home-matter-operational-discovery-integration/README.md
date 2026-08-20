# smart-home-matter-operational-discovery-integration

This package discovers commissioned Matter nodes through the official
`_matter._tcp.local` operational DNS-SD service. It reuses the production Smart
Home mDNS scanner, strictly validates the compressed fabric id and node id in
the service instance, validates resolved endpoints, and normalizes optional
MRP, TCP-support, and intermittently-connected-device TXT data into verified
D23 discovery candidates after authorization.

Discovery does not browse the commissionable `_matterc._udp` service, open the
advertised endpoint, or accept fabric credentials. It performs no PASE, CASE,
certificate validation, fabric membership check, Interaction Model read,
subscription, command, or control. Those operations require separate supervised
commissioning, secret-custody, secure-session, and operation-policy owners.

Official references:

- <https://github.com/project-chip/connectedhomeip/blob/master/src/lib/dnssd/ServiceNaming.h>
- <https://github.com/project-chip/connectedhomeip/blob/master/src/lib/dnssd/ServiceNaming.cpp>
- <https://github.com/project-chip/connectedhomeip/blob/master/src/lib/dnssd/TxtFields.h>
- <https://github.com/project-chip/connectedhomeip/blob/master/src/lib/dnssd/Types.h>

```sh
cargo run -p smart-home-matter-operational-discovery-integration -- discover
```
