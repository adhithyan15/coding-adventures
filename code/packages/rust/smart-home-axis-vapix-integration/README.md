# smart-home-axis-vapix-integration

Production-facing Axis VAPIX discovery, inspection, bounded PTZ control, and
supervised system-readiness events for D23.

The package:

- discovers documented `_axis-video._tcp.local` and `_axis-nvr._tcp.local`
  advertisements through the shared mDNS runtime;
- requires credential-free HTTPS origins for production, with plain HTTP
  permitted only for loopback transport tests;
- represents credentials as a `VaultRef` in request plans, probes without an
  authorization header, and selects a supported Basic or Digest challenge;
- prefers SHA-256 Digest over MD5 when a device advertises multiple supported
  challenges, uses CSPRNG-backed client nonces, maintains an in-memory nonce
  count, and retries once when the device returns a fresh or stale challenge;
- materializes Basic and Digest authorization values only inside the bounded
  transport, with credentials and derived response values in zeroizing memory;
- calls the authenticated Basic Device Information and API Discovery JSON CGIs;
- probes the documented image parameter inventory for VAPIX HTTP version 3,
  JPEG support, camera-1 availability, and camera-1 enablement before
  advertising `camera.snapshot`;
- probes the documented PTZ command inventory, current position, server presets,
  native speed control, and `CtlQueueing` mode before advertising `camera.ptz`;
- recalls only probed presets and bounds continuous directional movement to five
  seconds, followed by an explicit `0,0` stop;
- obtains exclusive PTZ control through `ptzqueue.cgi` when queueing is enabled,
  keeps the returned cookie out of request plans and debug output, and drops the
  queue lease after each command;
- normalizes Axis identity, firmware, product type, and the supported VAPIX API
  list into one confirmed camera entity; and
- authorizes D23 reads and human-approved PTZ commands before credentials or
  transport are touched;
- preserves confirmed position from inspection instead of inventing optimistic
  orientation after movement;
- mints the documented 15-second `wssession` token through the same bounded
  authenticated transport, then opens the exact certificate-verifying WSS
  event endpoint without exposing the token in debug or runtime state;
- configures only the documented stateful SystemReady topic, verifies the
  configuration acknowledgement and strict notification shape, normalizes one
  expiring event-sourced readiness state, and closes the WebSocket explicitly;
- uses the shared WebSocket codec for strict handshakes, masking, control-frame
  replies, message bounds, and extension rejection, and the shared event-stream
  state for lifecycle and checkpoints.

The production path intentionally targets VAPIX camera 1 and the SystemReady
event only. Snapshot execution is composed by `smart-home-axis-snapshot-host`;
broader event topics, media transfer, multi-channel enumeration, and advanced
zoom/guard-tour control remain separate slices with additional schema, privacy,
host, or capability prerequisites.

Protocol references:

- [Axis VAPIX authentication](https://developer.axis.com/vapix/authentication/)
- [Axis basic device information](https://developer.axis.com/vapix/network-video/basic-device-information/)
- [Axis API Discovery](https://developer.axis.com/vapix/network-video/api-discovery-service/)
- [Axis mDNS-SD](https://developer.axis.com/vapix/network-video/mdns-sd-api/)
- [Axis PTZ API](https://developer.axis.com/vapix/network-video/pantiltzoom-api/)
- [Axis video streaming API](https://developer.axis.com/vapix/network-video/video-streaming/)
- [Axis image parameters](https://developer.axis.com/vapix/network-video/parameter-management/image-api/)
- [Axis event streaming over WebSocket](https://developer.axis.com/vapix/network-video/event-streaming-over-websocket/)
- [Axis event and action services](https://developer.axis.com/vapix/network-video/event-and-action-services/)
