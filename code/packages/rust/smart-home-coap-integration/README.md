# smart-home-coap-integration

Authorized read-only CoAP telemetry for explicitly configured local devices.

The runtime:

- sends bounded Confirmable GET requests to one explicit private, link-local,
  or loopback unicast endpoint;
- correlates the exact peer, token, and acknowledgement message ID;
- handles piggybacked and one bounded separate response;
- decodes profile-selected plain-text or JSON number, boolean, and text values;
- installs normalized D23 sensor state only after the complete profile passes;
- authorizes `GetState` before opening a UDP socket.

It does not expose PUT, POST, DELETE, Observe, multicast discovery, blockwise
transfer, proxying, DTLS, or OSCORE. Unauthenticated CoAP is therefore kept
read-only and local.

Inspect one resource:

```sh
cargo run -p smart-home-coap-integration -- \
  inspect 192.168.1.40:5683 room-sensor temperature /temperature text-number C
```

Validate:

```sh
bash BUILD
```
