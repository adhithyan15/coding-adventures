# @coding-adventures/forme-dev-server

The local preview boundary for Forme. It serves successful `dist-tree`
`DeployArtifact` outputs directly from memory and not from a partially written
filesystem tree. Successful publishes notify browsers over Server-Sent Events;
failed rebuilds keep the last good snapshot available and surface an error
overlay without replacing it.

The server binds `127.0.0.1` by default. It exposes three reserved endpoints:

- `/__forme/events` — reconnecting live-reload event stream.
- `/__forme/client.js` — the injected browser client.
- `/__forme/status` — current build state, last good build ID, and error.

HTML receives only an external module script tag at response time. Artifact
bytes remain unchanged, hashed assets retain their authored MIME types, every
response is `no-store`, and the package performs no project filesystem I/O.
