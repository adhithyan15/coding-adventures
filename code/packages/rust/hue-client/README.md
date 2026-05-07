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
- Hue v2 envelope/error parsing
- Hue light resource decoding

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
