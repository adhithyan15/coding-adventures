# smart-home-ssdp-discovery-integration

`smart-home-ssdp-discovery-integration` performs one authorized, bounded UPnP
SSDP search from an explicit local IPv4 interface. It strictly correlates the
response target, USN, source endpoint, and credential-free local HTTP
`LOCATION`, deduplicates stable UDN identities, and records verified D23
discovery candidates.

The runtime does not fetch device descriptions, subscribe to events, invoke
UPnP actions, accept public endpoints, or retain a discovery socket.

```sh
cargo run -p smart-home-ssdp-discovery-integration -- discover 192.168.1.20
```
