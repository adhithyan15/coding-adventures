# hue-client

Transport-neutral Philips Hue CLIP v2 client core with injectable HTTP transport.

`hue-client` owns the HTTP-shaped Philips Hue CLIP v2 primitive without owning
real network I/O. Runtime packages can provide a transport backed by TLS,
simulators, process sandboxes, or capability cages while keeping the request,
response, and mapping behavior stable.

Included surfaces:

- bridge registration request/response parsing
- resource snapshot and collection requests
- resource-specific reads
- structured command request bodies from `hue-core`
- event-stream request shape
- event-stream batch parsing from Server-Sent Events data frames
- incremental event-stream decoding for split Server-Sent Events chunks
- Hue v2 envelope/error parsing
- Hue bridge resource decoding for paired bridge identity and time zone data
- Hue device resource decoding with product metadata and service references
- Hue grouped-light resource decoding for room/zone aggregate lights
- Hue room, zone, and scene resource decoding
- Hue motion and button resource decoding for sensor/input entities
- Hue light resource decoding
- Hue light, motion, and button state update extraction from snapshots and
  event-stream batches

## Dependencies

- hue-core
- http-core
- coding-adventures-json-value
- coding-adventures-json-serializer

## Development

```bash
# Run tests
bash BUILD
```
