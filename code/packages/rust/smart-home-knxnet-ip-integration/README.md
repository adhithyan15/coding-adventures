# smart-home-knxnet-ip-integration

`smart-home-knxnet-ip-integration` discovers KNXnet/IP interfaces without
opening a tunneling or routing session. It binds one explicit local IPv4
interface, sends one Search Request to an explicit destination, strictly parses
a bounded number of Search Responses, and records verified interfaces in the
shared Smart Home discovery catalog after authorization.

The integration does not import ETS projects, interpret group addresses, send
bus telegrams, change device configuration, or hold KNX IP Secure keys. The CLI
is an operator inspection tool; production composition uses the authorized
runtime entry point.

```sh
cargo run -p smart-home-knxnet-ip-integration -- discover 192.168.1.20
```
