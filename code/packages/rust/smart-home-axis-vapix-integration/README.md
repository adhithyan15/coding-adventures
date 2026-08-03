# smart-home-axis-vapix-integration

Production-facing Axis VAPIX discovery and read-only inspection for D23.

The package:

- discovers documented `_axis-video._tcp.local` and `_axis-nvr._tcp.local`
  advertisements through the shared mDNS runtime;
- requires credential-free HTTPS origins for production, with plain HTTP
  permitted only for loopback transport tests;
- represents credentials as a `VaultRef` in request plans and materializes the
  Basic authorization value only inside the bounded transport;
- calls the authenticated Basic Device Information and API Discovery JSON CGIs;
- normalizes Axis identity, firmware, product type, and the supported VAPIX API
  list into one confirmed camera entity; and
- authorizes D23 reads before credentials or transport are touched.

PTZ control, event streaming, snapshots, and media transfer remain separate
slices because they require capability/permission probes, event lifecycle, or a
camera-media executor beyond identity inspection.

Protocol references:

- [Axis VAPIX authentication](https://developer.axis.com/vapix/authentication/)
- [Axis basic device information](https://developer.axis.com/vapix/network-video/basic-device-information/)
- [Axis API Discovery](https://developer.axis.com/vapix/network-video/api-discovery-service/)
- [Axis mDNS-SD](https://developer.axis.com/vapix/network-video/mdns-sd-api/)
