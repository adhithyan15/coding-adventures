# smart-home-axis-vapix-integration

Production-facing Axis VAPIX discovery, inspection, and bounded PTZ control for
D23.

The package:

- discovers documented `_axis-video._tcp.local` and `_axis-nvr._tcp.local`
  advertisements through the shared mDNS runtime;
- requires credential-free HTTPS origins for production, with plain HTTP
  permitted only for loopback transport tests;
- represents credentials as a `VaultRef` in request plans and materializes the
  Basic authorization value only inside the bounded transport;
- calls the authenticated Basic Device Information and API Discovery JSON CGIs;
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
  transport are touched; and
- preserves confirmed position from inspection instead of inventing optimistic
  orientation after movement.

The first production slice intentionally targets VAPIX camera 1. Event
streaming, snapshots, media transfer, multi-channel enumeration, and advanced
zoom/guard-tour control remain separate slices with additional host or
capability prerequisites.

Protocol references:

- [Axis VAPIX authentication](https://developer.axis.com/vapix/authentication/)
- [Axis basic device information](https://developer.axis.com/vapix/network-video/basic-device-information/)
- [Axis API Discovery](https://developer.axis.com/vapix/network-video/api-discovery-service/)
- [Axis mDNS-SD](https://developer.axis.com/vapix/network-video/mdns-sd-api/)
- [Axis PTZ API](https://developer.axis.com/vapix/network-video/pantiltzoom-api/)
