# smart-home-zwave-host

Production byte-stream host for the normalized D23 Z-Wave runtime.

The host:

- opens an OS serial port at the configured baud rate and timeout
- implements Z-Wave Serial API SOF framing plus ACK, NAK, and CAN handling
- retries controller-rejected request frames within a bounded budget
- acknowledges valid controller frames and NAKs malformed frames
- runs the typed version, memory-id, capability, and init-data bootstrap
- installs the bootstrapped controller into `SmartHomeRuntime`
- preserves unsolicited application and callback frames while waiting for a
  matching response
- sends authorized commands through `smart-home-zwave-integration`, pumps
  callbacks and device reports, and publishes terminal command results

The included binary performs a real one-shot controller bootstrap:

```bash
cargo run -p smart-home-zwave-host --bin smart-home-zwave-host -- \
  --port /dev/ttyUSB0
```

Node inclusion and S2 remain explicit follow-on host state machines.

```bash
bash BUILD
```
