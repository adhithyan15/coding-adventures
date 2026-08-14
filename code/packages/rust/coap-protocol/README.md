# coap-protocol

Strict, bounded CoAP framing for read-only local telemetry.

The package encodes Confirmable GET requests with URI-Path and Accept options,
decodes piggybacked or separate responses, and exposes the empty ACK needed for
a separate Confirmable response. It performs no socket I/O and intentionally
does not implement writes, Observe subscriptions, multicast, blockwise
transfer, proxying, DTLS, OSCORE, or resource discovery.

```sh
bash BUILD
```
