# knxnet-ip-protocol

`knxnet-ip-protocol` implements the small KNXnet/IP wire boundary needed for
local interface discovery. It encodes Search Requests with an explicit UDP/IPv4
response endpoint and strictly decodes Search Responses, Device Information
Blocks, and Supported Service Families.

The package intentionally does not implement tunneling, routing telegrams,
device management, group-address interpretation, configuration, or KNX IP
Secure sessions. The framing and default discovery endpoint follow the KNX
Association's KNXnet/IP specification and current connection documentation:

- <https://support.knx.org/hc/en-us/articles/360000040999-KNX-Specifications>
- <https://support.knx.org/hc/en-us/articles/4402353231762-Connection-Manager-Detailed>

```rust
use knxnet_ip_protocol::encode_search_request;
use std::net::SocketAddrV4;

let endpoint: SocketAddrV4 = "192.0.2.5:50000".parse().unwrap();
let request = encode_search_request(endpoint).unwrap();
assert_eq!(request.len(), 14);
```
