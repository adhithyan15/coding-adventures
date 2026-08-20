# smart-home-thread-border-agent-discovery-integration

This package discovers Thread Border Agents through the Thread-required
`_meshcop._udp.local` DNS-SD service. It reuses the production Smart Home mDNS
scanner, validates the record version, Border Agent identity, Thread version,
state bitmap, extended address, endpoint, and optional network, backbone,
routing, and vendor fields, then records verified D23 discovery candidates
after authorization.

Discovery opens no Border Agent UDP session. It accepts no PSKc, ePSKc,
operational dataset, network key, commissioner credential, or joiner code and
performs no MeshCoP exchange, commissioning, Thread transport, dataset read,
network mutation, or control. Those operations require separate supervised
session, secret-custody, transport, and policy owners.

Official references:

- <https://openthread.io/guides/border-router/mdns-discovery>
- <https://github.com/openthread/openthread/blob/main/include/openthread/border_agent_txt_data.h>
- <https://www.threadgroup.org/Portals/0/documents/support/ThreadBorderRouterWhitePaper_07192022_4001_1.pdf>

```sh
cargo run -p smart-home-thread-border-agent-discovery-integration -- discover
```
