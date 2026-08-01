# smart-home-onvif-integration

This package provides the first production camera integration for D23:

- ONVIF WS-Discovery probes over bounded UDP.
- namespace-aware ProbeMatch parsing and D23 discovery records.
- WS-Security UsernameToken password-digest SOAP requests.
- bounded HTTP/1.1 and certificate-verifying HTTPS transport.
- device information, media profile, snapshot URI, and RTSP stream URI reads.
- normalized camera devices/entities with no media URI or credential material in
  runtime state.
- process-local snapshot and stream registration through the narrow
  `CameraMediaEndpointRegistry` surface; the host-owned camera-media service
  never returns a device endpoint URI to the lease holder.

The `smart-home-onvif-integration` binary can run `discover` or inspect one
device service. Inspection reads `ONVIF_USERNAME` and `ONVIF_PASSWORD` from the
environment and emits only a sanitized profile summary.
