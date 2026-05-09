# smart-home-capability-cage

Pure smart-home worker policy projections for the generic `capability-cage`
runtime.

This crate does not launch workers, open sockets, read devices, or resolve
secrets. It gives integration hosts a deterministic way to turn smart-home
worker needs into cage manifests for:

- local HTTP, MQTT, cloud API, and webhook network access
- serial/radio adapter filesystem access
- sidecar process execution
- stdout/time permissions used by supervised workers
- D23 command/read capability hints that stay separate from OS permissions

Runtime hosts can inspect these profiles before constructing a sidecar, process
sandbox, WASI host, or first-party Rust worker.

## Dependencies

- capability-cage
- smart-home-core

## Development

```bash
bash BUILD
```
